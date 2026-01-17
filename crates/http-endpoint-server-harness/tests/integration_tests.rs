//! Integration tests for http-endpoint-server-harness

use http_endpoint_server_harness::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;

/// Helper to get an available port
fn get_available_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Helper to create an address with an available port
fn get_test_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], get_available_port()))
}

/// Helper to wait for server to be ready with retries
async fn wait_for_server(addr: SocketAddr) {
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client
            .get(format!("http://{}/", addr))
            .timeout(Duration::from_millis(50))
            .send()
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn test_single_endpoint_get() {
    let addr = get_test_addr();

    // Spawn task to make HTTP requests
    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/api/test", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["status"], "ok");
    });

    // Execute scenario
    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/test", Method::Get)
                .with_handler(Handler::from_json(&json!({"status": "ok"}))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();

    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].method, Method::Get);
    assert_eq!(collected[0].path, "/api/test");
}

#[tokio::test]
async fn test_single_endpoint_post() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}/api/data", addr))
            .json(&json!({"name": "test"}))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["created"], true);
    });

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/data", Method::Post)
                .with_handler(Handler::from_json(&json!({"created": true}))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();

    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].method, Method::Post);
    assert!(collected[0].body_as_str().unwrap().contains("test"));
}

#[tokio::test]
async fn test_multiple_handlers_sequential() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();

        // First request should get first handler
        let response1: serde_json::Value = client
            .post(format!("http://{}/api/token", addr))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(response1["token"], "first");

        // Second request should get second handler
        let response2: serde_json::Value = client
            .post(format!("http://{}/api/token", addr))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(response2["token"], "second");

        // Third request should get third handler
        let response3: serde_json::Value = client
            .post(format!("http://{}/api/token", addr))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(response3["token"], "third");
    });

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/token", Method::Post)
                .with_handler(Handler::from_json(&json!({"token": "first"})))
                .with_handler(Handler::from_json(&json!({"token": "second"})))
                .with_handler(Handler::from_json(&json!({"token": "third"}))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
    assert_eq!(collected.len(), 3);
}

#[tokio::test]
async fn test_multiple_endpoints() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();

        let users: serde_json::Value = client
            .get(format!("http://{}/api/users", addr))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(users[0]["id"], 1);

        let posts: serde_json::Value = client
            .get(format!("http://{}/api/posts", addr))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(posts[0]["id"], 100);
    });

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/users", Method::Get)
                .with_handler(Handler::from_json(&json!([{"id": 1}]))),
        )
        .endpoint(
            Endpoint::new("/api/posts", Method::Get)
                .with_handler(Handler::from_json(&json!([{"id": 100}]))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
    assert_eq!(collected.len(), 2);
}

#[tokio::test]
async fn test_custom_status_code() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/api/error", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 404);
    });

    ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/error", Method::Get)
                .with_handler(Handler::from_json(&json!({"error": "not found"})).with_status(404)),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
}

#[tokio::test]
async fn test_custom_headers() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/api/headers", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(
            response.headers().get("X-Custom-Header").unwrap(),
            "custom-value"
        );
        assert_eq!(
            response.headers().get("X-Another").unwrap(),
            "another-value"
        );
    });

    ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/headers", Method::Get).with_handler(
                Handler::from_json(&json!({}))
                    .with_header("X-Custom-Header", "custom-value")
                    .with_header("X-Another", "another-value"),
            ),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
}

#[tokio::test]
async fn test_put_method() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let response = client
            .put(format!("http://{}/api/resource", addr))
            .json(&json!({"name": "updated"}))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    });

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/resource", Method::Put)
                .with_handler(Handler::from_json(&json!({"updated": true}))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
    assert_eq!(collected[0].method, Method::Put);
}

#[tokio::test]
async fn test_delete_method() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let response = client
            .delete(format!("http://{}/api/resource/123", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 204);
    });

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/resource/{id}", Method::Delete)
                .with_handler(Handler::new(Response::new(204))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
    assert_eq!(collected[0].method, Method::Delete);
}

#[tokio::test]
async fn test_patch_method() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let response = client
            .patch(format!("http://{}/api/resource", addr))
            .json(&json!({"field": "value"}))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    });

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/resource", Method::Patch)
                .with_handler(Handler::from_json(&json!({"patched": true}))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
    assert_eq!(collected[0].method, Method::Patch);
}

#[tokio::test]
async fn test_collected_request_headers() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        client
            .get(format!("http://{}/api/test", addr))
            .header("Authorization", "Bearer token123")
            .header("X-Request-Id", "req-456")
            .send()
            .await
            .unwrap();
    });

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/test", Method::Get)
                .with_handler(Handler::from_json(&json!({}))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
    let req = &collected[0];
    assert_eq!(req.headers.get("authorization").unwrap(), "Bearer token123");
    assert_eq!(req.headers.get("x-request-id").unwrap(), "req-456");
}

