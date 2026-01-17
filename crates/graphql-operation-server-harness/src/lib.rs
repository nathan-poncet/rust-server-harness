//! GraphQL Operation Server Harness
//!
//! A test harness for mocking GraphQL endpoints with predefined responses.
//!
//! # Example
//!
//! ```rust,no_run
//! use graphql_operation_server_harness::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HarnessError> {
//!     let result = ScenarioBuilder::new()
//!         .server(AsyncGraphQL::default())
//!         .collector(DefaultCollector::new())
//!         .operation(
//!             Operation::query()
//!                 .with_field(
//!                     Field::new("users")
//!                         .with_handler(Handler::new(serde_json::json!({
//!                             "users": [{"id": 1, "name": "John"}]
//!                         })))
//!                 )
//!         )
//!         .build()
//!         .execute()
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

mod adapters;
pub mod entities;
pub mod error;
pub mod use_cases;

pub use error::HarnessError;

#[cfg(feature = "async-graphql")]
pub use adapters::gateways::AsyncGraphQL;

/// Default collector implementation using a thread-safe vector
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
        CollectedRequest, Field, GraphQLError, Handler, HandlerResponse, Operation, OperationType,
        RequestContext, Scenario,
    };
    pub use crate::error::HarnessError;
    pub use crate::use_cases::ports::{Collector, Server};
    pub use crate::use_cases::ScenarioBuilder;
    pub use crate::DefaultCollector;
    pub use serde_json::json;

    #[cfg(feature = "async-graphql")]
    pub use crate::AsyncGraphQL;
}
