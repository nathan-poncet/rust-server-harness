use serde_json::Value;
use std::sync::Arc;

/// Context passed to dynamic handlers
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub query: String,
    pub operation_name: Option<String>,
    pub variables: Option<Value>,
    pub field_name: String,
}

impl RequestContext {
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            query: String::new(),
            operation_name: None,
            variables: None,
            field_name: field_name.into(),
        }
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    pub fn with_operation_name(mut self, name: impl Into<String>) -> Self {
        self.operation_name = Some(name.into());
        self
    }

    pub fn with_variables(mut self, variables: Value) -> Self {
        self.variables = Some(variables);
        self
    }

    /// Get a variable by name
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.as_ref().and_then(|v| v.get(name))
    }
}

/// A GraphQL error
#[derive(Debug, Clone)]
pub struct GraphQLError {
    pub message: String,
    pub path: Option<Vec<String>>,
}

/// Type alias for dynamic handler functions
pub type HandlerFn = Arc<dyn Fn(&RequestContext) -> HandlerResponse + Send + Sync>;

/// Response from a handler
#[derive(Debug, Clone)]
pub struct HandlerResponse {
    pub data: Value,
    pub errors: Option<Vec<GraphQLError>>,
}

impl HandlerResponse {
    pub fn new(data: Value) -> Self {
        Self { data, errors: None }
    }

    pub fn with_error(mut self, message: impl Into<String>) -> Self {
        let error = GraphQLError {
            message: message.into(),
            path: None,
        };
        self.errors.get_or_insert_with(Vec::new).push(error);
        self
    }

    pub fn to_response_value(&self) -> Value {
        let mut response = serde_json::json!({
            "data": self.data
        });
        if let Some(errors) = &self.errors {
            let error_values: Vec<Value> = errors
                .iter()
                .map(|e| {
                    let mut err = serde_json::json!({"message": e.message});
                    if let Some(path) = &e.path {
                        err["path"] = serde_json::json!(path);
                    }
                    err
                })
                .collect();
            response["errors"] = serde_json::json!(error_values);
        }
        response
    }
}

/// A handler that returns either a static or dynamic GraphQL response
#[derive(Clone)]
pub enum Handler {
    /// Static response - always returns the same data
    Static(HandlerResponse),
    /// Dynamic response - builds data based on the request context
    Dynamic(HandlerFn),
}

impl std::fmt::Debug for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Handler::Static(response) => f.debug_tuple("Static").field(response).finish(),
            Handler::Dynamic(_) => f.debug_tuple("Dynamic").field(&"<fn>").finish(),
        }
    }
}

impl Handler {
    /// Create a new static handler with predefined data
    pub fn new(data: Value) -> Self {
        Handler::Static(HandlerResponse::new(data))
    }

    /// Create a dynamic handler that builds responses based on the request context
    pub fn dynamic<F>(f: F) -> Self
    where
        F: Fn(&RequestContext) -> HandlerResponse + Send + Sync + 'static,
    {
        Handler::Dynamic(Arc::new(f))
    }

    /// Add an error to a static handler
    pub fn with_error(self, message: impl Into<String>) -> Self {
        match self {
            Handler::Static(response) => Handler::Static(response.with_error(message)),
            Handler::Dynamic(_) => self,
        }
    }

    /// Add an error with path to a static handler
    pub fn with_error_at_path(self, message: impl Into<String>, path: Vec<String>) -> Self {
        match self {
            Handler::Static(mut response) => {
                let error = GraphQLError {
                    message: message.into(),
                    path: Some(path),
                };
                response.errors.get_or_insert_with(Vec::new).push(error);
                Handler::Static(response)
            }
            Handler::Dynamic(_) => self,
        }
    }

    /// Get the response for a given request context
    pub fn respond(&self, ctx: &RequestContext) -> HandlerResponse {
        match self {
            Handler::Static(response) => response.clone(),
            Handler::Dynamic(f) => f(ctx),
        }
    }

    /// Get the static data (for backwards compatibility)
    pub fn data(&self) -> &Value {
        match self {
            Handler::Static(response) => &response.data,
            Handler::Dynamic(_) => &Value::Null,
        }
    }

    /// Get the errors (for backwards compatibility)
    pub fn errors(&self) -> Option<&Vec<GraphQLError>> {
        match self {
            Handler::Static(response) => response.errors.as_ref(),
            Handler::Dynamic(_) => None,
        }
    }

    /// Convert to response value (for backwards compatibility)
    pub fn to_response(&self) -> Value {
        match self {
            Handler::Static(response) => response.to_response_value(),
            Handler::Dynamic(_) => serde_json::json!({"data": null}),
        }
    }
}

impl From<Value> for Handler {
    fn from(data: Value) -> Self {
        Handler::new(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_new() {
        let handler = Handler::new(serde_json::json!({"users": []}));
        let ctx = RequestContext::new("users");
        assert_eq!(handler.respond(&ctx).data, serde_json::json!({"users": []}));
    }

    #[test]
    fn test_handler_with_error() {
        let handler = Handler::new(serde_json::json!(null)).with_error("Something went wrong");
        let ctx = RequestContext::new("test");
        let response = handler.respond(&ctx);
        let errors = response.errors.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Something went wrong");
    }

    #[test]
    fn test_handler_with_error_at_path() {
        let handler = Handler::new(serde_json::json!(null))
            .with_error_at_path("Field not found", vec!["user".to_string(), "name".to_string()]);
        let ctx = RequestContext::new("test");
        let response = handler.respond(&ctx);
        let errors = response.errors.unwrap();
        assert_eq!(errors[0].path, Some(vec!["user".to_string(), "name".to_string()]));
    }

    #[test]
    fn test_handler_to_response() {
        let handler = Handler::new(serde_json::json!({"id": 1}));
        let response = handler.to_response();
        assert_eq!(response["data"]["id"], 1);
        assert!(response.get("errors").is_none());
    }

    #[test]
    fn test_handler_to_response_with_errors() {
        let handler = Handler::new(serde_json::json!(null))
            .with_error("Error 1")
            .with_error("Error 2");
        let response = handler.to_response();
        let errors = response["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_handler_from_value() {
        let value = serde_json::json!({"test": true});
        let handler: Handler = value.into();
        let ctx = RequestContext::new("test");
        assert_eq!(handler.respond(&ctx).data, serde_json::json!({"test": true}));
    }

    #[test]
    fn test_dynamic_handler() {
        let handler = Handler::dynamic(|ctx: &RequestContext| {
            HandlerResponse::new(serde_json::json!({
                "field": ctx.field_name,
                "query": ctx.query
            }))
        });

        let ctx = RequestContext::new("users").with_query("query { users { id } }");
        let response = handler.respond(&ctx);
        assert_eq!(response.data["field"], "users");
        assert!(response.data["query"].as_str().unwrap().contains("users"));
    }

    #[test]
    fn test_dynamic_handler_with_variables() {
        let handler = Handler::dynamic(|ctx: &RequestContext| {
            let id = ctx.get_variable("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            HandlerResponse::new(serde_json::json!({
                "user": {"id": id, "name": format!("User {}", id)}
            }))
        });

        let ctx = RequestContext::new("user")
            .with_variables(serde_json::json!({"id": "123"}));
        let response = handler.respond(&ctx);
        assert_eq!(response.data["user"]["id"], "123");
        assert_eq!(response.data["user"]["name"], "User 123");
    }
}
