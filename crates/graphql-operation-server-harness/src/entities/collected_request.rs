use serde_json::Value;

/// A collected GraphQL request
#[derive(Debug, Clone)]
pub struct CollectedRequest {
    pub query: String,
    pub operation_name: Option<String>,
    pub variables: Option<Value>,
}

impl CollectedRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            operation_name: None,
            variables: None,
        }
    }

    pub fn with_operation_name(mut self, name: impl Into<String>) -> Self {
        self.operation_name = Some(name.into());
        self
    }

    pub fn with_variables(mut self, variables: Value) -> Self {
        self.variables = Some(variables);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collected_request_new() {
        let req = CollectedRequest::new("query { users { id } }");
        assert_eq!(req.query, "query { users { id } }");
        assert!(req.operation_name.is_none());
        assert!(req.variables.is_none());
    }

    #[test]
    fn test_collected_request_with_operation_name() {
        let req = CollectedRequest::new("query GetUsers { users { id } }")
            .with_operation_name("GetUsers");
        assert_eq!(req.operation_name, Some("GetUsers".to_string()));
    }

    #[test]
    fn test_collected_request_with_variables() {
        let req = CollectedRequest::new("query ($id: ID!) { user(id: $id) { name } }")
            .with_variables(serde_json::json!({"id": "123"}));
        assert_eq!(req.variables, Some(serde_json::json!({"id": "123"})));
    }
}

