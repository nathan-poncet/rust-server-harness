# graphql-operation-server-harness

A Rust library for creating **mock GraphQL servers** in your integration tests. Instead of mocking your GraphQL client, spin up a real GraphQL server that responds exactly as you configure it.

## 🎯 Why Use This?

When testing code that calls GraphQL APIs, you need to verify that:
- Your code sends the **correct queries/mutations** (right fields, variables, operation names)
- Your code **handles responses correctly** (data parsing, error handling, partial responses)

**Traditional approaches have drawbacks:**

| Approach | Problem |
|----------|---------|
| Mock the GraphQL client | Doesn't test actual query building or response parsing |
| Use a shared test server | Flaky tests, shared state, requires maintaining a schema |
| Schema-based mocking | Complex setup, may not match production behavior |

**Server Harness gives you:**
- ✅ **Real GraphQL requests** - Your code makes actual HTTP requests with GraphQL
- ✅ **Isolated per test** - Each test gets its own server with its own responses
- ✅ **No schema required** - Define query/mutation responses dynamically
- ✅ **Request inspection** - Assert on queries, variables, and operation names

## 📦 Use Cases

- **Testing GraphQL clients** - Verify your client sends correct queries and variables
- **Integration testing** - Test your app's behavior with specific GraphQL responses
- **Error scenario testing** - Simulate GraphQL errors (field errors, network errors)
- **Partial response testing** - Test handling of `data` + `errors` combined responses
- **BFF testing** - Mock downstream GraphQL services in Backend-for-Frontend tests

## ✨ Features

- 🏗️ **Builder Pattern** - Fluent API with `ScenarioBuilder` for defining test scenarios
- 🔄 **Auto-shutdown** - Server automatically shuts down when all handlers have been called
- ⚡ **Static & Dynamic Handlers** - Predefined responses or compute responses based on variables
- 📝 **Request Collection** - Capture all incoming requests (query, variables, operation name)
- 🔁 **Sequential Handlers** - Return different responses for successive calls to the same field
- 🌐 **async-graphql Backend** - Built on the mature async-graphql library

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

## 🔧 How It Works

```
┌─────────────────┐                    ┌──────────────────┐
│   Your Code     │  POST /graphql     │   Mock Server    │
│ (GraphQL Client)│───────────────────▶│ (async-graphql)  │
│                 │  { query, vars }   │                  │
│                 │◀───────────────────│  Returns JSON    │
│                 │  { data, errors }  │  you configured  │
└─────────────────┘                    └──────────────────┘
                                              │
                                              ▼
                                       Auto-shutdown when
                                       all handlers consumed
                                              │
                                              ▼
                                       ┌──────────────────┐
                                       │ Collected Requests│
                                       │ (query, variables,│
                                       │  operation name)  │
                                       └──────────────────┘
```

1. **Define operations** - Specify queries/mutations and their field responses
2. **Execute scenario** - Server starts and listens for GraphQL requests
3. **Your code runs** - Makes real GraphQL calls to the mock server
4. **Auto-shutdown** - Server stops when all expected handlers have responded
5. **Assert** - Verify collected requests match expectations

## License

MIT - see [LICENSE](../../LICENSE) for details.
