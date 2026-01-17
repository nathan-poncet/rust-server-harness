use std::net::SocketAddr;

use crate::entities::{Endpoint, Scenario};
use crate::error::HarnessError;
use crate::use_cases::ports::{Collector, Server};

/// Builder for creating scenarios with a fluent API
///
/// # Example
///
/// ```rust,no_run
/// use http_endpoint_server_harness::prelude::*;
/// use std::net::SocketAddr;
///
/// #[tokio::main]
/// async fn main() -> Result<(), HarnessError> {
///     // Define a known address for the server
///     let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
///
///     // Spawn a task to make HTTP requests
///     let requests_task = tokio::spawn(async move {
///         // Wait for server to be ready
///         tokio::time::sleep(std::time::Duration::from_millis(100)).await;
///         #[cfg(feature = "doctest")]
///         {
///             let client = reqwest::Client::new();
///             client.get(format!("http://{}/api/test", addr)).send().await.unwrap();
///         }
///     });
///
///     // Build and execute the scenario
///     let collected = ScenarioBuilder::new()
///         .server(Axum::bind(addr))
///         .collector(DefaultCollector::new())
///         .endpoint(Endpoint::new("/api/test", Method::Get)
///             .with_handler(Handler::from_json(&json!({"ok": true}))))
///         .build()
///         .execute()
///         .await?;
///
///     requests_task.await.unwrap();
///     Ok(())
/// }
/// ```
pub struct ScenarioBuilder<S, C> {
    server: Option<S>,
    collector: Option<C>,
    endpoints: Vec<Endpoint>,
}

impl ScenarioBuilder<(), ()> {
    /// Create a new scenario builder
    pub fn new() -> Self {
        Self {
            server: None,
            collector: None,
            endpoints: Vec::new(),
        }
    }
}

impl Default for ScenarioBuilder<(), ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, C> ScenarioBuilder<S, C> {
    /// Set the server implementation to use
    pub fn server<NewS: Server>(self, server: NewS) -> ScenarioBuilder<NewS, C> {
        ScenarioBuilder {
            server: Some(server),
            collector: self.collector,
            endpoints: self.endpoints,
        }
    }

    /// Set the collector to use
    pub fn collector<NewC: Collector>(self, collector: NewC) -> ScenarioBuilder<S, NewC> {
        ScenarioBuilder {
            server: self.server,
            collector: Some(collector),
            endpoints: self.endpoints,
        }
    }

    /// Add an endpoint to the scenario
    pub fn endpoint(mut self, endpoint: Endpoint) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    /// Add multiple endpoints to the scenario
    pub fn endpoints(mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> Self {
        self.endpoints.extend(endpoints);
        self
    }
}

impl<S: Server + 'static, C: Collector + 'static> ScenarioBuilder<S, C> {
    /// Build the scenario
    pub fn build(self) -> Scenario<S, C> {
        Scenario {
            server: self.server.expect("Server must be set before building"),
            collector: self.collector.expect("Collector must be set before building"),
            endpoints: self.endpoints,
        }
    }

    /// Execute the scenario directly from the builder
    pub async fn execute(self) -> Result<C::Output, HarnessError> {
        let scenario = self.build();
        scenario
            .server
            .run(scenario.endpoints, scenario.collector, None::<fn(SocketAddr)>)
            .await
    }
}

impl<S: Server + 'static, C: Collector + 'static> Scenario<S, C> {
    /// Execute the scenario.
    ///
    /// Starts the server and waits until all handlers have been called.
    /// The server automatically shuts down once all handlers have been called.
    /// Returns the collector's output type.
    pub async fn execute(self) -> Result<C::Output, HarnessError> {
        self.server
            .run(self.endpoints, self.collector, None::<fn(SocketAddr)>)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Method;

    #[test]
    fn test_scenario_builder() {
        let _builder = ScenarioBuilder::new();
    }

    #[test]
    fn test_scenario_builder_with_endpoint() {
        let _builder = ScenarioBuilder::new().endpoint(Endpoint::new("/api/test", Method::Get));
    }

    #[test]
    fn test_scenario_builder_with_multiple_endpoints() {
        let _builder = ScenarioBuilder::new()
            .endpoint(Endpoint::new("/api/users", Method::Get))
            .endpoint(Endpoint::new("/api/posts", Method::Get));
    }
}

