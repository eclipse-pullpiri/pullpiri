// SPDX-License-Identifier: Apache-2.0

pub mod api;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

pub async fn launch_tcp_listener() {
    let addr = common::apiserver::open_rest_server();
    let listener = TcpListener::bind(addr).await.unwrap();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let app = Router::new().merge(api::get_route()).layer(cors);

    println!("http api listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(serde::Serialize)]
pub struct ResponseData {
    message: String,
}

pub fn status_ok() -> Response {
    println!("StatusCode::OK, resp: Ok");
    let response = ResponseData {
        message: String::from("Ok"),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub fn status_err(msg: &str) -> Response {
    println!("StatusCode::NOT_FOUND, resp: {msg}");
    let response = ResponseData {
        message: String::from(msg),
    };
    (StatusCode::METHOD_NOT_ALLOWED, Json(response)).into_response()
}
