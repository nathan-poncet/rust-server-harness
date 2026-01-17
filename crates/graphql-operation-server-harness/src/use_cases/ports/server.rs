use async_trait::async_trait;
use std::net::SocketAddr;

use super::Collector;
use crate::entities::Operation;
use crate::error::HarnessError;

/// Trait for GraphQL server implementations
#[async_trait]
pub trait Server: Send + Sync + Clone {
    /// Run the server with the given operations until all handlers are called
    async fn run<C, F>(
        &self,
        operations: Vec<Operation>,
        collector: C,
        on_ready: Option<F>,
    ) -> Result<C::Output, HarnessError>
    where
        C: Collector + 'static,
        F: FnOnce(SocketAddr) + Send + 'static;
}
