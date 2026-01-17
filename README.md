# Server Harness

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Server Harness** is a collection of Rust libraries for creating **mock servers** in your integration tests. Instead of mocking HTTP clients or using complex test doubles, you spin up a real server that behaves exactly as you define it.

## 🎯 The Problem

When testing code that makes HTTP, gRPC, or GraphQL calls to external services, you typically have two options:

1. **Mock the client** - Replace your HTTP/gRPC client with a mock. This doesn't test the actual network layer and can miss serialization bugs.
2. **Use a shared test server** - Requires infrastructure setup, can cause flaky tests due to shared state, and is hard to customize per test.

## ✅ The Solution

**Server Harness** gives you a third option: **spin up a real, lightweight mock server for each test**.

- **Real network calls** - Your code makes actual HTTP/gRPC/GraphQL requests, testing the full stack
- **Isolated per test** - Each test gets its own server instance with its own configuration
- **Declarative scenarios** - Define endpoints, expected responses, and the server handles the rest
- **Auto-shutdown** - The server automatically stops when all expected requests have been handled
- **Request capture** - Inspect what requests were made for assertions

## 📦 Use Cases

- **Testing API clients** - Verify your REST/gRPC/GraphQL client code works correctly
- **Integration testing** - Test your application's behavior when external services return specific responses
- **Error handling** - Simulate error responses (500s, timeouts, malformed data) to test resilience
- **Contract testing** - Ensure your code handles the expected API contract
- **E2E testing** - Use as a mock backend for end-to-end tests

## Crates

| Crate | Description | crates.io |
|-------|-------------|-----------|
| [http-endpoint-server-harness](./crates/http-endpoint-server-harness) | Mock HTTP/REST servers with Axum | [![crates.io](https://img.shields.io/crates/v/http-endpoint-server-harness.svg)](https://crates.io/crates/http-endpoint-server-harness) |
| [grpc-rpc-server-harness](./crates/grpc-rpc-server-harness) | Mock gRPC servers with Tonic | [![crates.io](https://img.shields.io/crates/v/grpc-rpc-server-harness.svg)](https://crates.io/crates/grpc-rpc-server-harness) |
| [graphql-operation-server-harness](./crates/graphql-operation-server-harness) | Mock GraphQL servers with async-graphql | [![crates.io](https://img.shields.io/crates/v/graphql-operation-server-harness.svg)](https://crates.io/crates/graphql-operation-server-harness) |

## ✨ Features

- 🏗️ **Builder Pattern** - Fluent API with `ScenarioBuilder` for defining test scenarios
- 🔄 **Auto-shutdown** - Servers automatically shut down when all handlers have been called
- 📝 **Request Collection** - Collect all incoming requests for assertions
- ⚡ **Static & Dynamic Handlers** - Predefined responses or dynamic responses based on request content
- 🔁 **Sequential Handlers** - Define different responses for successive calls to the same endpoint
- 🧱 **Clean Architecture** - Extensible design with pluggable server backends and collectors

## 🚀 Quick Start

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

## 🔧 How It Works

1. **Define a scenario** - Specify which endpoints/services/operations your mock server should expose
2. **Attach handlers** - For each endpoint, define what response to return (static JSON, dynamic response, error, etc.)
3. **Execute** - The server starts, waits for all expected requests, then shuts down automatically
4. **Assert** - Inspect the collected requests to verify your code made the right calls

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Your Code     │────▶│   Mock Server    │────▶│   Assertions    │
│  (HTTP Client)  │     │  (Server Harness)│     │ (Collected Req) │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                        │                        │
        │   Real HTTP Request    │   Auto-shutdown        │
        │───────────────────────▶│   after all handlers   │
        │                        │   are consumed         │
        │◀───────────────────────│                        │
        │   Configured Response  │                        │
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
