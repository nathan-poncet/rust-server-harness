use super::Message;
use std::sync::Arc;

/// Context passed to dynamic handlers
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub service: String,
    pub method: String,
    pub message: Message,
}

impl RequestContext {
    pub fn new(service: impl Into<String>, method: impl Into<String>, message: Message) -> Self {
        Self {
            service: service.into(),
            method: method.into(),
            message,
        }
    }
}

/// Type alias for dynamic handler functions
pub type HandlerFn = Arc<dyn Fn(&RequestContext) -> Message + Send + Sync>;

/// A handler that returns either a static or dynamic gRPC response
#[derive(Clone)]
pub enum Handler {
    /// Static response - always returns the same message
    Static(Message),
    /// Dynamic response - builds message based on the request context
    Dynamic(HandlerFn),
}

impl std::fmt::Debug for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Handler::Static(msg) => f.debug_tuple("Static").field(msg).finish(),
            Handler::Dynamic(_) => f.debug_tuple("Dynamic").field(&"<fn>").finish(),
        }
    }
}

impl Handler {
    /// Create a new static handler with a predefined message
    pub fn new(message: Message) -> Self {
        Handler::Static(message)
    }

    /// Create a dynamic handler that builds responses based on the request context
    pub fn dynamic<F>(f: F) -> Self
    where
        F: Fn(&RequestContext) -> Message + Send + Sync + 'static,
    {
        Handler::Dynamic(Arc::new(f))
    }

    /// Create a static handler from raw bytes
    pub fn from_bytes(data: impl Into<Vec<u8>>) -> Self {
        Handler::Static(Message::new(data))
    }

    /// Create a static handler from a prost message
    pub fn from_prost<T: prost::Message>(msg: &T) -> Self {
        Handler::Static(Message::from_prost(msg))
    }

    /// Get the response for a given request context
    pub fn respond(&self, ctx: &RequestContext) -> Message {
        match self {
            Handler::Static(msg) => msg.clone(),
            Handler::Dynamic(f) => f(ctx),
        }
    }

    /// Get the static response (for backwards compatibility)
    /// Note: For dynamic handlers, this returns a reference to an empty message constant.
    /// Prefer using `respond()` for dynamic handlers.
    pub fn response(&self) -> &Message {
        // Use a constant static message for dynamic handlers
        static EMPTY_MESSAGE: std::sync::LazyLock<Message> =
            std::sync::LazyLock::new(|| Message { data: Vec::new() });
        match self {
            Handler::Static(msg) => msg,
            Handler::Dynamic(_) => &EMPTY_MESSAGE,
        }
    }

    /// Convert handler into the response message (for backwards compatibility)
    pub fn into_response(self) -> Message {
        match self {
            Handler::Static(msg) => msg,
            Handler::Dynamic(_) => Message::empty(),
        }
    }
}

impl From<Message> for Handler {
    fn from(message: Message) -> Self {
        Handler::Static(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_new() {
        let msg = Message::new(vec![1, 2, 3]);
        let handler = Handler::new(msg);
        assert!(matches!(handler, Handler::Static(_)));
    }

    #[test]
    fn test_handler_from_bytes() {
        let handler = Handler::from_bytes(vec![4, 5, 6]);
        let ctx = RequestContext::new("Svc", "Method", Message::empty());
        assert_eq!(handler.respond(&ctx).data, vec![4, 5, 6]);
    }

    #[test]
    fn test_handler_into_response() {
        let handler = Handler::from_bytes(vec![1, 2, 3]);
        let msg = handler.into_response();
        assert_eq!(msg.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_handler_from_message() {
        let msg = Message::new(vec![7, 8, 9]);
        let handler: Handler = msg.into();
        let ctx = RequestContext::new("Svc", "Method", Message::empty());
        assert_eq!(handler.respond(&ctx).data, vec![7, 8, 9]);
    }

    #[test]
    fn test_dynamic_handler() {
        let handler = Handler::dynamic(|ctx: &RequestContext| {
            // Echo back the input with a prefix
            let mut response = vec![0xFF];
            response.extend_from_slice(&ctx.message.data);
            Message::new(response)
        });

        let ctx = RequestContext::new("TestService", "TestMethod", Message::new(vec![1, 2, 3]));
        let response = handler.respond(&ctx);
        assert_eq!(response.data, vec![0xFF, 1, 2, 3]);
    }

    #[test]
    fn test_dynamic_handler_based_on_method() {
        let handler = Handler::dynamic(|ctx: &RequestContext| {
            match ctx.method.as_str() {
                "GetUser" => Message::new(vec![1, 0, 0]),
                "CreateUser" => Message::new(vec![2, 0, 0]),
                _ => Message::new(vec![0, 0, 0]),
            }
        });

        let ctx1 = RequestContext::new("UserService", "GetUser", Message::empty());
        assert_eq!(handler.respond(&ctx1).data, vec![1, 0, 0]);

        let ctx2 = RequestContext::new("UserService", "CreateUser", Message::empty());
        assert_eq!(handler.respond(&ctx2).data, vec![2, 0, 0]);
    }
}
