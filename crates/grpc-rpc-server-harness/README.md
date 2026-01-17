# grpc-rpc-server-harness

Mock gRPC servers for integration tests. Spin up a real gRPC server with predefined responses that automatically shuts down when all expected requests are handled.

## Installation

```toml
[dev-dependencies]
grpc-rpc-server-harness = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use grpc_rpc_server_harness::prelude::*;

let collected = ScenarioBuilder::new()
    .server(Tonic::bind("127.0.0.1:50051".parse().unwrap()))
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

assert_eq!(collected[0].service, "my.package.UserService");
assert_eq!(collected[0].method, "GetUser");
```

## Real-World Scenarios

### Polling Service Testing

Test a component that periodically calls a gRPC service for status updates:

```rust
#[tokio::test]
async fn test_status_polling() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    // Component polls StatusService.GetStatus every second
    let component = spawn_my_grpc_poller(addr);

    let collected = ScenarioBuilder::new()
        .server(Tonic::bind(addr))
        .collector(DefaultCollector::new())
        .service(
            Service::new("monitoring.StatusService")
                .with_method(
                    Method::new("GetStatus")
                        // Returns "initializing" twice, then "ready"
                        .with_handler(Handler::from_prost(&StatusResponse { state: "initializing".into() }))
                        .with_handler(Handler::from_prost(&StatusResponse { state: "initializing".into() }))
                        .with_handler(Handler::from_prost(&StatusResponse { state: "ready".into() }))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Verify 3 polling calls
    assert_eq!(collected.len(), 3);
    assert!(component.is_ready());
}
```

### Retry Logic Testing

Validate gRPC retry mechanism with transient failures:

```rust
#[tokio::test]
async fn test_grpc_retry() {
    let addr: SocketAddr = "127.0.0.1:50052".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(Tonic::bind(addr))
        .collector(DefaultCollector::new())
        .service(
            Service::new("data.DataService")
                .with_method(
                    Method::new("FetchData")
                        // First 2 calls return error
                        .with_handler(Handler::from_error(tonic::Code::Unavailable, "service down"))
                        .with_handler(Handler::from_error(tonic::Code::Unavailable, "still down"))
                        // Third call succeeds
                        .with_handler(Handler::from_prost(&DataResponse { value: 42 }))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Client should have retried 3 times
    assert_eq!(collected.len(), 3);
}
```

### Microservice Dependency Mocking

Test a service that calls multiple downstream gRPC services:

```rust
#[tokio::test]
async fn test_order_service_aggregation() {
    let addr: SocketAddr = "127.0.0.1:50053".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(Tonic::bind(addr))
        .collector(DefaultCollector::new())
        .service(
            Service::new("users.UserService")
                .with_method(
                    Method::new("GetUser")
                        .with_handler(Handler::from_prost(&User { id: 1, name: "Alice".into() }))
                )
        )
        .service(
            Service::new("inventory.InventoryService")
                .with_method(
                    Method::new("CheckStock")
                        .with_handler(Handler::from_prost(&StockResponse { available: true }))
                )
        )
        .service(
            Service::new("pricing.PricingService")
                .with_method(
                    Method::new("GetPrice")
                        .with_handler(Handler::from_prost(&PriceResponse { amount: 99_99 }))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Order service should call all 3 downstream services
    assert_eq!(collected.len(), 3);
}
```

### Streaming Simulation

Test bidirectional streaming with sequential responses:

```rust
#[tokio::test]
async fn test_streaming_updates() {
    let addr: SocketAddr = "127.0.0.1:50054".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(Tonic::bind(addr))
        .collector(DefaultCollector::new())
        .service(
            Service::new("updates.UpdateService")
                .with_method(
                    Method::new("Subscribe")
                        .with_handler(Handler::from_prost(&Update { version: 1 }))
                        .with_handler(Handler::from_prost(&Update { version: 2 }))
                        .with_handler(Handler::from_prost(&Update { version: 3 }))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    assert_eq!(collected.len(), 3);
}
```

### Authentication Flow Testing

Test token refresh with gRPC metadata:

```rust
#[tokio::test]
async fn test_auth_token_refresh() {
    let addr: SocketAddr = "127.0.0.1:50055".parse().unwrap();

    let collected = ScenarioBuilder::new()
        .server(Tonic::bind(addr))
        .collector(DefaultCollector::new())
        .service(
            Service::new("auth.AuthService")
                .with_method(
                    Method::new("RefreshToken")
                        .with_handler(Handler::from_prost(&TokenResponse {
                            token: "new_token".into(),
                            expires_in: 3600
                        }))
                )
        )
        .service(
            Service::new("api.ApiService")
                .with_method(
                    Method::new("SecureCall")
                        .with_handler(Handler::from_prost(&ApiResponse { data: "secret".into() }))
                )
        )
        .build()
        .execute()
        .await
        .unwrap();

    // Verify token refresh happened before API call
    assert_eq!(collected[0].method, "RefreshToken");
    assert_eq!(collected[1].method, "SecureCall");
}
```

## Common Patterns

### Sequential Responses

Each call gets the next handler:

```rust
Method::new("GetStatus")
    .with_handler(Handler::from_prost(&Status { code: 1 }))  // 1st call
    .with_handler(Handler::from_prost(&Status { code: 2 }))  // 2nd call
    .with_handler(Handler::from_prost(&Status { code: 3 })) // 3rd call
```

### Dynamic Responses

Build responses based on request content:

```rust
Method::new("Echo")
    .with_handler(Handler::dynamic(|ctx| {
        let mut response = vec![0xFF]; // prefix
        response.extend_from_slice(&ctx.message.data);
        Message::new(response)
    }))
```

### With Prost Messages

Serialize protobuf messages directly:

```rust
use prost::Message as ProstMessage;

#[derive(ProstMessage)]
struct GetUserResponse {
    #[prost(string, tag = "1")]
    name: String,
}

Handler::from_prost(&GetUserResponse { name: "Alice".into() })
```

### Error Simulation

```rust
Handler::from_error(tonic::Code::NotFound, "user not found")
Handler::from_error(tonic::Code::Unavailable, "service down")
Handler::from_error(tonic::Code::Internal, "internal error")
Handler::from_error(tonic::Code::PermissionDenied, "unauthorized")
```

### Multiple Services

```rust
ScenarioBuilder::new()
    .server(Tonic::bind(addr))
    .collector(DefaultCollector::new())
    .service(
        Service::new("users.UserService")
            .with_method(Method::new("GetUser").with_handler(...))
            .with_method(Method::new("CreateUser").with_handler(...))
    )
    .service(
        Service::new("orders.OrderService")
            .with_method(Method::new("GetOrder").with_handler(...))
    )
    .build()
```

### Request Assertions

```rust
let collected = scenario.execute().await?;

// Count
assert_eq!(collected.len(), 2);

// Service & method
assert_eq!(collected[0].service, "users.UserService");
assert_eq!(collected[0].method, "GetUser");

// Decode request message
let request: GetUserRequest = prost::Message::decode(&collected[0].message.data[..])?;
assert_eq!(request.user_id, 123);
```

## License

MIT - see [LICENSE](../../LICENSE) for details.
