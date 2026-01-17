use async_trait::async_trait;
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};

use crate::entities::{CollectedRequest, Handler, Message, RequestContext, Service};
use crate::error::HarnessError;
use crate::use_cases::ports::{Collector, Server};

/// Tonic-compatible gRPC server implementation
#[derive(Clone)]
pub struct Tonic {
    addr: SocketAddr,
}

impl Tonic {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub fn bind(addr: impl Into<SocketAddr>) -> Self {
        Self::new(addr.into())
    }
}

impl Default for Tonic {
    fn default() -> Self {
        Self::new(([127, 0, 0, 1], 0).into())
    }
}

/// Shared state for tracking completion
#[derive(Clone)]
struct CompletionTracker {
    total_handlers: usize,
    handlers_called: Arc<AtomicUsize>,
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

    async fn handler_called(&self) {
        let called = self.handlers_called.fetch_add(1, Ordering::SeqCst) + 1;
        if called >= self.total_handlers {
            if let Some(tx) = self.shutdown_tx.lock().await.take() {
                let _ = tx.send(());
            }
        }
    }
}

/// Type-erased collector trait for internal use
trait ErasedCollector: Send + Sync {
    fn collect(&self, request: CollectedRequest);
}

impl<C: Collector> ErasedCollector for std::sync::Mutex<Option<C>> {
    fn collect(&self, request: CollectedRequest) {
        if let Ok(guard) = self.lock() {
            if let Some(ref collector) = *guard {
                collector.collect(request);
            }
        }
    }
}

/// State shared with handlers
struct ServerState {
    /// Map from "/package.Service/Method" to handlers
    routes: HashMap<String, RouteState>,
    collector: Arc<dyn ErasedCollector>,
    completion_tracker: CompletionTracker,
}

struct RouteState {
    handlers: Vec<Handler>,
    call_count: AtomicUsize,
    service_name: String,
    method_name: String,
}

impl ServerState {
    fn new(
        services: Vec<Service>,
        collector: Arc<dyn ErasedCollector>,
        completion_tracker: CompletionTracker,
    ) -> Self {
        let mut routes = HashMap::new();

        for service in services {
            for method in service.methods {
                let path = format!("/{}/{}", service.name, method.name);
                routes.insert(
                    path,
                    RouteState {
                        handlers: method.handlers,
                        call_count: AtomicUsize::new(0),
                        service_name: service.name.clone(),
                        method_name: method.name.clone(),
                    },
                );
            }
        }

        Self {
            routes,
            collector,
            completion_tracker,
        }
    }
}

async fn handle_grpc_request(
    state: Arc<ServerState>,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path().to_string();

    // Collect the request body
    let body_bytes = req.into_body().collect().await?.to_bytes();

    // gRPC messages are prefixed with 5 bytes: 1 byte compression flag + 4 bytes length
    let message_data = if body_bytes.len() > 5 {
        body_bytes[5..].to_vec()
    } else {
        Vec::new()
    };

    if let Some(route) = state.routes.get(&path) {
        // Collect the request
        let collected = CollectedRequest::new(
            route.service_name.clone(),
            route.method_name.clone(),
            Message::new(message_data.clone()),
        );
        state.collector.collect(collected);

        // Get the response from the handler
        let call_index = route.call_count.fetch_add(1, Ordering::SeqCst);
        let handler_count = route.handlers.len();
        let handler_index = call_index.min(handler_count.saturating_sub(1));

        // Check if this is a new handler being called for the first time
        if call_index < handler_count.max(1) {
            state.completion_tracker.handler_called().await;
        }

        let response_data = if let Some(handler) = route.handlers.get(handler_index) {
            let ctx = RequestContext::new(
                route.service_name.clone(),
                route.method_name.clone(),
                Message::new(message_data),
            );
            handler.respond(&ctx).data
        } else {
            Vec::new()
        };

        // Build gRPC response with length prefix
        let mut grpc_response = Vec::with_capacity(5 + response_data.len());
        grpc_response.push(0); // No compression
        let len = response_data.len() as u32;
        grpc_response.extend_from_slice(&len.to_be_bytes());
        grpc_response.extend_from_slice(&response_data);

        Ok(Response::builder()
            .status(200)
            .header("content-type", "application/grpc")
            .header("grpc-status", "0")
            .body(Full::new(Bytes::from(grpc_response)))
            .unwrap())
    } else {
        // Service/method not found
        Ok(Response::builder()
            .status(200)
            .header("content-type", "application/grpc")
            .header("grpc-status", "12") // UNIMPLEMENTED
            .header("grpc-message", "Method not found")
            .body(Full::new(Bytes::new()))
            .unwrap())
    }
}

#[async_trait]
impl Server for Tonic {
    async fn run<C, F>(
        &self,
        services: Vec<Service>,
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
        let total_handlers: usize = services
            .iter()
            .map(|s| {
                s.methods
                    .iter()
                    .map(|m| m.handlers.len().max(1))
                    .sum::<usize>()
            })
            .sum();

        // Create shutdown channel for auto-shutdown
        let (auto_shutdown_tx, mut auto_shutdown_rx) = oneshot::channel();
        let completion_tracker = CompletionTracker::new(total_handlers, auto_shutdown_tx);

        let state = Arc::new(ServerState::new(
            services,
            erased_collector,
            completion_tracker,
        ));

        let listener = TcpListener::bind(self.addr)
            .await
            .map_err(|e| HarnessError::ServerError(e.to_string()))?;

        let addr = listener
            .local_addr()
            .map_err(|e| HarnessError::ServerError(e.to_string()))?;

        // Call the on_ready callback if provided
        if let Some(callback) = on_ready {
            callback(addr);
        }

        // Run the server loop until auto-shutdown
        loop {
            tokio::select! {
                result = listener.accept() => {
                    if let Ok((stream, _)) = result {
                        let state = state.clone();
                        let io = TokioIo::new(stream);

                        tokio::spawn(async move {
                            let service = service_fn(move |req| {
                                let state = state.clone();
                                async move { handle_grpc_request(state, req).await }
                            });

                            let _ = http2::Builder::new(TokioExecutor::new())
                                .serve_connection(io, service)
                                .await;
                        });
                    }
                }
                _ = &mut auto_shutdown_rx => {
                    break;
                }
            }
        }

        // Extract the collector and return its output
        let collector = collector_holder
            .lock()
            .map_err(|e| HarnessError::ServerError(e.to_string()))?
            .take()
            .ok_or_else(|| HarnessError::ServerError("Collector already taken".to_string()))?;

        Ok(collector.into_output())
    }
}

