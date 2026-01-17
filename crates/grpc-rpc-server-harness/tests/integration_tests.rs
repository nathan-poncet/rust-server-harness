//! Integration tests for grpc-rpc-server-harness

use grpc_rpc_server_harness::prelude::*;
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

#[tokio::test]
async fn test_single_service_single_method() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let addr_notify = Arc::new(Notify::new());
    let addr_holder = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));

    let addr_notify_clone = addr_notify.clone();
    let addr_holder_clone = addr_holder.clone();

    // Spawn the client task
    let client_task = tokio::spawn(async move {
        // Wait for server to be ready
        addr_notify_clone.notified().await;
        let server_addr = addr_holder_clone.lock().unwrap().unwrap();

        // Make HTTP/2 request
        let client = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http();

        let request = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.TestService/GetData", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[10, 20, 30]))))
            .unwrap();

        let response = client.request(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let response_data = parse_grpc_response(&body);
        assert_eq!(response_data, &[1, 2, 3, 4]);
    });

    // Build and execute the scenario (just testing the builder pattern compiles)
    let _scenario = ScenarioBuilder::new()
        .server(Tonic::bind(addr))
        .collector(DefaultCollector::new())
        .service(
            Service::new("test.TestService")
                .with_method(Method::new("GetData").with_handler(Handler::from_bytes(vec![1, 2, 3, 4]))),
        )
        .build();

    // Use Server::run directly with on_ready callback
    use grpc_rpc_server_harness::use_cases::ports::Server;
    let result = Tonic::bind(addr)
        .run(
            vec![Service::new("test.TestService")
                .with_method(Method::new("GetData").with_handler(Handler::from_bytes(vec![1, 2, 3, 4])))],
            DefaultCollector::new(),
            Some(move |actual_addr: SocketAddr| {
                *addr_holder.lock().unwrap() = Some(actual_addr);
                addr_notify.notify_one();
            }),
        )
        .await
        .unwrap();

    client_task.await.unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].service, "test.TestService");
    assert_eq!(result[0].method, "GetData");
}

#[tokio::test]
async fn test_multiple_methods() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let addr_notify = Arc::new(Notify::new());
    let addr_holder = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));

    let addr_notify_clone = addr_notify.clone();
    let addr_holder_clone = addr_holder.clone();

    // Spawn the client task
    let client_task = tokio::spawn(async move {
        addr_notify_clone.notified().await;
        let server_addr = addr_holder_clone.lock().unwrap().unwrap();

        let client = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http();

        // Call GetUser
        let request1 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.UserService/GetUser", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[]))))
            .unwrap();

        let response1 = client.request(request1).await.unwrap();
        let body1 = response1.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(parse_grpc_response(&body1), &[1, 1, 1]);

        // Call CreateUser
        let request2 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.UserService/CreateUser", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[]))))
            .unwrap();

        let response2 = client.request(request2).await.unwrap();
        let body2 = response2.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(parse_grpc_response(&body2), &[2, 2, 2]);
    });

    use grpc_rpc_server_harness::use_cases::ports::Server;
    let result = Tonic::bind(addr)
        .run(
            vec![Service::new("test.UserService")
                .with_method(Method::new("GetUser").with_handler(Handler::from_bytes(vec![1, 1, 1])))
                .with_method(Method::new("CreateUser").with_handler(Handler::from_bytes(vec![2, 2, 2])))],
            DefaultCollector::new(),
            Some(move |actual_addr: SocketAddr| {
                *addr_holder.lock().unwrap() = Some(actual_addr);
                addr_notify.notify_one();
            }),
        )
        .await
        .unwrap();

    client_task.await.unwrap();
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_sequential_handlers() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let addr_notify = Arc::new(Notify::new());
    let addr_holder = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));

    let addr_notify_clone = addr_notify.clone();
    let addr_holder_clone = addr_holder.clone();

    let client_task = tokio::spawn(async move {
        addr_notify_clone.notified().await;
        let server_addr = addr_holder_clone.lock().unwrap().unwrap();

        let client = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http();

        for expected in [1u8, 2, 3, 3] {
            let request = hyper::Request::builder()
                .method("POST")
                .uri(format!("http://{}/test.Service/Call", server_addr))
                .header("content-type", "application/grpc")
                .body(Full::new(Bytes::from(grpc_request_body(&[]))))
                .unwrap();

            let response = client.request(request).await.unwrap();
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(parse_grpc_response(&body), &[expected]);
        }
    });

    use grpc_rpc_server_harness::use_cases::ports::Server;
    let result = Tonic::bind(addr)
        .run(
            vec![Service::new("test.Service")
                .with_method(
                    Method::new("Call")
                        .with_handler(Handler::from_bytes(vec![1]))
                        .with_handler(Handler::from_bytes(vec![2]))
                        .with_handler(Handler::from_bytes(vec![3]))
                        .with_handler(Handler::from_bytes(vec![3])),
                )],
            DefaultCollector::new(),
            Some(move |actual_addr: SocketAddr| {
                *addr_holder.lock().unwrap() = Some(actual_addr);
                addr_notify.notify_one();
            }),
        )
        .await
        .unwrap();

    client_task.await.unwrap();
    assert_eq!(result.len(), 4);
}

