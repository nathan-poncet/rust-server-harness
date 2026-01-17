//! gRPC RPC Server Harness
//!
//! A test harness for mocking gRPC services with predefined responses.
//!
//! # Example
//!
//! ```rust,no_run
//! use grpc_rpc_server_harness::prelude::*;
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HarnessError> {
//!     // Define a known address for the server
//!     let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
//!
//!     // Spawn a task to make gRPC requests
//!     let requests_task = tokio::spawn(async move {
//!         // Wait for server to be ready
//!         tokio::time::sleep(std::time::Duration::from_millis(100)).await;
//!         // ... make gRPC calls ...
//!     });
//!
//!     // Build and execute a scenario using the builder pattern
//!     let collected = ScenarioBuilder::new()
//!         .server(Tonic::bind(addr))
//!         .collector(DefaultCollector::new())
//!         .service(
//!             Service::new("my.package.MyService")
//!                 .with_method(
//!                     Method::new("MyMethod")
//!                         .with_handler(Handler::from_bytes(vec![1, 2, 3]))
//!                 )
//!         )
//!         .build()
//!         .execute()
//!         .await?;
//!
//!     requests_task.await.unwrap();
//!
//!     // Server has automatically shut down, collected contains all requests
//!     for req in &collected {
//!         println!("Received: {} {}", req.service, req.method);
//!     }
//!
//!     Ok(())
//! }
//! ```

mod adapters;
pub mod entities;
pub mod error;
pub mod use_cases;

pub use error::HarnessError;

#[cfg(feature = "tonic")]
pub use adapters::gateways::Tonic;

/// Default collector implementation that collects requests into a Vec
pub struct DefaultCollector {
    requests: std::sync::Mutex<Vec<entities::CollectedRequest>>,
}

impl DefaultCollector {
    pub fn new() -> Self {
        Self {
            requests: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Default for DefaultCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl use_cases::ports::Collector for DefaultCollector {
    type Output = Vec<entities::CollectedRequest>;

    fn collect(&self, request: entities::CollectedRequest) {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request);
        }
    }

    fn into_output(self) -> Self::Output {
        self.requests.into_inner().unwrap_or_default()
    }
}

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::entities::{
        CollectedRequest, Handler, Message, Method, RequestContext, Service,
    };
    pub use crate::error::HarnessError;
    pub use crate::use_cases::ports::Collector;
    pub use crate::use_cases::ScenarioBuilder;
    pub use crate::DefaultCollector;

    #[cfg(feature = "tonic")]
    pub use crate::Tonic;
}
