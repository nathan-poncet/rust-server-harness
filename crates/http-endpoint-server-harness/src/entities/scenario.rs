use super::Endpoint;

/// A test scenario containing a server configuration, collector, and endpoints
pub struct Scenario<S, C> {
    pub(crate) server: S,
    pub(crate) collector: C,
    pub(crate) endpoints: Vec<Endpoint>,
}
