use async_trait::async_trait;
use std::net::SocketAddr;

use super::Collector;
use crate::entities::Service;
use crate::error::HarnessError;

/// Trait for gRPC server implementations
#[async_trait]
pub trait Server: Send + Sync + Clone {
    /// Start the server with the given services and collector.
    ///
    /// The server will automatically shut down once all handlers have been called.
    /// Returns the collector's output type.
    ///
    /// If `on_ready` is provided, it will be called with the actual server address
    /// once the server is ready to accept connections.
    async fn run<C, F>(
        &self,
        services: Vec<Service>,
        collector: C,
        on_ready: Option<F>,
    ) -> Result<C::Output, HarnessError>
    where
        C: Collector + 'static,
        F: FnOnce(SocketAddr) + Send + 'static;
}

