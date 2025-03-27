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
pub fn status(result: common::Result<()>) -> Response {
    if let Err(msg) = result {
        return (StatusCode::METHOD_NOT_ALLOWED, Json(String::from(msg.to_string()))).into_response();
    } else {
        return (StatusCode::OK, Json(String::from("Ok"))).into_response();
    }
}