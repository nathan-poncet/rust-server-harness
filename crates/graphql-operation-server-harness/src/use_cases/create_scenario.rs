use std::net::SocketAddr;

use crate::entities::{Operation, Scenario};
use crate::error::HarnessError;
use crate::use_cases::ports::{Collector, Server};

/// Builder for creating scenarios with a fluent API
pub struct ScenarioBuilder<S, C> {
    server: Option<S>,
    collector: Option<C>,
    operations: Vec<Operation>,
}

impl ScenarioBuilder<(), ()> {
    pub fn new() -> Self {
        Self {
            server: None,
            collector: None,
            operations: Vec::new(),
        }
    }
}

impl Default for ScenarioBuilder<(), ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, C> ScenarioBuilder<S, C> {
    pub fn server<NewS: Server>(self, server: NewS) -> ScenarioBuilder<NewS, C> {
        ScenarioBuilder {
            server: Some(server),
            collector: self.collector,
            operations: self.operations,
        }
    }

    pub fn collector<NewC: Collector>(self, collector: NewC) -> ScenarioBuilder<S, NewC> {
        ScenarioBuilder {
            server: self.server,
            collector: Some(collector),
            operations: self.operations,
        }
    }

    pub fn operation(mut self, operation: Operation) -> Self {
        self.operations.push(operation);
        self
    }
}

impl<S: Server + 'static, C: Collector + 'static> ScenarioBuilder<S, C> {
    pub fn build(self) -> Scenario<S, C> {
        Scenario {
            server: self.server.expect("server is required"),
            collector: self.collector.expect("collector is required"),
            operations: self.operations,
        }
    }

    /// Execute the scenario directly from the builder
    pub async fn execute(self) -> Result<C::Output, HarnessError> {
        let scenario = self.build();
        scenario
            .server
            .run(scenario.operations, scenario.collector, None::<fn(SocketAddr)>)
            .await
    }
}

impl<S: Server + 'static, C: Collector + 'static> Scenario<S, C> {
    /// Execute the scenario
    pub async fn execute(self) -> Result<C::Output, HarnessError> {
        self.server
            .run(self.operations, self.collector, None::<fn(SocketAddr)>)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_builder_new() {
        let builder = ScenarioBuilder::new();
        assert!(builder.operations.is_empty());
    }

    #[test]
    fn test_scenario_builder_with_operation() {
        let builder = ScenarioBuilder::new().operation(Operation::query());
        assert_eq!(builder.operations.len(), 1);
    }

    #[test]
    fn test_scenario_builder_with_multiple_operations() {
        let builder = ScenarioBuilder::new()
            .operation(Operation::query())
            .operation(Operation::mutation());
        assert_eq!(builder.operations.len(), 2);
    }
}

