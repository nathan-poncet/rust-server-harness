# http-endpoint-server-harness

A Rust library for creating mock HTTP servers for testing purposes. Built with **Clean Architecture** principles and a fluent **Builder Pattern** API.

## Features

- 🏗️ **Builder Pattern** - Fluent API with `ScenarioBuilder` for defining test scenarios
- 🔄 **Auto-shutdown** - Server automatically shuts down when all handlers have been called
- ⚡ **Static & Dynamic Handlers** - Support for predefined responses and dynamic responses based on request context
- 📝 **Request Collection** - Collect all incoming requests for assertions
- 🌐 **Axum Backend** - Built on top of the Axum web framework
- 🧱 **Clean Architecture** - Proper separation of entities, use cases, and adapters

## Installation

```toml
[dev-dependencies]
http-endpoint-server-harness = "0.1"
tokio = { version = "1", features = ["full"] }
reqwest = "0.12"
```

## Quick Start

```rust
use http_endpoint_server_harness::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), HarnessError> {
    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();

    // Spawn a task to make HTTP requests
    let requests_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let client = reqwest::Client::new();
        client.get(format!("http://{}/api/users", addr))
            .send()
            .await
            .unwrap();
    });

    // Build and execute the scenario
    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/users", Method::Get)
                .with_handler(Handler::from_json(&serde_json::json!({
                    "users": [{"id": 1, "name": "Alice"}]
                })))
        )
        .build()
        .execute()
        .await?;

    requests_task.await.unwrap();

    // Assert on collected requests
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].path, "/api/users");

    Ok(())
}
```

## Dynamic Handlers

Create handlers that respond dynamically based on the request:

```rust
let endpoint = Endpoint::new("/api/echo", Method::Post)
    .with_handler(Handler::dynamic(|ctx| {
        Response::ok()
            .with_json_body(&serde_json::json!({
                "echoed": ctx.body_as_str().unwrap_or(""),
                "method": format!("{:?}", ctx.method),
                "path": ctx.path
            }))
            .unwrap()
    }));
```

## Sequential Handlers

Define multiple handlers for the same endpoint - each subsequent request uses the next handler:

```rust
let endpoint = Endpoint::new("/api/counter", Method::Get)
    .with_handler(Handler::from_json(&json!({"count": 1})))
    .with_handler(Handler::from_json(&json!({"count": 2})))
    .with_handler(Handler::from_json(&json!({"count": 3})));
```

## Custom Responses

```rust
// Custom status code
let handler = Handler::new(Response::new(201).with_body("Created"));

// Custom headers
let handler = Handler::new(
    Response::ok()
        .with_header("X-Custom-Header", "value")
        .with_json_body(&json!({"success": true}))
        .unwrap()
);
```

## Architecture

```
src/
├── entities/           # Endpoint, Handler, Method, Request, Response, Scenario
├── use_cases/          # ScenarioBuilder, ports (Server, Collector traits)
├── adapters/gateways/  # Axum server implementation
└── lib.rs              # Public API and prelude
```

## License

MIT - see [LICENSE](../../LICENSE) for details.

