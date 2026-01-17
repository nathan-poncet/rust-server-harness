# grpc-rpc-server-harness

A Rust library for creating mock gRPC servers for testing purposes. Built with **Clean Architecture** principles and a fluent **Builder Pattern** API.

## Features

- 🏗️ **Builder Pattern** - Fluent API with `ScenarioBuilder` for defining test scenarios
- 🔄 **Auto-shutdown** - Server automatically shuts down when all handlers have been called
- ⚡ **Static & Dynamic Handlers** - Support for predefined responses and dynamic responses based on request context
- 📝 **Request Collection** - Collect all incoming requests for assertions
- 🌐 **Tonic-compatible** - Works with standard gRPC clients over HTTP/2
- 🧱 **Clean Architecture** - Proper separation of entities, use cases, and adapters

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

## Architecture

```
src/
├── entities/           # Service, Method, Handler, Message, Scenario
├── use_cases/          # ScenarioBuilder, ports (Server, Collector traits)
├── adapters/gateways/  # Tonic server implementation
└── lib.rs              # Public API and prelude
```

## License

MIT - see [LICENSE](../../LICENSE) for details.