#[tokio::test]
async fn test_concurrent_requests() {
    let addr = get_test_addr();

    // Create 10 handlers for 10 concurrent requests
    let mut endpoint = Endpoint::new("/api/concurrent", Method::Get);
    for _ in 0..10 {
        endpoint = endpoint.with_handler(Handler::from_json(&json!({"ok": true})));
    }

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let url = format!("http://{}/api/concurrent", addr);

        // Send 10 concurrent requests
        let futures: Vec<_> = (0..10)
            .map(|_| {
                let client = client.clone();
                let url = url.clone();
                async move { client.get(&url).send().await }
            })
            .collect();

        let responses = futures::future::join_all(futures).await;

        for response in responses {
            assert_eq!(response.unwrap().status(), 200);
        }
    });

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(endpoint)
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
    assert_eq!(collected.len(), 10);
}

#[tokio::test]
async fn test_dynamic_handler_echo_path() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let response: serde_json::Value = client
            .get(format!("http://{}/api/users/123", addr))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert!(response["path"].as_str().unwrap().contains("/api/"));
    });

    ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/{*path}", Method::Get).with_handler(Handler::dynamic(|req| {
                Response::new(200).with_json(&json!({
                    "path": req.path
                }))
            })),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
}

#[tokio::test]
async fn test_dynamic_handler_echo_body() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();
        let response: serde_json::Value = client
            .post(format!("http://{}/echo", addr))
            .json(&json!({"message": "hello"}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(response["received"]["message"], "hello");
    });

    ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/echo", Method::Post).with_handler(Handler::dynamic(|req| {
                if let Some(body) = req.body_as_str() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                        return Response::new(200).with_json(&json!({
                            "received": json
                        }));
                    }
                }
                Response::new(400).with_body("Invalid JSON")
            })),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
}

#[tokio::test]
async fn test_dynamic_handler_based_on_header() {
    let addr = get_test_addr();

    // Use the same dynamic handler twice to allow 2 requests
    let auth_handler = Handler::dynamic(|req| {
        if let Some(auth) = req.headers.get("authorization") {
            if auth.starts_with("Bearer ") {
                return Response::new(200).with_json(&json!({"authenticated": true}));
            }
        }
        Response::new(401).with_json(&json!({"authenticated": false}))
    });

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();

        // Without auth header
        let response1: serde_json::Value = client
            .get(format!("http://{}/auth", addr))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(response1["authenticated"], false);

        // With auth header
        let response2: serde_json::Value = client
            .get(format!("http://{}/auth", addr))
            .header("Authorization", "Bearer token123")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(response2["authenticated"], true);
    });

    ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/auth", Method::Get)
                .with_handler(auth_handler.clone())
                .with_handler(auth_handler),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
}

#[tokio::test]
async fn test_auto_shutdown_after_all_handlers() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();

        // Call first endpoint
        let resp1 = client
            .get(format!("http://{}/api/step1", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp1.status(), 200);

        // Call second endpoint
        let resp2 = client
            .get(format!("http://{}/api/step2", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp2.status(), 200);
    });

    // The scenario should complete automatically after all handlers are called
    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/step1", Method::Get)
                .with_handler(Handler::from_json(&json!({"step": 1}))),
        )
        .endpoint(
            Endpoint::new("/api/step2", Method::Get)
                .with_handler(Handler::from_json(&json!({"step": 2}))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
    assert_eq!(collected.len(), 2);
}

#[tokio::test]
async fn test_auto_shutdown_with_sequential_handlers() {
    let addr = get_test_addr();

    let requests_task = tokio::spawn(async move {
        wait_for_server(addr).await;

        let client = reqwest::Client::new();

        // Call 3 times to exhaust all handlers
        for expected in [1, 2, 3] {
            let resp: serde_json::Value = client
                .get(format!("http://{}/api/counter", addr))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            assert_eq!(resp["count"], expected);
        }
    });

    let collected = ScenarioBuilder::new()
        .server(Axum::bind(addr))
        .collector(DefaultCollector::new())
        .endpoint(
            Endpoint::new("/api/counter", Method::Get)
                .with_handler(Handler::from_json(&json!({"count": 1})))
                .with_handler(Handler::from_json(&json!({"count": 2})))
                .with_handler(Handler::from_json(&json!({"count": 3}))),
        )
        .build()
        .execute()
        .await
        .unwrap();

    requests_task.await.unwrap();
    assert_eq!(collected.len(), 3);
}
