/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Running gRPC message sending to pharos
use common::external::{
    connect_timpani_server, sched_info_service_client::SchedInfoServiceClient, Response, SchedInfo,
    SchedPolicy, TaskInfo,
};

pub async fn add_sched_info() {
    println!("Connecting to Timpani server ....");
    let mut client = SchedInfoServiceClient::connect(connect_timpani_server())
        .await
        .unwrap();

    let request = SchedInfo {
        workload_id: String::from("timpani_test"),
        tasks: vec![TaskInfo {
            name: String::from("container_task"),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 10000, // 10 miliseconds
            release_time: 0,
            runtime: 5000,   // 5 miliseconds
            deadline: 10000, // 10 miliseconds
            node_id: String::from("HPC"),
            max_dmiss: 3,
        }],
    };

    let response: Result<Response, tonic::Status> =
        client.add_sched_info(request).await.map(|r| r.into_inner());

    match response {
        Ok(res) => {
            println!("[add_sched_info] RESPONSE={:?}", res);
        }
        Err(e) => {
            println!("[add_sched_info] ERROR={:?}", e);
        }
    }
}
