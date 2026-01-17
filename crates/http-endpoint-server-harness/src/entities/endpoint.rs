use super::{Handler, Method};

/// Represents an HTTP endpoint with a path, method, and handlers
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub path: String,
    pub method: Method,
    pub handlers: Vec<Handler>,
}

impl Endpoint {
    pub fn new(path: impl Into<String>, method: Method) -> Self {
        Self {
            path: path.into(),
            method,
            handlers: Vec::new(),
        }
    }

    pub fn with_handler(mut self, handler: Handler) -> Self {
        self.handlers.push(handler);
        self
    }

    pub fn with_handlers(mut self, handlers: impl IntoIterator<Item = Handler>) -> Self {
        self.handlers.extend(handlers);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_new() {
        let endpoint = Endpoint::new("/api/test", Method::Get);
        assert_eq!(endpoint.path, "/api/test");
        assert_eq!(endpoint.method, Method::Get);
        assert!(endpoint.handlers.is_empty());
    }

    #[test]
    fn test_endpoint_with_handler() {
        let handler = Handler::from_json(&serde_json::json!({}));
        let endpoint = Endpoint::new("/api/test", Method::Post).with_handler(handler);
        assert_eq!(endpoint.handlers.len(), 1);
    }

    #[test]
    fn test_endpoint_with_multiple_handlers() {
        let endpoint = Endpoint::new("/api/test", Method::Get)
            .with_handler(Handler::from_json(&serde_json::json!({"first": true})))
            .with_handler(Handler::from_json(&serde_json::json!({"second": true})));
        assert_eq!(endpoint.handlers.len(), 2);
    }
}

