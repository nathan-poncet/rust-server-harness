# Server Harness

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Server Harness** is a collection of Rust libraries for creating **mock servers** in integration tests. Spin up a real server with predefined responses that automatically shuts down when all expected requests are handled.

## Crates

| Crate | Description |
|-------|-------------|
| [http-endpoint-server-harness](./crates/http-endpoint-server-harness) | Mock HTTP/REST servers with Axum |
| [grpc-rpc-server-harness](./crates/grpc-rpc-server-harness) | Mock gRPC servers with Tonic |
| [graphql-operation-server-harness](./crates/graphql-operation-server-harness) | Mock GraphQL servers with async-graphql |

## Quick Start

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

// Assert on collected requests
assert_eq!(collected.len(), 1);
assert_eq!(collected[0].path, "/api/users");
```

## Real-World Scenarios

### 1. Polling Service Testing

Test components that periodically call an external server (e.g., every second for status updates):

```rust
// Your component polls a server every second for status updates.
// Test that it handles the full lifecycle correctly.

let collected = ScenarioBuilder::new()
    .server(Axum::bind(addr))
    .collector(DefaultCollector::new())
    .endpoint(
        Endpoint::new("/api/status", Method::Get)
            // First 2 calls: service starting up
            .with_handler(Handler::from_json(&json!({"status": "starting"})))
            .with_handler(Handler::from_json(&json!({"status": "starting"})))
            // Next call: service ready
            .with_handler(Handler::from_json(&json!({"status": "ready"})))
    )
    .build()
    .execute()
    .await?;

// Verify your component made exactly 3 requests before reacting to "ready"
assert_eq!(collected.len(), 3);
```

### 2. Retry & Circuit Breaker Testing

Validate resilience logic by simulating transient failures:

```rust
let collected = ScenarioBuilder::new()
    .server(Axum::bind(addr))
    .collector(DefaultCollector::new())
    .endpoint(
        Endpoint::new("/api/data", Method::Get)
            // First 2 calls fail
            .with_handler(Handler::new(Response::internal_error()))
            .with_handler(Handler::new(Response::internal_error()))
            // Third call succeeds
            .with_handler(Handler::from_json(&json!({"data": "success"})))
    )
    .build()
    .execute()
    .await?;

// Verify retry logic worked
assert_eq!(collected.len(), 3);
```

### 3. Webhook Verification

Ensure your code sends webhooks correctly:

```rust
let collected = ScenarioBuilder::new()
    .server(Axum::bind(addr))
    .collector(DefaultCollector::new())
    .endpoint(
        Endpoint::new("/webhook/payment", Method::Post)
            .with_handler(Handler::from_json(&json!({"received": true})))
    )
    .build()
    .execute()
    .await?;

// Verify webhook payload
let webhook = &collected[0];
let body: serde_json::Value = serde_json::from_slice(&webhook.body)?;
assert_eq!(body["event"], "payment.completed");
assert_eq!(body["amount"], 100);
```

### 4. OAuth Token Refresh

Test authentication flows:

```rust
let collected = ScenarioBuilder::new()
    .server(Axum::bind(addr))
    .collector(DefaultCollector::new())
    // Token endpoint
    .endpoint(
        Endpoint::new("/oauth/token", Method::Post)
            .with_handler(Handler::from_json(&json!({
                "access_token": "token_v1",
                "expires_in": 1
            })))
            .with_handler(Handler::from_json(&json!({
                "access_token": "token_v2",
                "expires_in": 3600
            })))
    )
    // API endpoint requiring auth
    .endpoint(
        Endpoint::new("/api/data", Method::Get)
            .with_handler(Handler::from_json(&json!({"data": "protected"})))
    )
    .build()
    .execute()
    .await?;

// Verify token refresh happened
assert_eq!(collected.iter().filter(|r| r.path == "/oauth/token").count(), 2);
```

### 5. Microservice Dependency Testing

Mock downstream services in integration tests:

```rust
// Your service calls multiple downstream services
let collected = ScenarioBuilder::new()
    .server(Axum::bind(addr))
    .collector(DefaultCollector::new())
    .endpoint(
        Endpoint::new("/users/{id}", Method::Get)
            .with_handler(Handler::from_json(&json!({"id": 1, "name": "Alice"})))
    )
    .endpoint(
        Endpoint::new("/orders", Method::Get)
            .with_handler(Handler::from_json(&json!([{"id": 100, "user_id": 1}])))
    )
    .build()
    .execute()
    .await?;
```

## Common Patterns

### Sequential Responses

Return different responses for successive calls to the same endpoint:

```rust
Endpoint::new("/api/resource", Method::Get)
    .with_handler(Handler::from_json(&json!({"v": 1})))  // 1st call
    .with_handler(Handler::from_json(&json!({"v": 2})))  // 2nd call
    .with_handler(Handler::from_json(&json!({"v": 3}))) // 3rd call
```

### Dynamic Responses

Build responses based on request content:

```rust
Endpoint::new("/api/echo", Method::Post)
    .with_handler(Handler::dynamic(|req| {
        let body = req.body_as_str().unwrap_or("{}");
        Response::ok().with_json(&json!({"echoed": body}))
    }))
```

### Error Simulation

Test error handling paths:

```rust
// HTTP errors
Handler::new(Response::not_found())
Handler::new(Response::internal_error())
Handler::new(Response::new(429).with_json(&json!({"error": "rate limited"})))

// Custom error with headers
Handler::new(
    Response::new(503)
        .with_header("Retry-After", "30")
        .with_json(&json!({"error": "service unavailable"}))
)
```

### Request Validation

Assert on collected requests:

```rust
let collected = scenario.execute().await?;

// Verify request count
assert_eq!(collected.len(), 3);

// Verify specific request
assert_eq!(collected[0].method, Method::Post);
assert_eq!(collected[0].path, "/api/users");

// Verify headers
assert_eq!(collected[0].headers.get("Authorization"), Some(&"Bearer token".to_string()));

// Verify body
let body: MyRequest = serde_json::from_slice(&collected[0].body)?;
assert_eq!(body.name, "Alice");
```

### Path Parameters

Match dynamic path segments:

```rust
Endpoint::new("/users/{id}/orders/{order_id}", Method::Get)
    .with_handler(Handler::from_json(&json!({"status": "shipped"})))
```

## License

MIT - see [LICENSE](LICENSE) for details.
