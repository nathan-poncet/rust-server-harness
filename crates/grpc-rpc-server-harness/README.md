# grpc-rpc-server-harness

A Rust library for creating **mock gRPC servers** in your integration tests. Instead of mocking your gRPC client or stubs, spin up a real gRPC server that responds exactly as you configure it.

## 🎯 Why Use This?

When testing code that calls gRPC services, you need to verify that:
- Your code sends the **correct requests** (right service, method, protobuf message)
- Your code **handles responses correctly** (deserialization, error codes, edge cases)

**Traditional approaches have drawbacks:**

| Approach | Problem |
|----------|---------|
| Mock the generated stubs | Doesn't test actual protobuf serialization or HTTP/2 layer |
| Use a shared test server | Flaky tests, shared state, requires infrastructure |
| Mock at the transport layer | Complex setup, easy to miss protocol details |

**Server Harness gives you:**
- ✅ **Real gRPC calls** - Your code makes actual HTTP/2 requests with protobuf
- ✅ **Isolated per test** - Each test gets its own server with its own responses
- ✅ **No .proto files needed** - Define services/methods dynamically at runtime
- ✅ **Request inspection** - Assert on the exact requests your code made

## 📦 Use Cases

- **Testing gRPC clients** - Verify your client code sends correct protobuf messages
- **Integration testing** - Test your app's behavior with specific gRPC responses
- **Error scenario testing** - Simulate gRPC error codes (UNAVAILABLE, INTERNAL, etc.)
- **Microservice testing** - Mock dependent services in your test environment
- **Load testing setup** - Create predictable mock backends for load tests

## ✨ Features

- 🏗️ **Builder Pattern** - Fluent API with `ScenarioBuilder` for defining test scenarios
- 🔄 **Auto-shutdown** - Server automatically shuts down when all handlers have been called
- ⚡ **Static & Dynamic Handlers** - Predefined responses or compute responses based on the request
- 📝 **Request Collection** - Capture all incoming requests (service, method, message bytes)
- 🔁 **Sequential Handlers** - Return different responses for successive calls
- 🌐 **Tonic-compatible** - Works with any gRPC client over standard HTTP/2

## Installation

```toml
[dev-dependencies]
grpc-rpc-server-harness = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use grpc_rpc_server_harness::prelude::*;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), HarnessError> {
    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();

    // Spawn a task to make gRPC requests
    let requests_task = tokio::spawn(async move {
        // Your gRPC client code here
    });

    // Build and execute the scenario
    let collected = ScenarioBuilder::new()
        .server(Tonic::bind(addr))
        .collector(DefaultCollector::new())
        .service(
            Service::new("my.package.UserService")
                .with_method(
                    Method::new("GetUser")
                        .with_handler(Handler::from_bytes(vec![/* protobuf bytes */]))
                )
        )
        .build()
        .execute()
        .await?;

    requests_task.await.unwrap();

    // Assert on collected requests
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].service, "my.package.UserService");
    assert_eq!(collected[0].method, "GetUser");

    Ok(())
}
```

## Dynamic Handlers

Create handlers that respond dynamically based on the request:

```rust
let method = Method::new("Echo")
    .with_handler(Handler::dynamic(|ctx| {
        // Echo back the request with a prefix
        let mut response = vec![0xFF];
        response.extend_from_slice(&ctx.message.data);
        Message::new(response)
    }));
```

## With Prost Messages

Serialize protobuf messages directly:

```rust
use prost::Message as ProstMessage;

#[derive(ProstMessage)]
struct GetUserResponse {
    #[prost(string, tag = "1")]
    name: String,
}

let handler = Handler::from_prost(&GetUserResponse {
    name: "Alice".to_string(),
});
```

## Multiple Services

```rust
let scenario = ScenarioBuilder::new()
    .server(Tonic::bind(addr))
    .collector(DefaultCollector::new())
    .service(
        Service::new("my.package.UserService")
            .with_method(Method::new("GetUser").with_handler(Handler::from_bytes(vec![])))
            .with_method(Method::new("CreateUser").with_handler(Handler::from_bytes(vec![])))
    )
    .service(
        Service::new("my.package.OrderService")
            .with_method(Method::new("GetOrder").with_handler(Handler::from_bytes(vec![])))
    )
    .build();
```

## 🔧 How It Works

```
┌─────────────────┐                    ┌──────────────────┐
│   Your Code     │  gRPC Request      │   Mock Server    │
│  (gRPC Client)  │───────────────────▶│   (Tonic-based)  │
│                 │  HTTP/2 + Protobuf │                  │
│                 │◀───────────────────│  Returns bytes   │
│                 │  Protobuf Response │  you configured  │
└─────────────────┘                    └──────────────────┘
                                              │
                                              ▼
                                       Auto-shutdown when
                                       all handlers consumed
                                              │
                                              ▼
                                       ┌──────────────────┐
                                       │ Collected Requests│
                                       │ (service, method, │
                                       │  message bytes)   │
                                       └──────────────────┘
```

1. **Define services** - Specify service name, methods, and protobuf responses
2. **Execute scenario** - Server starts and listens for gRPC calls
3. **Your code runs** - Makes real gRPC calls to the mock server
4. **Auto-shutdown** - Server stops when all expected handlers have responded
5. **Assert** - Verify collected requests match expectations

## License

MIT - see [LICENSE](../../LICENSE) for details.
