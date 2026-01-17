use super::Message;

/// A collected gRPC request
#[derive(Debug, Clone)]
pub struct CollectedRequest {
    pub service: String,
    pub method: String,
    pub message: Message,
}

impl CollectedRequest {
    pub fn new(service: impl Into<String>, method: impl Into<String>, message: Message) -> Self {
        Self {
            service: service.into(),
            method: method.into(),
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collected_request_new() {
        let req = CollectedRequest::new("UserService", "GetUser", Message::new(vec![1, 2, 3]));
        assert_eq!(req.service, "UserService");
        assert_eq!(req.method, "GetUser");
        assert_eq!(req.message.data, vec![1, 2, 3]);
    }
}
