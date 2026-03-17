/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Running gRPC message sending to timpani
use common::external::timpani::{
    connect_timpani_server, sched_info_service_client::SchedInfoServiceClient, Response, SchedInfo,
    SchedPolicy, TaskInfo,
};

pub async fn add_sched_info(workload_id: String, task_name: &str, node_id: &str) {
    println!("Connecting to Timpani server ....");
    let mut client = SchedInfoServiceClient::connect(connect_timpani_server())
        .await
        .unwrap();

    let request = SchedInfo {
        workload_id: workload_id,
        tasks: vec![TaskInfo {
            name: task_name.to_string(),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 10000, // 10 miliseconds
            release_time: 0,
            runtime: 5000,   // 5 miliseconds
            deadline: 10000, // 10 miliseconds
            node_id: node_id.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Direct Function Call Tests ====================

    #[test]
    fn test_create_timpani_test_request_structure() {
        let request = create_timpani_test_request();

        assert_eq!(request.workload_id, "timpani_test");
        assert_eq!(request.tasks.len(), 1);
        assert_eq!(request.tasks[0].name, "container_task");
        assert_eq!(request.tasks[0].priority, 50);
        assert_eq!(request.tasks[0].policy, SchedPolicy::Fifo as i32);
        assert_eq!(request.tasks[0].cpu_affinity, 7);
        assert_eq!(request.tasks[0].period, 10000);
        assert_eq!(request.tasks[0].release_time, 0);
        assert_eq!(request.tasks[0].runtime, 5000);
        assert_eq!(request.tasks[0].deadline, 10000);
        assert_eq!(request.tasks[0].node_id, "HPC");
        assert_eq!(request.tasks[0].max_dmiss, 3);
    }

    #[test]
    fn test_validate_task_constraints_boundary_release_time_equals_runtime() {
        let task = TaskInfo {
            name: String::from("boundary_task"),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 10000,
            release_time: 5000,
            runtime: 5000,
            deadline: 10000,
            node_id: String::from("HPC"),
            max_dmiss: 3,
        };

        assert!(validate_task_constraints(&task));
    }

    #[test]
    fn test_validate_task_constraints_release_time_greater_than_runtime() {
        let task = TaskInfo {
            name: String::from("invalid_release_task"),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 10000,
            release_time: 6000,
            runtime: 5000,
            deadline: 10000,
            node_id: String::from("HPC"),
            max_dmiss: 3,
        };

        assert!(!validate_task_constraints(&task));
    }

    #[test]
    fn test_validate_task_constraints_runtime_equals_deadline() {
        let task = TaskInfo {
            name: String::from("tight_deadline_task"),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 10000,
            release_time: 0,
            runtime: 10000,
            deadline: 10000,
            node_id: String::from("HPC"),
            max_dmiss: 3,
        };

        assert!(validate_task_constraints(&task));
    }

    #[test]
    fn test_validate_task_constraints_deadline_equals_period() {
        let task = TaskInfo {
            name: String::from("period_aligned_task"),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 10000,
            release_time: 0,
            runtime: 5000,
            deadline: 10000,
            node_id: String::from("HPC"),
            max_dmiss: 3,
        };

        assert!(validate_task_constraints(&task));
    }

    #[test]
    fn test_validate_sched_info_with_valid_workload() {
        let sched_info = SchedInfo {
            workload_id: String::from("valid_workload"),
            tasks: vec![TaskInfo {
                name: String::from("task1"),
                priority: 50,
                policy: SchedPolicy::Fifo as i32,
                cpu_affinity: 7,
                period: 10000,
                release_time: 0,
                runtime: 5000,
                deadline: 10000,
                node_id: String::from("HPC"),
                max_dmiss: 3,
            }],
        };

        assert!(validate_sched_info(&sched_info));
    }

    #[test]
    fn test_validate_sched_info_short_workload_id() {
        let sched_info = SchedInfo {
            workload_id: String::from("a"),
            tasks: vec![TaskInfo {
                name: String::from("task1"),
                priority: 50,
                policy: SchedPolicy::Fifo as i32,
                cpu_affinity: 7,
                period: 10000,
                release_time: 0,
                runtime: 5000,
                deadline: 10000,
                node_id: String::from("HPC"),
                max_dmiss: 3,
            }],
        };

        assert!(validate_sched_info(&sched_info));
    }

    #[test]
    fn test_validate_sched_info_empty_tasks_and_valid_id() {
        let sched_info = SchedInfo {
            workload_id: String::from("valid_id"),
            tasks: vec![],
        };

        assert!(validate_sched_info(&sched_info));
    }

    #[test]
    fn test_validate_sched_info_empty_workload_id_with_tasks() {
        let sched_info = SchedInfo {
            workload_id: String::new(),
            tasks: vec![TaskInfo {
                name: String::from("task1"),
                priority: 50,
                policy: SchedPolicy::Fifo as i32,
                cpu_affinity: 7,
                period: 10000,
                release_time: 0,
                runtime: 5000,
                deadline: 10000,
                node_id: String::from("HPC"),
                max_dmiss: 3,
            }],
        };

        assert!(!validate_sched_info(&sched_info));
    }

    #[test]
    fn test_validate_sched_info_first_task_valid_second_invalid() {
        let sched_info = SchedInfo {
            workload_id: String::from("multi_workload"),
            tasks: vec![
                TaskInfo {
                    name: String::from("task1"),
                    priority: 80,
                    policy: SchedPolicy::Fifo as i32,
                    cpu_affinity: 3,
                    period: 10000,
                    release_time: 0,
                    runtime: 5000,
                    deadline: 10000,
                    node_id: String::from("HPC"),
                    max_dmiss: 3,
                },
                TaskInfo {
                    name: String::from("task2"),
                    priority: 50,
                    policy: SchedPolicy::Fifo as i32,
                    cpu_affinity: 5,
                    period: 5000,
                    release_time: 0,
                    runtime: 10000,
                    deadline: 20000,
                    node_id: String::from("ZONE"),
                    max_dmiss: 5,
                },
            ],
        };

        assert!(!validate_sched_info(&sched_info));
    }

    #[test]
    fn test_validate_sched_info_all_tasks_valid() {
        let sched_info = SchedInfo {
            workload_id: String::from("multi_workload"),
            tasks: vec![
                TaskInfo {
                    name: String::from("task1"),
                    priority: 80,
                    policy: SchedPolicy::Fifo as i32,
                    cpu_affinity: 3,
                    period: 10000,
                    release_time: 0,
                    runtime: 5000,
                    deadline: 10000,
                    node_id: String::from("HPC"),
                    max_dmiss: 3,
                },
                TaskInfo {
                    name: String::from("task2"),
                    priority: 50,
                    policy: SchedPolicy::Fifo as i32,
                    cpu_affinity: 5,
                    period: 20000,
                    release_time: 0,
                    runtime: 10000,
                    deadline: 20000,
                    node_id: String::from("ZONE"),
                    max_dmiss: 5,
                },
            ],
        };

        assert!(validate_sched_info(&sched_info));
    }

    // ==================== TaskInfo Construction Tests ====================

    #[test]
    fn test_task_info_creation_with_valid_parameters() {
        let task = TaskInfo {
            name: String::from("test_task"),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 10000,
            release_time: 0,
            runtime: 5000,
            deadline: 10000,
            node_id: String::from("HPC"),
            max_dmiss: 3,
        };

        assert_eq!(task.name, "test_task");
        assert_eq!(task.priority, 50);
        assert_eq!(task.policy, SchedPolicy::Fifo as i32);
        assert_eq!(task.cpu_affinity, 7);
    }

    #[test]
    fn test_task_info_with_fifo_policy() {
        let task = TaskInfo {
            name: String::from("fifo_task"),
            priority: 80,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 1,
            period: 20000,
            release_time: 0,
            runtime: 10000,
            deadline: 20000,
            node_id: String::from("HPC"),
            max_dmiss: 5,
        };

        assert_eq!(task.policy, SchedPolicy::Fifo as i32);
        assert_eq!(task.priority, 80);
    }

    #[test]
    fn test_task_info_with_various_cpu_affinities() {
        let cpu_affinities = [0u64, 1, 7, 15, 31, 63, 127, 255];

        for cpu_aff in &cpu_affinities {
            let task = TaskInfo {
                name: String::from("cpu_affinity_task"),
                priority: 50,
                policy: SchedPolicy::Fifo as i32,
                cpu_affinity: *cpu_aff,
                period: 10000,
                release_time: 0,
                runtime: 5000,
                deadline: 10000,
                node_id: String::from("HPC"),
                max_dmiss: 3,
            };

            assert_eq!(task.cpu_affinity, *cpu_aff);
        }
    }

    #[test]
    fn test_task_info_runtime_less_than_deadline() {
        let task = TaskInfo {
            name: String::from("feasible_task"),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 10000,
            release_time: 0,
            runtime: 5000,
            deadline: 10000,
            node_id: String::from("HPC"),
            max_dmiss: 3,
        };

        assert!(task.runtime <= task.deadline);
    }

    // ==================== SchedInfo Construction Tests ====================

    #[test]
    fn test_sched_info_with_single_task() {
        let task = TaskInfo {
            name: String::from("single_task"),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 10000,
            release_time: 0,
            runtime: 5000,
            deadline: 10000,
            node_id: String::from("HPC"),
            max_dmiss: 3,
        };

        let sched_info = SchedInfo {
            workload_id: String::from("test_workload"),
            tasks: vec![task],
        };

        assert_eq!(sched_info.workload_id, "test_workload");
        assert_eq!(sched_info.tasks.len(), 1);
    }

    #[test]
    fn test_sched_info_with_multiple_tasks() {
        let tasks = vec![
            TaskInfo {
                name: String::from("task_1"),
                priority: 80,
                policy: SchedPolicy::Fifo as i32,
                cpu_affinity: 3,
                period: 10000,
                release_time: 0,
                runtime: 5000,
                deadline: 10000,
                node_id: String::from("HPC"),
                max_dmiss: 3,
            },
            TaskInfo {
                name: String::from("task_2"),
                priority: 60,
                policy: SchedPolicy::Fifo as i32,
                cpu_affinity: 5,
                period: 20000,
                release_time: 100,
                runtime: 10000,
                deadline: 20000,
                node_id: String::from("ZONE"),
                max_dmiss: 5,
            },
        ];

        let sched_info = SchedInfo {
            workload_id: String::from("multi_workload"),
            tasks,
        };

        assert_eq!(sched_info.tasks.len(), 2);
    }

    #[test]
    fn test_sched_info_with_empty_tasks() {
        let sched_info = SchedInfo {
            workload_id: String::from("empty_workload"),
            tasks: vec![],
        };

        assert_eq!(sched_info.workload_id, "empty_workload");
        assert_eq!(sched_info.tasks.len(), 0);
    }

    #[test]
    fn test_default_timpani_test_sched_info() {
        let request = create_timpani_test_request();

        assert_eq!(request.workload_id, "timpani_test");
        assert_eq!(request.tasks.len(), 1);
        assert_eq!(request.tasks[0].name, "container_task");
        assert_eq!(request.tasks[0].priority, 50);
        assert_eq!(request.tasks[0].period, 10000);
    }

    #[test]
    fn test_realistic_realtime_scenario() {
        let high_priority_task = TaskInfo {
            name: String::from("critical_control"),
            priority: 99,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 1,
            period: 5000,
            release_time: 0,
            runtime: 2000,
            deadline: 5000,
            node_id: String::from("HPC"),
            max_dmiss: 0,
        };

        let medium_priority_task = TaskInfo {
            name: String::from("data_processing"),
            priority: 50,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 6,
            period: 20000,
            release_time: 0,
            runtime: 10000,
            deadline: 20000,
            node_id: String::from("ZONE"),
            max_dmiss: 2,
        };

        let sched_info = SchedInfo {
            workload_id: String::from("realtime_system"),
            tasks: vec![high_priority_task, medium_priority_task],
        };

        assert_eq!(sched_info.tasks.len(), 2);
        assert!(validate_sched_info(&sched_info));
    }

    #[test]
    fn test_multiple_nodes_configuration() {
        let hpc_task = TaskInfo {
            name: String::from("hpc_compute"),
            priority: 80,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 15,
            period: 10000,
            release_time: 0,
            runtime: 5000,
            deadline: 10000,
            node_id: String::from("HPC"),
            max_dmiss: 1,
        };

        let zone_task = TaskInfo {
            name: String::from("zone_io"),
            priority: 60,
            policy: SchedPolicy::Fifo as i32,
            cpu_affinity: 7,
            period: 20000,
            release_time: 100,
            runtime: 10000,
            deadline: 20000,
            node_id: String::from("ZONE"),
            max_dmiss: 3,
        };

        let sched_info = SchedInfo {
            workload_id: String::from("multi_node_workload"),
            tasks: vec![hpc_task, zone_task],
        };

        assert_eq!(sched_info.tasks.len(), 2);
        assert_eq!(sched_info.tasks[0].node_id, "HPC");
        assert_eq!(sched_info.tasks[1].node_id, "ZONE");
        assert!(validate_sched_info(&sched_info));
    }

    // Helper function to create default TimPani test request
    pub fn create_timpani_test_request() -> SchedInfo {
        SchedInfo {
            workload_id: String::from("timpani_test"),
            tasks: vec![TaskInfo {
                name: String::from("container_task"),
                priority: 50,
                policy: SchedPolicy::Fifo as i32,
                cpu_affinity: 7,
                period: 10000,
                release_time: 0,
                runtime: 5000,
                deadline: 10000,
                node_id: String::from("HPC"),
                max_dmiss: 3,
            }],
        }
    }

    // Helper function to validate task constraints
    pub fn validate_task_constraints(task: &TaskInfo) -> bool {
        task.release_time <= task.runtime
            && task.runtime <= task.deadline
            && task.deadline <= task.period
    }

    // Helper function to validate SchedInfo
    pub fn validate_sched_info(sched_info: &SchedInfo) -> bool {
        !sched_info.workload_id.is_empty() && sched_info.tasks.iter().all(validate_task_constraints)
    }
}
