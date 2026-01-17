# http-endpoint-server-harness

A Rust library for creating **mock HTTP servers** in your integration tests. Instead of mocking your HTTP client, spin up a real server that responds exactly as you configure it.

## 🎯 Why Use This?

When testing code that calls external HTTP APIs, you need to verify that:
- Your code sends the **correct requests** (right path, method, headers, body)
- Your code **handles responses correctly** (parsing, error handling, edge cases)

**Traditional approaches have drawbacks:**

| Approach | Problem |
|----------|---------|
| Mock the HTTP client | Doesn't test actual serialization, headers, or network code |
| Use a shared test server | Flaky tests, shared state issues, hard to customize per test |
| Record/replay (VCR) | Brittle when APIs change, hard to test error scenarios |

**Server Harness gives you:**
- ✅ **Real HTTP requests** - Your code makes actual network calls
- ✅ **Isolated per test** - Each test gets its own server with its own responses
- ✅ **Full control** - Define exactly what each endpoint returns
- ✅ **Request inspection** - Assert on the exact requests your code made

## 📦 Use Cases

- **Testing REST API clients** - Verify your client library sends correct requests
- **Integration testing** - Test your app's behavior with specific API responses
- **Error scenario testing** - Simulate 500 errors, timeouts, malformed JSON
- **Contract testing** - Ensure your code handles the expected API format
- **Webhook testing** - Verify your code sends webhooks correctly

## ✨ Features

- 🏗️ **Builder Pattern** - Fluent API with `ScenarioBuilder` for defining test scenarios
- 🔄 **Auto-shutdown** - Server automatically shuts down when all handlers have been called
- ⚡ **Static & Dynamic Handlers** - Predefined responses or compute responses based on the request
- 📝 **Request Collection** - Capture all incoming requests for assertions
- 🔁 **Sequential Handlers** - Return different responses for successive calls
- 🌐 **Axum Backend** - Built on the battle-tested Axum web framework

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

## 🔧 How It Works

```
┌─────────────────┐                    ┌──────────────────┐
│   Your Code     │   GET /api/users   │   Mock Server    │
│  (HTTP Client)  │───────────────────▶│   (Axum-based)   │
│                 │                    │                  │
│                 │◀───────────────────│  Returns JSON    │
│                 │   200 OK + JSON    │  you configured  │
└─────────────────┘                    └──────────────────┘
                                              │
                                              ▼
                                       Auto-shutdown when
                                       all handlers consumed
                                              │
                                              ▼
                                       ┌──────────────────┐
                                       │ Collected Requests│
                                       │ for assertions   │
                                       └──────────────────┘
```

1. **Define endpoints** - Specify path, method, and response for each endpoint
2. **Execute scenario** - Server starts and waits for requests
3. **Your code runs** - Makes real HTTP calls to the mock server
4. **Auto-shutdown** - Server stops when all expected handlers have responded
5. **Assert** - Verify collected requests match expectations

## License

MIT - see [LICENSE](../../LICENSE) for details.
