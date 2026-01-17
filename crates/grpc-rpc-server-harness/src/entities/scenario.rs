use crate::entities::Service;

/// A test scenario containing a server configuration, collector, and services
pub struct Scenario<S, C> {
    pub(crate) server: S,
    pub(crate) collector: C,
    pub(crate) services: Vec<Service>,
}
