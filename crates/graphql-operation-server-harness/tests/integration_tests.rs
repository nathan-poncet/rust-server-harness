//! Integration tests for graphql-operation-server-harness

use graphql_operation_server_harness::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Notify;

/// Helper to run a server and execute requests against it
async fn run_with_requests<F, Fut>(
    operations: Vec<Operation>,
    make_requests: F,
) -> Vec<CollectedRequest>
where
    F: FnOnce(SocketAddr) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    let ready = Arc::new(Notify::new());
    let ready_clone = ready.clone();
    let addr_holder = Arc::new(std::sync::Mutex::new(None));
    let addr_holder_clone = addr_holder.clone();

    let server = AsyncGraphQL::default();
    let collector = DefaultCollector::new();

    let server_task = tokio::spawn(async move {
        server
            .run(
                operations,
                collector,
                Some(move |addr| {
                    *addr_holder_clone.lock().unwrap() = Some(addr);
                    ready_clone.notify_one();
                }),
            )
            .await
    });

    ready.notified().await;
    let addr = addr_holder.lock().unwrap().unwrap();

    make_requests(addr).await;

    server_task.await.unwrap().unwrap()
}

#[tokio::test]
async fn test_single_query_field() {
    let result = run_with_requests(
        vec![Operation::query().with_field(
            Field::new("users").with_handler(Handler::new(json!({
                "users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]
            }))),
        )],
        |addr| async move {
            let client = reqwest::Client::new();
            let response = client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({
                    "query": "query { users { id name } }"
                }))
                .send()
                .await
                .unwrap();

            assert_eq!(response.status(), 200);
            let body: serde_json::Value = response.json().await.unwrap();
            assert!(body["data"]["users"].is_array());
            assert_eq!(body["data"]["users"][0]["name"], "Alice");
        },
    )
    .await;

    assert_eq!(result.len(), 1);
    assert!(result[0].query.contains("users"));
}

#[tokio::test]
async fn test_mutation() {
    let result = run_with_requests(
        vec![Operation::mutation().with_field(
            Field::new("createUser").with_handler(Handler::new(json!({
                "createUser": {"id": 100, "name": "NewUser"}
            }))),
        )],
        |addr| async move {
            let client = reqwest::Client::new();
            let response = client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({
                    "query": "mutation { createUser(name: \"NewUser\") { id name } }"
                }))
                .send()
                .await
                .unwrap();

            assert_eq!(response.status(), 200);
            let body: serde_json::Value = response.json().await.unwrap();
            assert_eq!(body["data"]["createUser"]["id"], 100);
        },
    )
    .await;

    assert_eq!(result.len(), 1);
}

#[tokio::test]
async fn test_multiple_fields() {
    let result = run_with_requests(
        vec![Operation::query()
            .with_field(Field::new("users").with_handler(Handler::new(json!({
                "users": [{"id": 1}]
            }))))
            .with_field(Field::new("posts").with_handler(Handler::new(json!({
                "posts": [{"id": 100}]
            }))))],
        |addr| async move {
            let client = reqwest::Client::new();

            // Query users
            let response1 = client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({"query": "query { users { id } }"}))
                .send()
                .await
                .unwrap();
            let body1: serde_json::Value = response1.json().await.unwrap();
            assert!(body1["data"]["users"].is_array());

            // Query posts
            let response2 = client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({"query": "query { posts { id } }"}))
                .send()
                .await
                .unwrap();
            let body2: serde_json::Value = response2.json().await.unwrap();
            assert!(body2["data"]["posts"].is_array());
        },
    )
    .await;

    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_sequential_handlers() {
    let result = run_with_requests(
        vec![Operation::query().with_field(
            Field::new("counter")
                .with_handler(Handler::new(json!({"counter": 1})))
                .with_handler(Handler::new(json!({"counter": 2})))
                .with_handler(Handler::new(json!({"counter": 3}))),
        )],
        |addr| async move {
            let client = reqwest::Client::new();

            // Test sequential handlers - each call uses the next handler in sequence
            for expected in [1, 2, 3] {
                let response = client
                    .post(format!("http://{}/graphql", addr))
                    .json(&json!({"query": "query { counter }"}))
                    .send()
                    .await
                    .unwrap();
                let body: serde_json::Value = response.json().await.unwrap();
                assert_eq!(body["data"]["counter"], expected);
            }
        },
    )
    .await;

    assert_eq!(result.len(), 3);
}

