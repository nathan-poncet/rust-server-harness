use crate::entities::CollectedRequest;

/// Trait for collecting GraphQL requests during scenario execution
pub trait Collector: Send + Sync {
    /// The output type returned when the collector is consumed
    type Output: Send;

    /// Called when a request is received
    fn collect(&self, request: CollectedRequest);

    /// Consume the collector and return the collected output
    fn into_output(self) -> Self::Output;
}
