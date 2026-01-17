use async_trait::async_trait;
use crate::entities::Endpoint;
use crate::error::HarnessError;
use std::net::SocketAddr;
use super::Collector;

/// Trait for HTTP server implementations
#[async_trait]
pub trait Server: Send + Sync + Clone {
    /// Start the server with the given endpoints and collector.
    ///
    /// The server will automatically shut down once all handlers have been called.
    /// Returns the collector's output type.
    ///
    /// If `on_ready` is provided, it will be called with the actual server address
    /// once the server is ready to accept connections.
    async fn run<C, F>(
        &self,
        endpoints: Vec<Endpoint>,
        collector: C,
        on_ready: Option<F>,
    ) -> Result<C::Output, HarnessError>
    where
        C: Collector + 'static,
        F: FnOnce(SocketAddr) + Send + 'static;
}

