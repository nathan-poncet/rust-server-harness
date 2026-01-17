use super::Field;

/// Type of GraphQL operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Query,
    Mutation,
    Subscription,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Query => write!(f, "query"),
            OperationType::Mutation => write!(f, "mutation"),
            OperationType::Subscription => write!(f, "subscription"),
        }
    }
}

/// Represents a GraphQL operation (Query, Mutation, or Subscription)
#[derive(Debug, Clone)]
pub struct Operation {
    pub operation_type: OperationType,
    pub fields: Vec<Field>,
}

impl Operation {
    pub fn query() -> Self {
        Self {
            operation_type: OperationType::Query,
            fields: Vec::new(),
        }
    }

    pub fn mutation() -> Self {
        Self {
            operation_type: OperationType::Mutation,
            fields: Vec::new(),
        }
    }

    pub fn subscription() -> Self {
        Self {
            operation_type: OperationType::Subscription,
            fields: Vec::new(),
        }
    }

    pub fn with_field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    pub fn with_fields(mut self, fields: impl IntoIterator<Item = Field>) -> Self {
        self.fields.extend(fields);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_query() {
        let op = Operation::query();
        assert_eq!(op.operation_type, OperationType::Query);
        assert!(op.fields.is_empty());
    }

    #[test]
    fn test_operation_mutation() {
        let op = Operation::mutation();
        assert_eq!(op.operation_type, OperationType::Mutation);
    }

    #[test]
    fn test_operation_subscription() {
        let op = Operation::subscription();
        assert_eq!(op.operation_type, OperationType::Subscription);
    }

    #[test]
    fn test_operation_with_field() {
        let op = Operation::query()
            .with_field(Field::new("users"));
        assert_eq!(op.fields.len(), 1);
    }

    #[test]
    fn test_operation_type_display() {
        assert_eq!(format!("{}", OperationType::Query), "query");
        assert_eq!(format!("{}", OperationType::Mutation), "mutation");
        assert_eq!(format!("{}", OperationType::Subscription), "subscription");
    }
}
