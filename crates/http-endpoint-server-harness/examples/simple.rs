//! Simple example demonstrating basic usage of http-endpoint-server-harness
//!
//! This example shows how to:
//! - Define a scenario with static and dynamic handlers using the Builder pattern
//! - Execute the scenario from another thread
//! - Get the collected requests once all handlers have been called
//!
//! The server automatically shuts down once all handlers have been called.

use http_endpoint_server_harness::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), HarnessError> {
    println!("Starting scenario...");
    println!("The server will shut down automatically once all handlers are called.\n");

    // Define a fixed address for the server
    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();

    // Spawn a task that will make HTTP requests to our server
    let requests_task = tokio::spawn(async move {
        // Wait for server to be ready
        let client = reqwest::Client::new();
        loop {
            if client
                .get(format!("http://{}/", addr))
                .timeout(Duration::from_millis(50))
                .send()
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        println!("Server is ready at http://{}\n", addr);

        // Request 1: Get auth token
        println!("Making request to /api/auth/token...");
        let resp: serde_json::Value = client
            .post(format!("http://{}/api/auth/token", addr))
            .send()
            .await
            .expect("Request failed")
            .json()
            .await
            .expect("Failed to parse JSON");
        println!("Response: {}\n", resp);

        // Request 2: Greet endpoint
        println!("Making request to /api/greet...");
        let resp: serde_json::Value = client
            .post(format!("http://{}/api/greet", addr))
            .json(&serde_json::json!({"name": "World"}))
            .send()
            .await
            .expect("Request failed")
            .json()
            .await
            .expect("Failed to parse JSON");
        println!("Response: {}\n", resp);

        // Request 3: Get user
        println!("Making request to /api/users/1...");
        let resp: serde_json::Value = client
            .get(format!("http://{}/api/users/1", addr))
            .send()
            .await
            .expect("Request failed")
            .json()
            .await
            .expect("Failed to parse JSON");
        println!("Response: {}\n", resp);
    });

    // Build and execute the scenario
    let collected_requests = ScenarioBuilder::new()
        // Set the server implementation with fixed address
        .server(Axum::bind(addr))
        // Set the collector
        .collector(DefaultCollector::new())
        // Static handler: returns predefined responses sequentially
        .endpoint(
            Endpoint::new("/api/auth/token", Method::Post)
                .with_handler(Handler::from_json(&json!({
                    "access_token": "abc123",
                    "token_type": "Bearer",
                    "expires_in": 3600
                }))),
        )
        // Dynamic handler: builds response based on the request
        .endpoint(
            Endpoint::new("/api/greet", Method::Post).with_handler(Handler::dynamic(
                |req: &Request| {
                    if let Some(body_str) = req.body_as_str() {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
                            if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                                return Response::ok().with_json(&json!({
                                    "message": format!("Hello, {}!", name)
                                }));
                            }
                        }
                    }
                    Response::new(400).with_body("Invalid request: missing 'name' field")
                },
            )),
        )
        // Static handler with custom headers
        .endpoint(
            Endpoint::new("/api/users/{id}", Method::Get).with_handler(
                Handler::from_json(&json!({
                    "id": 1,
                    "name": "John Doe",
                    "email": "john@example.com"
                }))
                .with_header("X-Custom-Header", "custom-value"),
            ),
        )
        .build()
        .execute()
        .await?;

    // Wait for the requests task to complete
    requests_task.await.expect("Requests task panicked");

    // Print collected requests
    println!("=== Collected Requests ===");
    for (i, req) in collected_requests.iter().enumerate() {
        println!("Request {}: {} {}", i + 1, req.method, req.path);
        if let Some(body) = req.body_as_str() {
            if !body.is_empty() {
                println!("  Body: {}", body);
            }
        }
    }

    Ok(())
}
