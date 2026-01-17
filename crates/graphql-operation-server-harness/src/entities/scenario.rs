use crate::entities::Operation;

/// A test scenario containing a server configuration, collector, and operations
pub struct Scenario<S, C> {
    pub(crate) server: S,
    pub(crate) collector: C,
    pub(crate) operations: Vec<Operation>,
}
