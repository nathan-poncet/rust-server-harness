use super::Handler;

/// Represents a gRPC method within a service
#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub handlers: Vec<Handler>,
}

impl Method {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
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
    fn test_method_new() {
        let method = Method::new("GetUser");
        assert_eq!(method.name, "GetUser");
        assert!(method.handlers.is_empty());
    }

    #[test]
    fn test_method_with_handler() {
        let method = Method::new("GetUser")
            .with_handler(Handler::from_bytes(vec![1, 2, 3]));
        assert_eq!(method.handlers.len(), 1);
    }

    #[test]
    fn test_method_with_multiple_handlers() {
        let method = Method::new("GetUser")
            .with_handler(Handler::from_bytes(vec![1]))
            .with_handler(Handler::from_bytes(vec![2]))
            .with_handler(Handler::from_bytes(vec![3]));
        assert_eq!(method.handlers.len(), 3);
    }
}
