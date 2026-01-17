# graphql-operation-server-harness

A Rust library for creating mock GraphQL servers for testing purposes. Built with **Clean Architecture** principles and a fluent **Builder Pattern** API.

## Features

- 🏗️ **Builder Pattern** - Fluent API with `ScenarioBuilder` for defining test scenarios
- 🔄 **Auto-shutdown** - Server automatically shuts down when all handlers have been called
- ⚡ **Static & Dynamic Handlers** - Support for predefined responses and dynamic responses based on request context
- 📝 **Request Collection** - Collect all incoming requests for assertions
- 🌐 **async-graphql Backend** - Built on top of async-graphql and Axum
- 🧱 **Clean Architecture** - Proper separation of entities, use cases, and adapters

## Installation

```toml
[dev-dependencies]
graphql-operation-server-harness = "0.1"
tokio = { version = "1", features = ["full"] }
reqwest = "0.12"
```

## Quick Start

```rust
use graphql_operation_server_harness::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), HarnessError> {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

    // Spawn a task to make GraphQL requests
    let requests_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let client = reqwest::Client::new();
        client.post(format!("http://{}/", addr))
            .json(&serde_json::json!({
                "query": "{ users { id name } }"
            }))
            .send()
            .await
            .unwrap();
    });

    // Build and execute the scenario
    let collected = ScenarioBuilder::new()
        .server(AsyncGraphQL::bind(addr))
        .collector(DefaultCollector::new())
        .operation(
            Operation::query()
                .with_field(
                    Field::new("users")
                        .with_handler(Handler::new(serde_json::json!([
                            {"id": 1, "name": "Alice"}
                        ])))
                )
        )
        .build()
        .execute()
        .await?;

    requests_task.await.unwrap();

    // Assert on collected requests
    assert_eq!(collected.len(), 1);

    Ok(())
}
```

## Dynamic Handlers

Create handlers that respond dynamically based on the request variables:

```rust
let field = Field::new("createUser")
    .with_handler(Handler::dynamic(|ctx| {
        let name = ctx.variables
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        serde_json::json!({
            "id": 42,
            "name": name
        })
    }));
```

## Mutations

```rust
let scenario = ScenarioBuilder::new()
    .server(AsyncGraphQL::bind(addr))
    .collector(DefaultCollector::new())
    .operation(
        Operation::mutation()
            .with_field(
                Field::new("createUser")
                    .with_handler(Handler::new(json!({"id": 1, "name": "New User"})))
            )
            .with_field(
                Field::new("deleteUser")
                    .with_handler(Handler::new(json!(true)))
            )
    )
    .build();
```

## Error Responses

```rust
// Field with error
let handler = Handler::with_error("Something went wrong");

// Error at specific path
let handler = Handler::with_error_at_path(
    "Validation failed",
    vec!["user", "email"]
);
```

## Multiple Operations

```rust
let scenario = ScenarioBuilder::new()
    .server(AsyncGraphQL::bind(addr))
    .collector(DefaultCollector::new())
    .operation(
        Operation::query()
            .with_field(Field::new("users").with_handler(Handler::new(json!([]))))
            .with_field(Field::new("posts").with_handler(Handler::new(json!([]))))
    )
    .operation(
        Operation::mutation()
            .with_field(Field::new("createUser").with_handler(Handler::new(json!({}))))
    )
    .build();
```

## Architecture

```
src/
├── entities/           # Operation, Field, Handler, Scenario
├── use_cases/          # ScenarioBuilder, ports (Server, Collector traits)
├── adapters/gateways/  # async-graphql server implementation
└── lib.rs              # Public API and prelude
```

## License

MIT - see [LICENSE](../../LICENSE) for details.

