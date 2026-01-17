use super::Method;

/// Represents a gRPC service with methods
#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub methods: Vec<Method>,
}

impl Service {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            methods: Vec::new(),
        }
    }

    pub fn with_method(mut self, method: Method) -> Self {
        self.methods.push(method);
        self
    }

    pub fn with_methods(mut self, methods: impl IntoIterator<Item = Method>) -> Self {
        self.methods.extend(methods);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_new() {
        let service = Service::new("my.package.UserService");
        assert_eq!(service.name, "my.package.UserService");
        assert!(service.methods.is_empty());
    }

    #[test]
    fn test_service_with_method() {
        let service = Service::new("my.package.UserService")
            .with_method(Method::new("GetUser"));
        assert_eq!(service.methods.len(), 1);
    }

    #[test]
    fn test_service_with_multiple_methods() {
        let service = Service::new("my.package.UserService")
            .with_method(Method::new("GetUser"))
            .with_method(Method::new("CreateUser"))
            .with_method(Method::new("DeleteUser"));
        assert_eq!(service.methods.len(), 3);
    }
}
