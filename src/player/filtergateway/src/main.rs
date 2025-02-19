// SPDX-License-Identifier: Apache-2.0

mod filter;
mod grpc;
mod manager;

use common::filtergateway::Condition;
use tokio::sync::mpsc::{channel, Receiver, Sender};

async fn launch_manager(rx: Receiver<Condition>) {
    let mut manager = manager::Manager::new(rx);
    manager.run().await;
}

async fn launch_grpc(tx: Sender<Condition>) {
    use common::filtergateway::connection_server::ConnectionServer;
    use tonic::transport::Server;

    let server = crate::grpc::receiver::GrpcServer { grpc_msg_tx: tx };
    let addr = common::filtergateway::open_server()
        .parse()
        .expect("filtergateway address parsing error");

    println!("Piccolo filtergateway listening on {}", addr);

    let _ = Server::builder()
        .add_service(ConnectionServer::new(server))
        .serve(addr)
        .await;
}

#[tokio::main]
async fn main() {
    let (tx_grpc, rx_grpc) = channel::<Condition>(100);
    let f_grpc = launch_grpc(tx_grpc);
    let f_manage = launch_manager(rx_grpc);

    tokio::join!(f_grpc, f_manage);
}
