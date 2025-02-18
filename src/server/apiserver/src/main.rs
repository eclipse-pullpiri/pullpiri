/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

mod grpc;
mod route;

async fn launch_grpc() {
    use common::apiserver::metric_connection_server::MetricConnectionServer;
    use grpc::receiver::metric_notifier::GrpcMetricServer;
    use tonic::transport::Server;

    let addr = common::apiserver::open_server()
        .parse()
        .expect("apiserver address parsing error");
    let metric_server = GrpcMetricServer::default();

    println!("grpc listening on {}", addr);
    let _ = Server::builder()
        .add_service(MetricConnectionServer::new(metric_server))
        .serve(addr)
        .await;
}

async fn launch_rest() {
    use axum::Router;
    use tokio::net::TcpListener;
    use tower_http::cors::{Any, CorsLayer};

    let addr = common::apiserver::open_rest_server();
    let listener = TcpListener::bind(addr).await.unwrap();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let app = Router::new()
        .merge(route::package::get_route())
        .merge(route::scenario::get_route())
        .merge(route::metric::get_route())
        .layer(cors);

    println!("http api listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn deploy_exist_package() {
    let _ = internal_deploy_exist_package().await;
}

async fn internal_deploy_exist_package() -> common::Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(3000));

    let package_path = format!("{}/packages", common::get_config().yaml_storage);
    let entries = std::fs::read_dir(package_path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "tar" {
                    if let Some(file_name) = path.file_stem() {
                        let name = file_name.to_string_lossy().to_string();
                        crate::route::package::handle_post(name).await;
                    }
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    tokio::join!(launch_grpc(), launch_rest(), deploy_exist_package());
}
