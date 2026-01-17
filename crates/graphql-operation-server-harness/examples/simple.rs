//! Simple example demonstrating basic usage of graphql-operation-server-harness
//!
//! This example shows how to:
//! - Define a scenario with GraphQL operations using the Builder pattern
//! - Execute the scenario from another thread
//! - Get the collected requests once all handlers have been called

use graphql_operation_server_harness::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), HarnessError> {
    // Define a fixed address
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

    // Spawn a task to make GraphQL requests
    let requests_task = tokio::spawn(async move {
        // Wait for server to be ready
        let client = reqwest::Client::new();
        loop {
            if client
                .post(format!("http://{}/graphql", addr))
                .json(&serde_json::json!({
                    "query": "{ __typename }"
                }))
                .timeout(Duration::from_millis(50))
                .send()
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Make a query request
        println!("Making GraphQL query request...");
        let response = client
            .post(format!("http://{}/graphql", addr))
            .json(&serde_json::json!({
                "query": "{ users { id name } }"
            }))
            .send()
            .await
            .unwrap();
        println!("Query response: {:?}", response.text().await);

        // Make a mutation request
        println!("Making GraphQL mutation request...");
        let response = client
            .post(format!("http://{}/graphql", addr))
            .json(&serde_json::json!({
                "query": "mutation { createUser(name: \"Jane\") { id name } }",
                "variables": { "name": "Jane" }
            }))
            .send()
            .await
            .unwrap();
        println!("Mutation response: {:?}", response.text().await);
    });

    // Build and execute the scenario
    let collected = ScenarioBuilder::new()
        .server(AsyncGraphQL::bind(addr))
        .collector(DefaultCollector::new())
        // Define a query operation with a static handler
        .operation(
            Operation::query().with_field(
                Field::new("users").with_handler(Handler::new(json!({
                    "users": [
                        {"id": 1, "name": "John"},
                        {"id": 2, "name": "Alice"}
                    ]
                }))),
            ),
        )
        // Define a mutation operation with a dynamic handler
        .operation(
            Operation::mutation().with_field(
                Field::new("createUser").with_handler(Handler::dynamic(|ctx: &RequestContext| {
                    // Access variables from the request context
                    let name = ctx
                        .get_variable("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    HandlerResponse::new(json!({
                        "createUser": {"id": 3, "name": name}
                    }))
                })),
            ),
        )
        .build()
        .execute()
        .await?;

    requests_task.await.unwrap();

    // Print results
    println!("\n=== Collected Requests ===");
    for (i, req) in collected.iter().enumerate() {
        println!("Request {}:", i + 1);
        println!("  Query: {}", req.query);
        if let Some(op_name) = &req.operation_name {
            println!("  Operation Name: {}", op_name);
        }
        if let Some(vars) = &req.variables {
            println!("  Variables: {}", vars);
        }
        println!();
    }

    Ok(())
}

