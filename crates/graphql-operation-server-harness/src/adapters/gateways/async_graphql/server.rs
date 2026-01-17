use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex as TokioMutex};

use crate::entities::{CollectedRequest, Handler, Operation, OperationType, RequestContext};
use crate::error::HarnessError;
use crate::use_cases::ports::{Collector, Server};

/// AsyncGraphQL-compatible server implementation
#[derive(Clone)]
pub struct AsyncGraphQL {
    addr: SocketAddr,
}

impl AsyncGraphQL {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub fn bind(addr: impl Into<SocketAddr>) -> Self {
        Self::new(addr.into())
    }
}

impl Default for AsyncGraphQL {
    fn default() -> Self {
        Self::new(([127, 0, 0, 1], 0).into())
    }
}

#[derive(Debug, Deserialize)]
struct GraphQLRequest {
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    variables: Option<Value>,
}

#[derive(Debug, Serialize)]
struct GraphQLResponse {
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<Value>>,
}

/// Shared state for tracking completion
#[derive(Clone)]
struct CompletionTracker {
    total_handlers: usize,
    handlers_called: Arc<AtomicUsize>,
    shutdown_tx: Arc<TokioMutex<Option<oneshot::Sender<()>>>>,
}

impl CompletionTracker {
    fn new(total_handlers: usize, shutdown_tx: oneshot::Sender<()>) -> Self {
        Self {
            total_handlers,
            handlers_called: Arc::new(AtomicUsize::new(0)),
            shutdown_tx: Arc::new(TokioMutex::new(Some(shutdown_tx))),
        }
    }

    async fn handler_called(&self) {
        let called = self.handlers_called.fetch_add(1, Ordering::SeqCst) + 1;
        if called >= self.total_handlers {
            if let Some(tx) = self.shutdown_tx.lock().await.take() {
                let _ = tx.send(());
            }
        }
    }
}

/// Internal trait for collecting requests (without Output type)
trait InternalCollector: Send + Sync {
    fn collect(&self, request: CollectedRequest);
}

impl<C: Collector> InternalCollector for C {
    fn collect(&self, request: CollectedRequest) {
        Collector::collect(self, request);
    }
}

/// State shared with handlers
#[derive(Clone)]
struct ServerState {
    /// Map from field name to handlers
    query_handlers: Arc<HashMap<String, FieldState>>,
    mutation_handlers: Arc<HashMap<String, FieldState>>,
    collector: Arc<dyn InternalCollector>,
    completion_tracker: CompletionTracker,
}

#[derive(Clone)]
struct FieldState {
    handlers: Vec<Handler>,
    call_count: Arc<AtomicUsize>,
}

