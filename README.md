# Server Harness

A collection of Rust libraries for creating mock servers for testing purposes. These libraries follow **Clean Architecture** principles and provide a fluent **Builder Pattern** API for defining test scenarios.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Crates

| Crate | Description | crates.io |
|-------|-------------|-----------|
| [http-endpoint-server-harness](./crates/http-endpoint-server-harness) | Mock HTTP servers with Axum | [![crates.io](https://img.shields.io/crates/v/http-endpoint-server-harness.svg)](https://crates.io/crates/http-endpoint-server-harness) |
| [grpc-rpc-server-harness](./crates/grpc-rpc-server-harness) | Mock gRPC servers with Tonic | [![crates.io](https://img.shields.io/crates/v/grpc-rpc-server-harness.svg)](https://crates.io/crates/grpc-rpc-server-harness) |
| [graphql-operation-server-harness](./crates/graphql-operation-server-harness) | Mock GraphQL servers with async-graphql | [![crates.io](https://img.shields.io/crates/v/graphql-operation-server-harness.svg)](https://crates.io/crates/graphql-operation-server-harness) |

## Features

- 🏗️ **Builder Pattern** - Fluent API with `ScenarioBuilder` for defining test scenarios
- 🔄 **Auto-shutdown** - Servers automatically shut down when all handlers have been called
- 📝 **Request Collection** - Collect all incoming requests for assertions
- ⚡ **Static & Dynamic Handlers** - Support for predefined responses and dynamic responses based on request context
- 🧱 **Clean Architecture** - Separation of concerns between entities, use cases, and adapters

## Quick Start

### HTTP

```rust
use http_endpoint_server_harness::prelude::*;

let collected = ScenarioBuilder::new()
    .server(Axum::bind("127.0.0.1:3000".parse().unwrap()))
    .collector(DefaultCollector::new())
    .endpoint(
        Endpoint::new("/api/users", Method::Get)
            .with_handler(Handler::from_json(&json!({"users": []})))
    )
    .build()
    .execute()
    .await?;
```

### gRPC

```rust
use grpc_rpc_server_harness::prelude::*;

let collected = ScenarioBuilder::new()
    .server(Tonic::bind("127.0.0.1:50051".parse().unwrap()))
    .collector(DefaultCollector::new())
    .service(
        Service::new("my.package.UserService")
            .with_method(Method::new("GetUser").with_handler(Handler::from_bytes(vec![])))
    )
    .build()
    .execute()
    .await?;
```

### GraphQL

```rust
use graphql_operation_server_harness::prelude::*;

let collected = ScenarioBuilder::new()
    .server(AsyncGraphQL::bind("127.0.0.1:8080".parse().unwrap()))
    .collector(DefaultCollector::new())
    .operation(
        Operation::query()
            .with_field(Field::new("users").with_handler(Handler::new(json!([]))))
    )
    .build()
    .execute()
    .await?;
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
