/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Access point of Piccolo REST API

pub mod api;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

/// Serve Piccolo HTTP API service
///
/// ### Parametets
/// None
/// ### Description
/// CORS layer needs to be considerd.
pub async fn launch_tcp_listener() {
    let addr = common::apiserver::open_rest_server();
    let listener = TcpListener::bind(addr).await.unwrap();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let app = Router::new().merge(api::router()).layer(cors);

    println!("http api listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

/// Generate appropriate API response based on handler execution result
///
/// ### Parametets
/// * `result: Result<()>` - result of API handler logic
/// ### Description
/// Additional StatusCode may be added depending on the error.
#[allow(clippy::useless_conversion)]
pub fn status(result: common::Result<()>) -> Response {
    if let Err(msg) = result {
        (
            StatusCode::METHOD_NOT_ALLOWED,
            Json(String::from(msg.to_string())),
        )
            .into_response()
    } else {
        (StatusCode::OK, Json(String::from("Ok"))).into_response()
    }
}

//UNIT TEST CASES
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::{delete, get, post},
        Router,
    };
    use std::{error::Error as StdError, net::SocketAddr};
    use tokio::{
        net::{TcpListener, TcpStream},
        task,
        time::{sleep, Duration},
    };
    use tower::ServiceExt;
    use tower_http::cors::{Any, CorsLayer};

    // Test status function responses (Positive and Negative)
    #[test]
    fn test_status_responses() {
        // Positive case: OK response
        let ok_response = status(Ok(()));
        assert_eq!(ok_response.status(), StatusCode::OK);

        // Negative case: Error response
        let err = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error"))
            as Box<dyn StdError + Send + Sync>;
        let err_response = status(Err(err));
        assert_eq!(err_response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    // Test successful TCP listener launch (Positive)
    #[tokio::test]
    async fn test_launch_tcp_listener_success() {
        let handle = task::spawn(async {
            launch_tcp_listener().await;
        });

        sleep(Duration::from_millis(500)).await;

        let addr: SocketAddr = common::apiserver::open_rest_server()
            .parse()
            .expect("Invalid server address");

        let mut attempts = 0;
        let max_attempts = 5;
        let mut connected = false;

        while attempts < max_attempts && !connected {
            match TcpStream::connect(&addr).await {
                Ok(_) => connected = true,
                Err(_) => {
                    sleep(Duration::from_millis(100)).await;
                    attempts += 1;
                }
            }
        }

        assert!(
            connected,
            "Failed to connect to server after {} attempts",
            max_attempts
        );

        handle.abort();
    }

    // ❌ Negative test: Trying to bind to the same port twice (port conflict)
    #[tokio::test]
    async fn test_port_binding_conflict() {
        let addr: SocketAddr = common::apiserver::open_rest_server()
            .parse()
            .expect("Invalid server address");

        let listener1 = TcpListener::bind(&addr)
            .await
            .expect("First bind should succeed");

        let result = TcpListener::bind(&addr).await;
        assert!(
            result.is_err(),
            "Expected error when binding to the same port twice, but succeeded"
        );

        drop(listener1); // cleanup
    }

    // Test router configuration and valid endpoints (Positive)
    #[tokio::test]
    async fn test_router_configuration() {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route("/api/notify", get(|| async { StatusCode::OK }))
            .route("/api/artifact", post(|| async { StatusCode::OK }))
            .route("/api/artifact", delete(|| async { StatusCode::OK }))
            .layer(cors);

        let test_cases = [
            ("GET", "/api/notify", StatusCode::OK),
            ("POST", "/api/artifact", StatusCode::OK),
            ("DELETE", "/api/artifact", StatusCode::OK),
        ];

        for (method, path, expected_status) in test_cases {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                expected_status,
                "Failed for {} {}: expected {}, got {}",
                method,
                path,
                expected_status,
                response.status()
            );
        }
    }

    // ❌ Negative test: Invalid method on /api/notify
    #[tokio::test]
    async fn test_invalid_method_notify() {
        let app = Router::new().route("/api/notify", get(|| async { StatusCode::OK }));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST") // invalid method
                    .uri("/api/notify")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "Expected 405 when using invalid method on /api/notify"
        );
    }

    // ❌ Negative test: Invalid route
    #[tokio::test]
    async fn test_invalid_route() {
        let app = Router::new().route("/api/notify", get(|| async { StatusCode::OK }));

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/invalid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Expected 404 when accessing an invalid route"
        );
    }

    // Test CORS headers (Positive)
    #[tokio::test]
    async fn test_cors_headers() {
        let app = Router::new()
            .route("/dummy", get(|| async { StatusCode::OK }))
            .layer(CorsLayer::permissive());

        let response = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .header("origin", "http://example.com")
                    .header("access-control-request-method", "GET")
                    .uri("/dummy")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .contains_key("access-control-allow-origin"),
            "Missing CORS headers"
        );
    }
}
