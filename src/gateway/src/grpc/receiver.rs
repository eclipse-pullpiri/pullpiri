/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::gateway::connection_server::Connection;
use common::gateway::{Condition, Response};
use tokio::sync::mpsc;

pub struct GrpcServer {
    pub grpc_msg_tx: mpsc::Sender<Condition>,
}

#[tonic::async_trait]
impl Connection for GrpcServer {
    async fn send_condition(
        &self,
        request: tonic::Request<Condition>,
    ) -> Result<tonic::Response<Response>, tonic::Status> {
        println!("Got a request from api-server");
        let req: Condition = request.into_inner();
        //println!("req msg : {:#?}", req);
        match self.grpc_msg_tx.send(req).await {
            Ok(_) => Ok(tonic::Response::new(Response {
                resp: true.to_string(),
            })),
            Err(_) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                "cannot send condition",
            )),
        }
    }
}
