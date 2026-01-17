//! Simple example demonstrating basic usage of grpc-rpc-server-harness
//!
//! This example shows how to:
//! - Define a scenario with gRPC services and methods using the Builder pattern
//! - Use static and dynamic handlers
//! - Execute the scenario from another thread
//! - Get the collected requests once all handlers have been called
//!
//! The server automatically shuts down once all handlers have been called.

use grpc_rpc_server_harness::prelude::*;
use grpc_rpc_server_harness::use_cases::ports::Server;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Notify;

/// Helper to create a gRPC request body with length prefix
fn grpc_request_body(data: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(5 + data.len());
    body.push(0); // No compression
    body.extend_from_slice(&(data.len() as u32).to_be_bytes());
    body.extend_from_slice(data);
    body
}

/// Helper to parse gRPC response body (skip 5-byte header)
fn parse_grpc_response(data: &[u8]) -> &[u8] {
    if data.len() > 5 {
        &data[5..]
    } else {
        &[]
    }
}

#[tokio::main]
async fn main() -> Result<(), HarnessError> {
    println!("Starting gRPC scenario...");
    println!("The server will shut down automatically once all handlers are called.\n");

    // Use port 0 to let the OS assign an available port
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    // Create notification mechanism for when server is ready
    let addr_notify = Arc::new(Notify::new());
    let addr_holder = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));

    let addr_notify_clone = addr_notify.clone();
    let addr_holder_clone = addr_holder.clone();

    // Spawn a task that will make gRPC requests to our server
    let requests_task = tokio::spawn(async move {
        // Wait for server to be ready
        addr_notify_clone.notified().await;
        let server_addr = addr_holder_clone.lock().unwrap().unwrap();
        println!("Server is ready at http://{}\n", server_addr);

        // Create HTTP/2 client for gRPC requests
        let client = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http();

        // Request 1: GetUser (static handler)
        println!("Making gRPC request to UserService/GetUser...");
        let request1 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/example.UserService/GetUser", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[1, 2, 3]))))
            .unwrap();

        let response1 = client.request(request1).await.expect("Request failed");
        let body1 = response1.into_body().collect().await.unwrap().to_bytes();
        println!("Response: {:?}\n", parse_grpc_response(&body1));

        // Request 2: Echo (dynamic handler)
        println!("Making gRPC request to EchoService/Echo...");
        let request2 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/example.EchoService/Echo", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[10, 20, 30]))))
            .unwrap();

        let response2 = client.request(request2).await.expect("Request failed");
        let body2 = response2.into_body().collect().await.unwrap().to_bytes();
        println!("Response (echoed with prefix): {:?}\n", parse_grpc_response(&body2));

        // Request 3: CreateUser (static handler)
        println!("Making gRPC request to UserService/CreateUser...");
        let request3 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/example.UserService/CreateUser", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[4, 5, 6]))))
            .unwrap();

        let response3 = client.request(request3).await.expect("Request failed");
        let body3 = response3.into_body().collect().await.unwrap().to_bytes();
        println!("Response: {:?}\n", parse_grpc_response(&body3));
    });

    // Build and execute the scenario using the Server::run API directly
    // (to use the on_ready callback for port notification)
    let collected_requests = Tonic::bind(addr)
        .run(
            vec![
                // UserService with static handlers
                Service::new("example.UserService")
                    .with_method(
                        Method::new("GetUser")
                            .with_handler(Handler::from_bytes(vec![0x01, 0x02, 0x03, 0x04])),
                    )
                    .with_method(
                        Method::new("CreateUser")
                            .with_handler(Handler::from_bytes(vec![0xAA, 0xBB, 0xCC])),
                    ),
                // EchoService with dynamic handler
                Service::new("example.EchoService").with_method(
                    Method::new("Echo").with_handler(Handler::dynamic(|ctx: &RequestContext| {
                        // Echo back the input with a prefix byte
                        let mut response = vec![0xFF];
                        response.extend_from_slice(&ctx.message.data);
                        Message::new(response)
                    })),
                ),
            ],
            DefaultCollector::new(),
            Some(move |actual_addr: SocketAddr| {
                *addr_holder.lock().unwrap() = Some(actual_addr);
                addr_notify.notify_one();
            }),
        )
        .await?;

    // Wait for the requests task to complete
    requests_task.await.expect("Requests task panicked");

    // Print collected requests
    println!("=== Collected Requests ===");
    for (i, req) in collected_requests.iter().enumerate() {
        println!("Request {}: {}/{}", i + 1, req.service, req.method);
        if !req.message.data.is_empty() {
            println!("  Message data: {:?}", req.message.data);
        }
    }

    Ok(())
}

