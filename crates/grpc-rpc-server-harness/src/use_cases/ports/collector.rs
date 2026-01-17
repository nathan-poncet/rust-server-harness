use crate::entities::CollectedRequest;

/// Trait for collecting gRPC requests during scenario execution
///
/// The `Output` type is the final result returned when the scenario completes.
/// This allows users to define their own collection strategy and return type.
pub trait Collector: Send + Sync {
    /// The type returned when the scenario completes
    type Output: Send;

    /// Called when a request is received
    fn collect(&self, request: CollectedRequest);

    /// Consume the collector and return the final output
    fn into_output(self) -> Self::Output;
}

