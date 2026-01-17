# graphql-operation-server-harness

Mock GraphQL servers for integration tests. Spin up a real GraphQL server with predefined responses that automatically shuts down when all expected requests are handled.

## Installation

```toml
[dev-dependencies]
graphql-operation-server-harness = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use graphql_operation_server_harness::prelude::*;

let collected = ScenarioBuilder::new()
    .server(AsyncGraphQL::bind("127.0.0.1:8080".parse().unwrap()))
    .collector(DefaultCollector::new())
    .operation(
        Operation::query()
            .with_field(
                Field::new("users")
                    .with_handler(Handler::new(json!([{"id": 1, "name": "Alice"}])))
            )
    )
    .build()
    .execute()
    .await?;

assert_eq!(collected.len(), 1);
assert!(collected[0].query.contains("users"));
```

## Real-World Scenarios

### Polling Service Testing

Test a component that periodically queries a GraphQL endpoint for updates:

```rust
#[tokio::test]
async fn test_graphql_polling() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    // Component polls for sync status every second
    let component = spawn_sync_poller(addr);

    let collected = ScenarioBuilder::new()
        .server(AsyncGraphQL::bind(addr))
        .collector(DefaultCollector::new())
        .operation(
            Operation::query()
                .with_field(
                    Field::new("syncStatus")
                        // First 2 calls: syncing
                        .with_handler(Handler::new(json!({"status": "syncing", "progress": 50})))
                        .with_handler(Handler::new(json!({"status": "syncing", "progress": 80})))
                        // Third call: complete
                        .with_handler(Handler::new(json!({"status": "complete", "progress": 100})))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Component polled 3 times
    assert_eq!(collected.len(), 3);
    assert!(component.sync_complete());
}
```

### Pagination Testing

Test a component that fetches paginated data:

```rust
#[tokio::test]
async fn test_pagination() {
    let addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(AsyncGraphQL::bind(addr))
        .collector(DefaultCollector::new())
        .operation(
            Operation::query()
                .with_field(
                    Field::new("users")
                        // Page 1
                        .with_handler(Handler::new(json!({
                            "nodes": [{"id": 1}, {"id": 2}],
                            "pageInfo": {"hasNextPage": true, "cursor": "abc"}
                        })))
                        // Page 2
                        .with_handler(Handler::new(json!({
                            "nodes": [{"id": 3}, {"id": 4}],
                            "pageInfo": {"hasNextPage": true, "cursor": "def"}
                        })))
                        // Page 3 (last)
                        .with_handler(Handler::new(json!({
                            "nodes": [{"id": 5}],
                            "pageInfo": {"hasNextPage": false, "cursor": null}
                        })))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Verify cursor was passed correctly
    assert_eq!(collected.len(), 3);
    assert!(collected[1].variables.as_ref().unwrap()["after"] == "abc");
    assert!(collected[2].variables.as_ref().unwrap()["after"] == "def");
}
```

### Error Handling Testing

Test how your client handles GraphQL errors:

```rust
#[tokio::test]
async fn test_partial_error_response() {
    let addr: SocketAddr = "127.0.0.1:8082".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(AsyncGraphQL::bind(addr))
        .collector(DefaultCollector::new())
        .operation(
            Operation::query()
                .with_field(
                    Field::new("user")
                        .with_handler(Handler::with_error("User not found"))
                )
                .with_field(
                    Field::new("posts")
                        .with_handler(Handler::new(json!([{"id": 1, "title": "Hello"}])))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Response has both data and errors
    // { "data": { "posts": [...] }, "errors": [{ "message": "User not found" }] }
}
```

### Mutation Flow Testing

Test a complete CRUD workflow:

```rust
#[tokio::test]
async fn test_crud_workflow() {
    let addr: SocketAddr = "127.0.0.1:8083".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(AsyncGraphQL::bind(addr))
        .collector(DefaultCollector::new())
        // Create
        .operation(
            Operation::mutation()
                .with_field(
                    Field::new("createUser")
                        .with_handler(Handler::dynamic(|ctx| {
                            let name = ctx.get_variable("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown");
                            HandlerResponse::new(json!({"id": 42, "name": name}))
                        }))
                )
        )
        // Update
        .operation(
            Operation::mutation()
                .with_field(
                    Field::new("updateUser")
                        .with_handler(Handler::new(json!({"id": 42, "name": "Updated"})))
                )
        )
        // Delete
        .operation(
            Operation::mutation()
                .with_field(
                    Field::new("deleteUser")
                        .with_handler(Handler::new(json!(true)))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    assert_eq!(collected.len(), 3);
}
```

### BFF (Backend-for-Frontend) Testing

Mock a downstream GraphQL service:

```rust
#[tokio::test]
async fn test_bff_aggregation() {
    let addr: SocketAddr = "127.0.0.1:8084".parse().unwrap();

    // Your BFF service aggregates data from a downstream GraphQL API
    let collected = ScenarioBuilder::new()
        .server(AsyncGraphQL::bind(addr))
        .collector(DefaultCollector::new())
        .operation(
            Operation::query()
                .with_field(
                    Field::new("user")
                        .with_handler(Handler::new(json!({"id": 1, "name": "Alice"})))
                )
                .with_field(
                    Field::new("userOrders")
                        .with_handler(Handler::new(json!([{"id": 100, "total": 50}])))
                )
                .with_field(
                    Field::new("userPreferences")
                        .with_handler(Handler::new(json!({"theme": "dark"})))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    // BFF made one query that fetched all fields
    assert_eq!(collected.len(), 1);
}
```

## Common Patterns

### Sequential Responses

Each call gets the next handler:

```rust
Field::new("counter")
    .with_handler(Handler::new(json!({"value": 1})))  // 1st call
    .with_handler(Handler::new(json!({"value": 2})))  // 2nd call
    .with_handler(Handler::new(json!({"value": 3}))) // 3rd call
```

### Dynamic Responses

Build responses based on variables:

```rust
Field::new("user")
    .with_handler(Handler::dynamic(|ctx| {
        let id = ctx.get_variable("id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        HandlerResponse::new(json!({"id": id, "name": format!("User {}", id)}))
    }))
```

### Error Simulation

```rust
// Simple error
Handler::with_error("Something went wrong")

// Error at specific path
Handler::with_error_at_path("Invalid email", vec!["user", "email"])
```

### Queries and Mutations

```rust
// Query
Operation::query()
    .with_field(Field::new("users").with_handler(...))

// Mutation
Operation::mutation()
    .with_field(Field::new("createUser").with_handler(...))
```

### Multiple Fields

```rust
Operation::query()
    .with_field(Field::new("user").with_handler(Handler::new(json!({"id": 1}))))
    .with_field(Field::new("posts").with_handler(Handler::new(json!([]))))
    .with_field(Field::new("comments").with_handler(Handler::new(json!([]))))
```

### Request Assertions

```rust
let collected = scenario.execute().await?;

// Count
assert_eq!(collected.len(), 2);

// Query content
assert!(collected[0].query.contains("users"));
assert!(collected[1].query.contains("createUser"));

// Variables
let vars = collected[0].variables.as_ref().unwrap();
assert_eq!(vars["limit"], 10);
assert_eq!(vars["offset"], 0);

// Operation name
assert_eq!(collected[0].operation_name, Some("GetUsers".into()));
```

## License

MIT - see [LICENSE](../../LICENSE) for details.