#[tokio::test]
async fn test_unimplemented_method() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let addr_notify = Arc::new(Notify::new());
    let addr_holder = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));

    let addr_notify_clone = addr_notify.clone();
    let addr_holder_clone = addr_holder.clone();

    let client_task = tokio::spawn(async move {
        addr_notify_clone.notified().await;
        let server_addr = addr_holder_clone.lock().unwrap().unwrap();

        let client = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http();

        // Call non-existent method
        let request = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.Service/DoesNotExist", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[]))))
            .unwrap();

        let response = client.request(request).await.unwrap();
        let grpc_status = response
            .headers()
            .get("grpc-status")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(grpc_status, "12"); // UNIMPLEMENTED

        // Also call the existing method so the server shuts down
        let request2 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.Service/Exists", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[]))))
            .unwrap();
        let _ = client.request(request2).await.unwrap();
    });

    use grpc_rpc_server_harness::use_cases::ports::Server;
    let _ = Tonic::bind(addr)
        .run(
            vec![Service::new("test.Service")
                .with_method(Method::new("Exists").with_handler(Handler::from_bytes(vec![1])))],
            DefaultCollector::new(),
            Some(move |actual_addr: SocketAddr| {
                *addr_holder.lock().unwrap() = Some(actual_addr);
                addr_notify.notify_one();
            }),
        )
        .await
        .unwrap();

    client_task.await.unwrap();
}

#[tokio::test]
async fn test_dynamic_handler_echo() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let addr_notify = Arc::new(Notify::new());
    let addr_holder = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));

    let addr_notify_clone = addr_notify.clone();
    let addr_holder_clone = addr_holder.clone();

    let client_task = tokio::spawn(async move {
        addr_notify_clone.notified().await;
        let server_addr = addr_holder_clone.lock().unwrap().unwrap();

        let client = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http();

        let request = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.EchoService/Echo", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[1, 2, 3]))))
            .unwrap();

        let response = client.request(request).await.unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let response_data = parse_grpc_response(&body);

        // Should be prefix 0xEE followed by input [1, 2, 3]
        assert_eq!(response_data, &[0xEE, 1, 2, 3]);
    });

    use grpc_rpc_server_harness::use_cases::ports::Server;
    let _ = Tonic::bind(addr)
        .run(
            vec![Service::new("test.EchoService").with_method(
                Method::new("Echo").with_handler(Handler::dynamic(|ctx| {
                    let mut response = vec![0xEE];
                    response.extend_from_slice(&ctx.message.data);
                    Message::new(response)
                })),
            )],
            DefaultCollector::new(),
            Some(move |actual_addr: SocketAddr| {
                *addr_holder.lock().unwrap() = Some(actual_addr);
                addr_notify.notify_one();
            }),
        )
        .await
        .unwrap();

    client_task.await.unwrap();
}

