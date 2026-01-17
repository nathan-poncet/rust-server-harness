use super::Handler;

/// Represents a GraphQL field (query or mutation field)
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub handlers: Vec<Handler>,
}

impl Field {
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
    fn test_field_new() {
        let field = Field::new("users");
        assert_eq!(field.name, "users");
        assert!(field.handlers.is_empty());
    }

    #[test]
    fn test_field_with_handler() {
        let field = Field::new("users")
            .with_handler(Handler::new(serde_json::json!([])));
        assert_eq!(field.handlers.len(), 1);
    }

    #[test]
    fn test_field_with_multiple_handlers() {
        let field = Field::new("user")
            .with_handler(Handler::new(serde_json::json!({"id": 1})))
            .with_handler(Handler::new(serde_json::json!({"id": 2})));
        assert_eq!(field.handlers.len(), 2);
    }
}