#[tokio::test]
async fn test_operation_name_captured() {
    let result = run_with_requests(
        vec![Operation::query().with_field(
            Field::new("user").with_handler(Handler::new(json!({"user": {"id": 1}}))),
        )],
        |addr| async move {
            let client = reqwest::Client::new();
            client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({
                    "query": "query GetUser { user { id } }",
                    "operationName": "GetUser"
                }))
                .send()
                .await
                .unwrap();
        },
    )
    .await;

    assert_eq!(result[0].operation_name, Some("GetUser".to_string()));
}

#[tokio::test]
async fn test_variables_captured() {
    let result = run_with_requests(
        vec![Operation::query().with_field(
            Field::new("user").with_handler(Handler::new(json!({"user": {"id": 1}}))),
        )],
        |addr| async move {
            let client = reqwest::Client::new();
            client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({
                    "query": "query GetUser($id: ID!) { user(id: $id) { id } }",
                    "variables": {"id": "123"}
                }))
                .send()
                .await
                .unwrap();
        },
    )
    .await;

    assert_eq!(result[0].variables, Some(json!({"id": "123"})));
}

#[tokio::test]
async fn test_invalid_json_request() {
    // For invalid JSON, we need a custom test since run_with_requests expects the handler to be called
    let ready = Arc::new(Notify::new());
    let ready_clone = ready.clone();
    let addr_holder = Arc::new(std::sync::Mutex::new(None));
    let addr_holder_clone = addr_holder.clone();

    let server = AsyncGraphQL::default();
    let collector = DefaultCollector::new();

    let _server_task = tokio::spawn(async move {
        server
            .run(
                vec![Operation::query().with_field(
                    Field::new("test").with_handler(Handler::new(json!({"test": true}))),
                )],
                collector,
                Some(move |addr| {
                    *addr_holder_clone.lock().unwrap() = Some(addr);
                    ready_clone.notify_one();
                }),
            )
            .await
    });

    ready.notified().await;
    let addr = addr_holder.lock().unwrap().unwrap();

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/graphql", addr))
        .header("content-type", "application/json")
        .body("not valid json")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["errors"].is_array());

    // Note: Server will keep running since the handler wasn't called, but that's ok for this test
}

#[tokio::test]
async fn test_dynamic_handler_echo_variables() {
    run_with_requests(
        vec![Operation::query().with_field(
            Field::new("user").with_handler(Handler::dynamic(|ctx| {
                let id = ctx
                    .get_variable("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                HandlerResponse::new(json!({
                    "user": {"id": id, "name": format!("User {}", id)}
                }))
            })),
        )],
        |addr| async move {
            let client = reqwest::Client::new();
            let response: serde_json::Value = client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({
                    "query": "query GetUser($id: ID!) { user(id: $id) { id name } }",
                    "variables": {"id": "42"}
                }))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            assert_eq!(response["data"]["user"]["id"], "42");
            assert_eq!(response["data"]["user"]["name"], "User 42");
        },
    )
    .await;
}

#[tokio::test]
async fn test_dynamic_handler_conditional_response() {
    run_with_requests(
        vec![Operation::query().with_field(
            Field::new("items").with_handler(Handler::dynamic(|ctx| {
                let limit = ctx
                    .get_variable("limit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(10) as usize;

                let items: Vec<serde_json::Value> =
                    (1..=limit).map(|i| json!({"id": i})).collect();

                HandlerResponse::new(json!({"items": items}))
            })),
        )],
        |addr| async move {
            let client = reqwest::Client::new();

            // Request with limit 3
            let response: serde_json::Value = client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({
                    "query": "query ($limit: Int) { items(limit: $limit) { id } }",
                    "variables": {"limit": 3}
                }))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            assert_eq!(response["data"]["items"].as_array().unwrap().len(), 3);
        },
    )
    .await;
}