async fn handle_graphql(
    State(state): State<ServerState>,
    body: String,
) -> impl IntoResponse {
    let request: GraphQLRequest = match serde_json::from_str(&body) {
        Ok(req) => req,
        Err(e) => {
            let response = GraphQLResponse {
                data: None,
                errors: Some(vec![serde_json::json!({"message": e.to_string()})]),
            };
            return (StatusCode::OK, axum::Json(response));
        }
    };

    // Collect the request
    let mut collected = CollectedRequest::new(&request.query);
    if let Some(op_name) = &request.operation_name {
        collected = collected.with_operation_name(op_name);
    }
    if let Some(vars) = &request.variables {
        collected = collected.with_variables(vars.clone());
    }
    state.collector.collect(collected);

    // Parse the query to find the operation type and field
    let query = request.query.trim();
    let (handlers_map, _op_type) = if query.starts_with("mutation") {
        (&state.mutation_handlers, "mutation")
    } else {
        (&state.query_handlers, "query")
    };

    // Simple field extraction - find field names in the query
    let mut response_data = serde_json::Map::new();
    let mut errors: Vec<Value> = Vec::new();

    for (field_name, field_state) in handlers_map.iter() {
        if query.contains(field_name) {
            let call_index = field_state.call_count.fetch_add(1, Ordering::SeqCst);
            let handler_count = field_state.handlers.len();
            let handler_index = call_index.min(handler_count.saturating_sub(1));

            // Check if this is a new handler being called for the first time
            if call_index < handler_count {
                state.completion_tracker.handler_called().await;
            }

            if let Some(handler) = field_state.handlers.get(handler_index) {
                let mut ctx = RequestContext::new(field_name).with_query(&request.query);
                if let Some(op_name) = &request.operation_name {
                    ctx = ctx.with_operation_name(op_name);
                }
                if let Some(vars) = &request.variables {
                    ctx = ctx.with_variables(vars.clone());
                }

                let handler_response = handler.respond(&ctx);
                if let Some(obj) = handler_response.data.as_object() {
                    for (k, v) in obj {
                        response_data.insert(k.clone(), v.clone());
                    }
                } else {
                    response_data.insert(field_name.clone(), handler_response.data.clone());
                }
                if let Some(errs) = &handler_response.errors {
                    for err in errs {
                        let mut err_val = serde_json::json!({"message": err.message});
                        if let Some(path) = &err.path {
                            err_val["path"] = serde_json::json!(path);
                        }
                        errors.push(err_val);
                    }
                }
            }
        }
    }

    let response = GraphQLResponse {
        data: Some(Value::Object(response_data)),
        errors: if errors.is_empty() { None } else { Some(errors) },
    };

    (StatusCode::OK, axum::Json(response))
}

#[async_trait]
impl Server for AsyncGraphQL {
    async fn run<C, F>(
        &self,
        operations: Vec<Operation>,
        collector: C,
        on_ready: Option<F>,
    ) -> Result<C::Output, HarnessError>
    where
        C: Collector + 'static,
        F: FnOnce(SocketAddr) + Send + 'static,
    {
        let collector_arc: Arc<C> = Arc::new(collector);

        // Count total handlers
        let total_handlers: usize = operations
            .iter()
            .map(|op| {
                op.fields
                    .iter()
                    .map(|f| f.handlers.len().max(1))
                    .sum::<usize>()
            })
            .sum();

        // Create shutdown channel for auto-shutdown
        let (auto_shutdown_tx, auto_shutdown_rx) = oneshot::channel();
        let completion_tracker = CompletionTracker::new(total_handlers, auto_shutdown_tx);

        let mut query_handlers = HashMap::new();
        let mut mutation_handlers = HashMap::new();

        for operation in operations {
            let handlers_map = match operation.operation_type {
                OperationType::Query => &mut query_handlers,
                OperationType::Mutation => &mut mutation_handlers,
                OperationType::Subscription => continue, // Skip subscriptions for now
            };

            for field in operation.fields {
                handlers_map.insert(
                    field.name,
                    FieldState {
                        handlers: field.handlers,
                        call_count: Arc::new(AtomicUsize::new(0)),
                    },
                );
            }
        }

        let state = ServerState {
            query_handlers: Arc::new(query_handlers),
            mutation_handlers: Arc::new(mutation_handlers),
            collector: collector_arc.clone(),
            completion_tracker,
        };

        let router = Router::new()
            .route("/graphql", post(handle_graphql))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| HarnessError::ServerError(e.to_string()))?;

        let addr = listener
            .local_addr()
            .map_err(|e| HarnessError::ServerError(e.to_string()))?;

        // Call on_ready callback if provided
        if let Some(callback) = on_ready {
            callback(addr);
        }

        // Run the server until auto-shutdown
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = auto_shutdown_rx.await;
            })
            .await
            .map_err(|e| HarnessError::ServerError(e.to_string()))?;

        // Extract the collector from the Arc and return its output
        let collector = Arc::try_unwrap(collector_arc)
            .map_err(|_| HarnessError::ServerError("Failed to unwrap collector".to_string()))?;
        Ok(collector.into_output())
    }
}
