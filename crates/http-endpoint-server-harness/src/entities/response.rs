use std::collections::HashMap;

/// Represents an HTTP response to be sent by the harness
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn ok() -> Self {
        Self::new(200)
    }

    pub fn created() -> Self {
        Self::new(201)
    }

    pub fn not_found() -> Self {
        Self::new(404)
    }

    pub fn internal_error() -> Self {
        Self::new(500)
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn with_json<T: serde::Serialize>(mut self, value: &T) -> Self {
        self.headers.insert("content-type".to_string(), "application/json".to_string());
        self.body = serde_json::to_vec(value).unwrap_or_default();
        self
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_new() {
        let response = Response::new(200);
        assert_eq!(response.status, 200);
        assert!(response.headers.is_empty());
        assert!(response.body.is_empty());
    }

    #[test]
    fn test_response_with_body() {
        let response = Response::new(200).with_body("Hello");
        assert_eq!(response.body, b"Hello");
    }

    #[test]
    fn test_response_with_json_body() {
        let response = Response::new(200).with_json(&serde_json::json!({"key": "value"}));
        assert!(response.body.contains(&b"key"[0]));
        assert!(response.headers.get("content-type").unwrap().contains("application/json"));
    }

    #[test]
    fn test_response_with_header() {
        let response = Response::new(200).with_header("X-Custom", "value");
        assert_eq!(response.headers.get("X-Custom").unwrap(), "value");
    }

    #[test]
    fn test_response_ok() {
        let response = Response::ok();
        assert_eq!(response.status, 200);
    }

    #[test]
    fn test_response_created() {
        let response = Response::created();
        assert_eq!(response.status, 201);
    }

    #[test]
    fn test_response_not_found() {
        let response = Response::not_found();
        assert_eq!(response.status, 404);
    }

    #[test]
    fn test_response_internal_error() {
        let response = Response::internal_error();
        assert_eq!(response.status, 500);
    }
}