#[tokio::test]
async fn test_dynamic_handler_with_error() {
    // Create a dynamic handler that validates input
    let dynamic_handler = Handler::dynamic(|ctx| {
        let name = ctx.get_variable("name").and_then(|v| v.as_str());

        match name {
            Some(n) if !n.is_empty() => {
                HandlerResponse::new(json!({"createUser": {"id": 1, "name": n}}))
            }
            _ => HandlerResponse::new(json!({"createUser": null})).with_error("Name is required"),
        }
    });

    run_with_requests(
        vec![Operation::mutation().with_field(
            Field::new("createUser")
                // Add two handlers since we make two calls (server auto-shuts down after all handlers are called)
                .with_handler(dynamic_handler.clone())
                .with_handler(dynamic_handler),
        )],
        |addr| async move {
            let client = reqwest::Client::new();

            // Valid request
            let response1: serde_json::Value = client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({
                    "query": "mutation ($name: String!) { createUser(name: $name) { id name } }",
                    "variables": {"name": "Alice"}
                }))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            assert_eq!(response1["data"]["createUser"]["name"], "Alice");
            assert!(response1.get("errors").is_none());

            // Invalid request (empty name)
            let response2: serde_json::Value = client
                .post(format!("http://{}/graphql", addr))
                .json(&json!({
                    "query": "mutation ($name: String!) { createUser(name: $name) { id name } }",
                    "variables": {"name": ""}
                }))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            assert!(response2["errors"].is_array());
        },
    )
    .await;
}

#[tokio::test]
async fn test_auto_shutdown_after_all_handlers() {
    // Test that the server automatically shuts down after all handlers are called
    let ready = Arc::new(Notify::new());
    let ready_clone = ready.clone();
    let addr_holder = Arc::new(std::sync::Mutex::new(None));
    let addr_holder_clone = addr_holder.clone();

    let server = AsyncGraphQL::default();
    let collector = DefaultCollector::new();

    let server_task = tokio::spawn(async move {
        server
            .run(
                vec![Operation::query()
                    .with_field(
                        Field::new("users").with_handler(Handler::new(json!({"users": []}))),
                    )
                    .with_field(
                        Field::new("posts").with_handler(Handler::new(json!({"posts": []}))),
                    )],
                collector,
                Some(move |addr| {
                    *addr_holder_clone.lock().unwrap() = Some(addr);
                    ready_clone.notify_one();
                }),
            )
            .await
    });

    ready.notified().await;
    let addr = addr_holder.lock().unwrap().unwrap();

    let client = reqwest::Client::new();

    // Call users field
    let _ = client
        .post(format!("http://{}/graphql", addr))
        .json(&json!({"query": "query { users { id } }"}))
        .send()
        .await
        .unwrap();

    // Call posts field
    let _ = client
        .post(format!("http://{}/graphql", addr))
        .json(&json!({"query": "query { posts { id } }"}))
        .send()
        .await
        .unwrap();

    // Wait for server to complete
    let result = server_task.await.unwrap();
    assert!(result.is_ok(), "Server should have completed successfully");

    // Server should be down - verify by trying to connect
    let result = client
        .post(format!("http://{}/graphql", addr))
        .json(&json!({"query": "query { users { id } }"}))
        .timeout(std::time::Duration::from_millis(500))
        .send()
        .await;

    assert!(
        result.is_err(),
        "Server should have shut down automatically"
    );
}

#[tokio::test]
async fn test_auto_shutdown_with_sequential_handlers() {
    let ready = Arc::new(Notify::new());
    let ready_clone = ready.clone();
    let addr_holder = Arc::new(std::sync::Mutex::new(None));
    let addr_holder_clone = addr_holder.clone();

    let server = AsyncGraphQL::default();
    let collector = DefaultCollector::new();

    let server_task = tokio::spawn(async move {
        server
            .run(
                vec![Operation::query().with_field(
                    Field::new("counter")
                        .with_handler(Handler::new(json!({"counter": 1})))
                        .with_handler(Handler::new(json!({"counter": 2}))),
                )],
                collector,
                Some(move |addr| {
                    *addr_holder_clone.lock().unwrap() = Some(addr);
                    ready_clone.notify_one();
                }),
            )
            .await
    });

    ready.notified().await;
    let addr = addr_holder.lock().unwrap().unwrap();

    let client = reqwest::Client::new();

    // Call twice to exhaust all handlers
    for _ in 0..2 {
        let _ = client
            .post(format!("http://{}/graphql", addr))
            .json(&json!({"query": "query { counter }"}))
            .send()
            .await
            .unwrap();
    }

    // Wait for server to complete
    let result = server_task.await.unwrap();
    assert!(result.is_ok(), "Server should have completed successfully");

    // Server should be down
    let result = client
        .post(format!("http://{}/graphql", addr))
        .json(&json!({"query": "query { counter }"}))
        .timeout(std::time::Duration::from_millis(500))
        .send()
        .await;

    assert!(
        result.is_err(),
        "Server should have shut down after all handlers were called"
    );
}
