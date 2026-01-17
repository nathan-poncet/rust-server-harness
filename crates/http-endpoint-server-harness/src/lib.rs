//! HTTP Endpoint Server Harness
//!
//! A test harness for mocking HTTP endpoints with predefined responses.
//! The server automatically shuts down once all handlers have been called.
//!
//! # Example
//!
//! ```rust,no_run
//! use http_endpoint_server_harness::prelude::*;
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HarnessError> {
//!     // Define a fixed address for the server
//!     let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
//!
//!     // Spawn a task to make HTTP requests
//!     let requests_task = tokio::spawn(async move {
//!         // Wait for server to be ready (in real code, add proper retry logic)
//!         tokio::time::sleep(std::time::Duration::from_millis(100)).await;
//!
//!         let client = reqwest::Client::new();
//!         let _response = client
//!             .get(format!("http://{}/api/users", addr))
//!             .send()
//!             .await
//!             .unwrap();
//!     });
//!
//!     // Build and execute a scenario using the builder pattern
//!     let collected = ScenarioBuilder::new()
//!         .server(Axum::bind(addr))
//!         .collector(DefaultCollector::new())
//!         .endpoint(
//!             Endpoint::new("/api/users", Method::Get)
//!                 .with_handler(Handler::from_json(&json!({"id": 1})))
//!         )
//!         .build()
//!         .execute()
//!         .await?;
//!
//!     requests_task.await.unwrap();
//!
//!     // Server has automatically shut down, collected contains all requests
//!     for req in &collected {
//!         println!("Received: {} {}", req.method, req.path);
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

#[cfg(feature = "axum")]
pub use adapters::gateways::Axum;

/// Default collector implementation that collects requests into a Vec
pub struct DefaultCollector {
    requests: std::sync::Mutex<Vec<entities::Request>>,
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
    type Output = Vec<entities::Request>;

    fn collect(&self, request: entities::Request) {
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
    pub use crate::entities::{Endpoint, Handler, Method, Request, Response};
    pub use crate::error::HarnessError;
    pub use crate::use_cases::ports::Collector;
    pub use crate::use_cases::ScenarioBuilder;
    pub use crate::DefaultCollector;

    #[cfg(feature = "axum")]
    pub use crate::Axum;

    pub use serde_json::json;
}
