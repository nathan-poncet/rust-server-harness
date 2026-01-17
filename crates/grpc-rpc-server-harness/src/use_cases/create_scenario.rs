use std::net::SocketAddr;

use crate::entities::{Scenario, Service};
use crate::error::HarnessError;
use crate::use_cases::ports::{Collector, Server};

/// Builder for creating scenarios with a fluent API
///
/// # Example
///
/// ```rust,no_run
/// use grpc_rpc_server_harness::prelude::*;
/// use std::net::SocketAddr;
///
/// #[tokio::main]
/// async fn main() -> Result<(), HarnessError> {
///     // Define a known address for the server
///     let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
///
///     // Spawn a task to make gRPC requests
///     let requests_task = tokio::spawn(async move {
///         // Wait for server to be ready
///         tokio::time::sleep(std::time::Duration::from_millis(100)).await;
///         // ... make gRPC calls ...
///     });
///
///     // Build and execute the scenario
///     let collected = ScenarioBuilder::new()
///         .server(Tonic::bind(addr))
///         .collector(DefaultCollector::new())
///         .service(Service::new("my.package.MyService")
///             .with_method(Method::new("MyMethod")
///                 .with_handler(Handler::from_bytes(vec![1, 2, 3]))))
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
    services: Vec<Service>,
}

impl ScenarioBuilder<(), ()> {
    /// Create a new scenario builder
    pub fn new() -> Self {
        Self {
            server: None,
            collector: None,
            services: Vec::new(),
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
            services: self.services,
        }
    }

    /// Set the collector to use
    pub fn collector<NewC: Collector>(self, collector: NewC) -> ScenarioBuilder<S, NewC> {
        ScenarioBuilder {
            server: self.server,
            collector: Some(collector),
            services: self.services,
        }
    }

    /// Add a service to the scenario
    pub fn service(mut self, service: Service) -> Self {
        self.services.push(service);
        self
    }

    /// Add multiple services to the scenario
    pub fn services(mut self, services: impl IntoIterator<Item = Service>) -> Self {
        self.services.extend(services);
        self
    }
}

impl<S: Server + 'static, C: Collector + 'static> ScenarioBuilder<S, C> {
    /// Build the scenario
    pub fn build(self) -> Scenario<S, C> {
        Scenario {
            server: self.server.expect("Server must be set before building"),
            collector: self.collector.expect("Collector must be set before building"),
            services: self.services,
        }
    }

    /// Execute the scenario directly from the builder
    pub async fn execute(self) -> Result<C::Output, HarnessError> {
        let scenario = self.build();
        scenario.execute().await
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
            .run(self.services, self.collector, None::<fn(SocketAddr)>)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_builder() {
        let _builder = ScenarioBuilder::new();
    }

    #[test]
    fn test_scenario_builder_with_service() {
        let _builder = ScenarioBuilder::new().service(Service::new("my.package.MyService"));
    }

    #[test]
    fn test_scenario_builder_with_multiple_services() {
        let _builder = ScenarioBuilder::new()
            .service(Service::new("my.package.UserService"))
            .service(Service::new("my.package.PostService"));
    }
}

