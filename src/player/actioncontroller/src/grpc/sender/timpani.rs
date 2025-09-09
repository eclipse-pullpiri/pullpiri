/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Running gRPC message sending to pharos

use common::external::{
    connect_timpani_server, sched_info_service_client::SchedInfoServiceClient, Response, SchedInfo, SchedPolicy,
    TaskInfo,
};

pub async fn add_sched_info() {
    println!("Connecting to Timpani server ....");
    let mut client = SchedInfoServiceClient::connect(connect_timpani_server())
        .await
        .unwrap();

    let request = SchedInfo {
        workload_id: String::from("workload-123"),
        tasks: vec![TaskInfo {
            name: String::from("task-1"),
            priority: 50,
            policy: SchedPolicy::Normal as i32,
            cpu_affinity: 0,
            period: 1000000,
            release_time: 0,
            runtime: 100000,
            deadline: 900000,
            node_id: String::from("node1"),
            max_dmiss: 3,
        }],
    };

    let response: Result<Response, tonic::Status> =
        client.add_sched_info(request).await.map(|r| r.into_inner());

    match response {
        Ok(res) => {
            println!("RESPONSE={:?}", res);
        }
        Err(e) => {
            println!("ERROR={:?}", e);
        }
    }
}