#[tokio::test]
async fn test_dynamic_handler_based_on_method_name() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let addr_notify = Arc::new(Notify::new());
    let addr_holder = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));

    let addr_notify_clone = addr_notify.clone();
    let addr_holder_clone = addr_holder.clone();

    let client_task = tokio::spawn(async move {
        addr_notify_clone.notified().await;
        let server_addr = addr_holder_clone.lock().unwrap().unwrap();

        let client = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http();

        // Test GetData
        let request1 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.MultiService/GetData", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[]))))
            .unwrap();

        let response1 = client.request(request1).await.unwrap();
        let body1 = response1.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(parse_grpc_response(&body1), &[0x01, 0x01]);

        // Test SetData with 5 bytes of input
        let request2 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.MultiService/SetData", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[1, 2, 3, 4, 5]))))
            .unwrap();

        let response2 = client.request(request2).await.unwrap();
        let body2 = response2.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(parse_grpc_response(&body2), &[5]); // Length of input
    });

    use grpc_rpc_server_harness::use_cases::ports::Server;
    let _ = Tonic::bind(addr)
        .run(
            vec![Service::new("test.MultiService")
                .with_method(
                    Method::new("GetData").with_handler(Handler::dynamic(|ctx| {
                        if ctx.method == "GetData" {
                            Message::new(vec![0x01, 0x01])
                        } else {
                            Message::new(vec![0x00, 0x00])
                        }
                    })),
                )
                .with_method(
                    Method::new("SetData").with_handler(Handler::dynamic(|ctx| {
                        let len = ctx.message.data.len() as u8;
                        Message::new(vec![len])
                    })),
                )],
            DefaultCollector::new(),
            Some(move |actual_addr: SocketAddr| {
                *addr_holder.lock().unwrap() = Some(actual_addr);
                addr_notify.notify_one();
            }),
        )
        .await
        .unwrap();

    client_task.await.unwrap();
}


#[tokio::test]
async fn test_auto_shutdown_after_all_handlers() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let addr_notify = Arc::new(Notify::new());
    let addr_holder = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));

    let addr_notify_clone = addr_notify.clone();
    let addr_holder_clone = addr_holder.clone();

    let client_task = tokio::spawn(async move {
        addr_notify_clone.notified().await;
        let server_addr = addr_holder_clone.lock().unwrap().unwrap();

        let client = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http();

        // Call first service
        let request1 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.Service1/Call1", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[]))))
            .unwrap();
        let _ = client.request(request1).await.unwrap();

        // Call second service
        let request2 = hyper::Request::builder()
            .method("POST")
            .uri(format!("http://{}/test.Service2/Call2", server_addr))
            .header("content-type", "application/grpc")
            .body(Full::new(Bytes::from(grpc_request_body(&[]))))
            .unwrap();
        let _ = client.request(request2).await.unwrap();
    });

    use grpc_rpc_server_harness::use_cases::ports::Server;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        Tonic::bind(addr).run(
            vec![
                Service::new("test.Service1")
                    .with_method(Method::new("Call1").with_handler(Handler::from_bytes(vec![1]))),
                Service::new("test.Service2")
                    .with_method(Method::new("Call2").with_handler(Handler::from_bytes(vec![2]))),
            ],
            DefaultCollector::new(),
            Some(move |actual_addr: SocketAddr| {
                *addr_holder.lock().unwrap() = Some(actual_addr);
                addr_notify.notify_one();
            }),
        ),
    )
    .await;

    assert!(
        result.is_ok(),
        "Server should have shut down automatically after all handlers were called"
    );

    client_task.await.unwrap();

    // Verify collected requests
    let collected = result.unwrap().unwrap();
    assert_eq!(collected.len(), 2);
}