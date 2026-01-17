use super::{Request, Response};
use std::sync::Arc;

/// Type alias for dynamic handler functions
pub type HandlerFn = Arc<dyn Fn(&Request) -> Response + Send + Sync>;

/// A handler that returns either a static or dynamic HTTP response
#[derive(Clone)]
pub enum Handler {
    /// Static response - always returns the same response
    Static(Response),
    /// Dynamic response - builds response based on the request
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
    /// Create a new static handler with a predefined response
    pub fn new(response: Response) -> Self {
        Handler::Static(response)
    }

    /// Create a dynamic handler that builds responses based on the request
    pub fn dynamic<F>(f: F) -> Self
    where
        F: Fn(&Request) -> Response + Send + Sync + 'static,
    {
        Handler::Dynamic(Arc::new(f))
    }

    /// Create a static handler from a JSON value
    pub fn from_json<T: serde::Serialize>(value: &T) -> Self {
        Handler::Static(Response::ok().with_json(value))
    }

    /// Modify the status code (only works for static handlers, returns a new static handler)
    pub fn with_status(self, status: u16) -> Self {
        match self {
            Handler::Static(mut response) => {
                response.status = status;
                Handler::Static(response)
            }
            Handler::Dynamic(_) => self, // Cannot modify dynamic handler
        }
    }

    /// Add a header (only works for static handlers)
    pub fn with_header(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        match self {
            Handler::Static(response) => Handler::Static(response.with_header(key, value)),
            Handler::Dynamic(_) => self, // Cannot modify dynamic handler
        }
    }

    /// Get the response for a given request
    pub fn respond(&self, request: &Request) -> Response {
        match self {
            Handler::Static(response) => response.clone(),
            Handler::Dynamic(f) => f(request),
        }
    }

    /// Get the static response (for backwards compatibility)
    /// Returns a default response for dynamic handlers
    pub fn response(&self) -> Response {
        match self {
            Handler::Static(response) => response.clone(),
            Handler::Dynamic(_) => Response::new(200),
        }
    }
}

impl From<Response> for Handler {
    fn from(response: Response) -> Self {
        Handler::Static(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Method;
    use std::collections::HashMap;

    fn create_test_request(method: Method, path: &str, body: &[u8]) -> Request {
        Request {
            method,
            path: path.to_string(),
            headers: HashMap::new(),
            body: body.to_vec(),
        }
    }

    #[test]
    fn test_handler_new() {
        let response = Response::new(200);
        let handler = Handler::new(response.clone());
        assert!(matches!(handler, Handler::Static(_)));
    }

    #[test]
    fn test_handler_from_json() {
        let handler = Handler::from_json(&serde_json::json!({"test": true}));
        let req = create_test_request(Method::Get, "/", &[]);
        let response = handler.respond(&req);
        assert_eq!(response.status, 200);
        assert!(response.headers.get("content-type").unwrap().contains("application/json"));
    }

    #[test]
    fn test_handler_with_status() {
        let handler = Handler::from_json(&serde_json::json!({})).with_status(201);
        let req = create_test_request(Method::Get, "/", &[]);
        assert_eq!(handler.respond(&req).status, 201);
    }

    #[test]
    fn test_handler_from_response() {
        let response = Response::new(404);
        let handler: Handler = response.into();
        let req = create_test_request(Method::Get, "/", &[]);
        assert_eq!(handler.respond(&req).status, 404);
    }

    #[test]
    fn test_dynamic_handler() {
        let handler = Handler::dynamic(|req: &Request| {
            let body = format!("You requested: {}", req.path);
            Response::new(200).with_body(body)
        });

        let req = create_test_request(Method::Get, "/api/users", &[]);
        let response = handler.respond(&req);
        assert_eq!(response.status, 200);
        assert!(String::from_utf8_lossy(&response.body).contains("/api/users"));
    }

    #[test]
    fn test_dynamic_handler_with_body() {
        let handler = Handler::dynamic(|req: &Request| {
            if let Some(body_str) = req.body_as_str() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
                    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                        return Response::new(200).with_json(&serde_json::json!({
                            "message": format!("Hello, {}!", name)
                        }));
                    }
                }
            }
            Response::new(400).with_body("Invalid request")
        });

        let req = create_test_request(Method::Post, "/greet", b"{\"name\": \"World\"}");
        let response = handler.respond(&req);
        assert_eq!(response.status, 200);
        assert!(String::from_utf8_lossy(&response.body).contains("Hello, World!"));
    }

    #[test]
    fn test_dynamic_handler_based_on_method() {
        let handler = Handler::dynamic(|req: &Request| {
            match req.method {
                Method::Get => Response::new(200).with_body("GET response"),
                Method::Post => Response::new(201).with_body("POST response"),
                _ => Response::new(405).with_body("Method not allowed"),
            }
        });

        let get_req = create_test_request(Method::Get, "/", &[]);
        assert_eq!(handler.respond(&get_req).status, 200);

        let post_req = create_test_request(Method::Post, "/", &[]);
        assert_eq!(handler.respond(&post_req).status, 201);

        let delete_req = create_test_request(Method::Delete, "/", &[]);
        assert_eq!(handler.respond(&delete_req).status, 405);
    }
}

