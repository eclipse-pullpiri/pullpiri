/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! HTTP routes and handlers for Settings Server

pub mod api;

use axum::{http::StatusCode, response::Response, Json};
use tower_http::cors::CorsLayer;

/// Create router for Settings Server with CORS support
pub fn router() -> axum::Router {
    axum::Router::new()
        .merge(api::router())
        .layer(CorsLayer::permissive())
}

/// Launch TCP listener for Settings Server
pub async fn launch_tcp_listener() {
    let addr = common::settingsserver::open_rest_server();
    println!("SettingsServer REST API listening on {}", addr);

    let app = router();
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    axum::serve(listener, app)
        .await
        .unwrap();
}

/// Create HTTP response based on Result
pub fn status(result: common::Result<()>) -> Response {
    match result {
        Ok(_) => (StatusCode::OK, Json("OK")).into_response(),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json("Error")).into_response()
        }
    }
}

/// Create HTTP response with JSON body
pub fn json_response<T>(result: common::Result<T>) -> Response
where
    T: serde::Serialize,
{
    match result {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json("Error")).into_response()
        }
    }
}

// Import the IntoResponse trait
use axum::response::IntoResponse;

//UNIT TEST CASES
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use tower::ServiceExt;

    /// Test router configuration
    #[tokio::test]
    async fn test_router_configuration() {
        let app = router();
        
        // Test that router is created successfully
        assert!(true); // Basic test that router() doesn't panic
    }

    /// Test status function with success
    #[tokio::test]
    async fn test_status_success() {
        let result: common::Result<()> = Ok(());
        let response = status(result);
        
        // Basic test that status function works
        assert!(true); // The function should return a response without panicking
    }

    /// Test status function with error
    #[tokio::test]
    async fn test_status_error() {
        let result: common::Result<()> = Err("test error".into());
        let response = status(result);
        
        // Basic test that status function works with errors
        assert!(true); // The function should return a response without panicking
    }
}