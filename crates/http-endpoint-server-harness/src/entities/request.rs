use std::collections::HashMap;

/// Represents an HTTP request received by the harness
#[derive(Debug, Clone)]
pub struct Request {
    pub method: super::Method,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn new(method: super::Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn body_as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.body).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Method;
    use std::collections::HashMap;

    #[test]
    fn test_request_body_as_str() {
        let request = Request {
            method: Method::Post,
            path: "/test".to_string(),
            headers: HashMap::new(),
            body: b"Hello World".to_vec(),
        };
        assert_eq!(request.body_as_str(), Some("Hello World"));
    }

    #[test]
    fn test_request_body_as_str_invalid_utf8() {
        let request = Request {
            method: Method::Post,
            path: "/test".to_string(),
            headers: HashMap::new(),
            body: vec![0xFF, 0xFE],
        };
        assert_eq!(request.body_as_str(), None);
    }

    #[test]
    fn test_request_method_display() {
        assert_eq!(format!("{}", Method::Get), "GET");
        assert_eq!(format!("{}", Method::Post), "POST");
        assert_eq!(format!("{}", Method::Put), "PUT");
        assert_eq!(format!("{}", Method::Delete), "DELETE");
    }
}

