use async_trait::async_trait;
use axum::{
    body::Body,
    extract::State,
    http::{Request as AxumRequest, StatusCode},
    response::IntoResponse,
    routing::MethodRouter,
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{oneshot, Mutex};

use crate::entities::{Endpoint, Handler, Method, Request};
use crate::error::HarnessError;
use crate::use_cases::ports::{Collector, Server};

/// Axum-based HTTP server implementation
#[derive(Clone)]
pub struct Axum {
    addr: SocketAddr,
}

impl Axum {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub fn bind(addr: impl Into<SocketAddr>) -> Self {
        Self::new(addr.into())
    }
}

impl Default for Axum {
    fn default() -> Self {
        Self::new(([127, 0, 0, 1], 0).into())
    }
}

/// Shared state for tracking completion
#[derive(Clone)]
struct CompletionTracker {
    /// Total number of handlers across all endpoints
    total_handlers: usize,
    /// Number of handlers that have been called at least once
    handlers_called: Arc<AtomicUsize>,
    /// Shutdown signal sender (wrapped in Mutex for Clone)
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl CompletionTracker {
    fn new(total_handlers: usize, shutdown_tx: oneshot::Sender<()>) -> Self {
        Self {
            total_handlers,
            handlers_called: Arc::new(AtomicUsize::new(0)),
            shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
        }
    }

    /// Called when a handler is used for the first time
    async fn handler_called(&self) {
        let called = self.handlers_called.fetch_add(1, Ordering::SeqCst) + 1;
        if called >= self.total_handlers {
            // All handlers have been called, trigger shutdown
            if let Some(tx) = self.shutdown_tx.lock().await.take() {
                let _ = tx.send(());
            }
        }
    }
}

/// Type-erased collector trait for internal use
trait ErasedCollector: Send + Sync {
    fn collect(&self, request: Request);
}

impl<C: Collector> ErasedCollector for std::sync::Mutex<Option<C>> {
    fn collect(&self, request: Request) {
        if let Ok(guard) = self.lock() {
            if let Some(ref collector) = *guard {
                collector.collect(request);
            }
        }
    }
}

/// State shared with Axum handlers using type erasure
#[derive(Clone)]
struct EndpointState {
    handlers: Arc<Vec<Handler>>,
    call_count: Arc<AtomicUsize>,
    collector: Arc<dyn ErasedCollector>,
    completion_tracker: CompletionTracker,
}

async fn handle_request(
    State(state): State<EndpointState>,
    request: AxumRequest<Body>,
) -> impl IntoResponse {
    // Parse method
    let method = match request.method().as_str() {
        "GET" => Method::Get,
        "POST" => Method::Post,
        "PUT" => Method::Put,
        "PATCH" => Method::Patch,
        "DELETE" => Method::Delete,
        "HEAD" => Method::Head,
        "OPTIONS" => Method::Options,
        _ => Method::Get,
    };

    let path = request.uri().path().to_string();
    let headers: HashMap<String, String> = request
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body = axum::body::to_bytes(request.into_body(), usize::MAX)
        .await
        .map(|b| b.to_vec())
        .unwrap_or_default();

    // Collect the request
    let collected_request = Request {
        method,
        path,
        headers,
        body,
    };
    state.collector.collect(collected_request.clone());

    // Get the response from the handler (sequential through handlers)
    let call_index = state.call_count.fetch_add(1, Ordering::SeqCst);
    let handler_count = state.handlers.len();
    let handler_index = call_index.min(handler_count.saturating_sub(1));

    // Check if this is a new handler being called for the first time
    if call_index < handler_count {
        state.completion_tracker.handler_called().await;
    }

    if let Some(handler) = state.handlers.get(handler_index) {
        let response = handler.respond(&collected_request);
        let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::OK);
        let mut builder = axum::http::Response::builder().status(status);

        for (key, value) in &response.headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        builder
            .body(Body::from(response.body.clone()))
            .unwrap_or_else(|_| {
                axum::http::Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap()
            })
    } else {
        axum::http::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("No handler configured"))
            .unwrap()
    }
}

fn create_method_router(method: Method) -> MethodRouter<EndpointState> {
    match method {
        Method::Get => axum::routing::get(handle_request),
        Method::Post => axum::routing::post(handle_request),
        Method::Put => axum::routing::put(handle_request),
        Method::Patch => axum::routing::patch(handle_request),
        Method::Delete => axum::routing::delete(handle_request),
        Method::Head => axum::routing::head(handle_request),
        Method::Options => axum::routing::options(handle_request),
    }
}

#[async_trait]
impl Server for Axum {
    async fn run<C, F>(
        &self,
        endpoints: Vec<Endpoint>,
        collector: C,
        on_ready: Option<F>,
    ) -> Result<C::Output, HarnessError>
    where
        C: Collector + 'static,
        F: FnOnce(SocketAddr) + Send + 'static,
    {
        // Wrap collector in Mutex<Option<C>> so we can take it out at the end
        let collector_holder: Arc<std::sync::Mutex<Option<C>>> =
            Arc::new(std::sync::Mutex::new(Some(collector)));
        let erased_collector: Arc<dyn ErasedCollector> = collector_holder.clone();

        // Count total handlers
        let total_handlers: usize = endpoints.iter().map(|e| e.handlers.len().max(1)).sum();

        // Create shutdown channel for auto-shutdown
        let (auto_shutdown_tx, auto_shutdown_rx) = oneshot::channel();
        let completion_tracker = CompletionTracker::new(total_handlers, auto_shutdown_tx);

        let mut router: Router<EndpointState> = Router::new();

        for endpoint in endpoints {
            let state = EndpointState {
                handlers: Arc::new(endpoint.handlers),
                call_count: Arc::new(AtomicUsize::new(0)),
                collector: erased_collector.clone(),
                completion_tracker: completion_tracker.clone(),
            };

            let method_router = create_method_router(endpoint.method);
            router = router.route(&endpoint.path, method_router).with_state(state);
        }

        // Convert to Router<()> for serving
        let router = router.with_state(EndpointState {
            handlers: Arc::new(vec![]),
            call_count: Arc::new(AtomicUsize::new(0)),
            collector: erased_collector.clone(),
            completion_tracker: completion_tracker.clone(),
        });

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| HarnessError::ServerError(e.to_string()))?;

        let addr = listener
            .local_addr()
            .map_err(|e| HarnessError::ServerError(e.to_string()))?;

        // Call the on_ready callback if provided
        if let Some(callback) = on_ready {
            callback(addr);
        }

        // Serve and wait for auto-shutdown
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                auto_shutdown_rx.await.ok();
            })
            .await
            .map_err(|e| HarnessError::ServerError(e.to_string()))?;

        // Extract the collector and return its output
        let collector = collector_holder
            .lock()
            .map_err(|e| HarnessError::ServerError(e.to_string()))?
            .take()
            .ok_or_else(|| HarnessError::ServerError("Collector already taken".to_string()))?;

        Ok(collector.into_output())
    }
}

