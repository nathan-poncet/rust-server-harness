# http-endpoint-server-harness

Mock HTTP servers for integration tests. Spin up a real server with predefined responses that automatically shuts down when all expected requests are handled.

## Installation

```toml
[dev-dependencies]
http-endpoint-server-harness = "0.1"
tokio = { version = "1", features = ["full"] }
```

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

assert_eq!(collected.len(), 1);
assert_eq!(collected[0].path, "/api/users");
```

## Real-World Scenarios

### Polling Service Testing

Test a component that periodically calls a server (e.g., health checks every second):

```rust
#[tokio::test]
async fn test_polling_component() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    // Component polls /status every second until "ready"
    let component = spawn_my_polling_component(addr);

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/status", Method::Get)
                .with_handler(Handler::from_json(&json!({"state": "starting"})))
                .with_handler(Handler::from_json(&json!({"state": "starting"})))
                .with_handler(Handler::from_json(&json!({"state": "ready"})))
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Verify component polled exactly 3 times
    assert_eq!(collected.len(), 3);

    // Verify component stopped after receiving "ready"
    assert!(component.is_finished());
}
```

### Retry Logic Testing

Validate your retry mechanism handles transient failures:

```rust
#[tokio::test]
async fn test_retry_on_failure() {
    let addr: SocketAddr = "127.0.0.1:3001".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/data", Method::Get)
                // Fail twice, then succeed
                .with_handler(Handler::new(Response::internal_error()))
                .with_handler(Handler::new(Response::new(503)))
                .with_handler(Handler::from_json(&json!({"data": "success"})))
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Client should have retried 3 times
    assert_eq!(collected.len(), 3);
}
```

### Webhook Delivery Verification

Ensure your system sends webhooks with correct payloads:

```rust
#[tokio::test]
async fn test_webhook_payload() {
    let addr: SocketAddr = "127.0.0.1:3002".parse().unwrap();

    // Trigger the action that sends a webhook
    trigger_payment_completion(addr);

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/webhook", Method::Post)
                .with_handler(Handler::from_json(&json!({"ok": true})))
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Verify webhook content
    let webhook = &collected[0];
    let body: serde_json::Value = serde_json::from_slice(&webhook.body).unwrap();

    assert_eq!(webhook.headers.get("Content-Type"), Some(&"application/json".into()));
    assert_eq!(body["event"], "payment.completed");
    assert!(body["signature"].is_string());
}
```

### API Gateway / BFF Testing

Test a service that aggregates multiple downstream APIs:

```rust
#[tokio::test]
async fn test_bff_aggregation() {
    let addr: SocketAddr = "127.0.0.1:3003".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/users/1", Method::Get)
                .with_handler(Handler::from_json(&json!({"id": 1, "name": "Alice"})))
        )
        .endpoint(
            Endpoint::new("/users/1/orders", Method::Get)
                .with_handler(Handler::from_json(&json!([{"id": 100}])))
        )
        .endpoint(
            Endpoint::new("/users/1/preferences", Method::Get)
                .with_handler(Handler::from_json(&json!({"theme": "dark"})))
        )
        .build()
        .execute()
        .await
        .unwrap();

    // BFF should call all 3 endpoints
    assert_eq!(collected.len(), 3);
}
```

### Rate Limiting Client Testing

Verify your client handles 429 responses:

```rust
#[tokio::test]
async fn test_rate_limit_handling() {
    let addr: SocketAddr = "127.0.0.1:3004".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/resource", Method::Get)
                .with_handler(Handler::new(
                    Response::new(429)
                        .with_header("Retry-After", "1")
                        .with_json(&json!({"error": "rate limited"}))
                ))
                .with_handler(Handler::from_json(&json!({"data": "ok"})))
        )
        .build()
        .execute()
        .await
        .unwrap();

    assert_eq!(collected.len(), 2);
}
```

## Common Patterns

### Sequential Responses

Each call gets the next handler:

```rust
Endpoint::new("/api/counter", Method::Get)
    .with_handler(Handler::from_json(&json!({"n": 1})))  // 1st
    .with_handler(Handler::from_json(&json!({"n": 2})))  // 2nd
    .with_handler(Handler::from_json(&json!({"n": 3}))) // 3rd
```

### Dynamic Responses

Build responses based on request content:

```rust
Endpoint::new("/api/greet", Method::Post)
    .with_handler(Handler::dynamic(|req| {
        let body: serde_json::Value = serde_json::from_slice(&req.body)
            .unwrap_or(json!({}));
        let name = body["name"].as_str().unwrap_or("World");
        Response::ok().with_json(&json!({"message": format!("Hello, {}!", name)}))
    }))
```

### Error Simulation

```rust
Handler::new(Response::not_found())                    // 404
Handler::new(Response::internal_error())               // 500
Handler::new(Response::new(503))                       // 503
Handler::new(Response::new(429)
    .with_header("Retry-After", "60"))                 // 429 with header
```

### Path Parameters

Match dynamic segments:

```rust
Endpoint::new("/users/{id}/posts/{post_id}", Method::Get)
    .with_handler(Handler::from_json(&json!({"title": "Hello"})))
```

### Request Assertions

```rust
let collected = scenario.execute().await?;

// Count
assert_eq!(collected.len(), 2);

// Method & path
assert_eq!(collected[0].method, Method::Post);
assert_eq!(collected[0].path, "/api/users");

// Headers
assert!(collected[0].headers.get("Authorization").unwrap().starts_with("Bearer "));

// Body
let body: CreateUserRequest = serde_json::from_slice(&collected[0].body)?;
assert_eq!(body.email, "alice@example.com");
```

### Custom Headers

```rust
Handler::from_json(&json!({"data": "ok"}))
    .with_header("X-Request-Id", "abc123")
    .with_header("Cache-Control", "no-store")
```

## License

MIT - see [LICENSE](../../LICENSE) for details.
