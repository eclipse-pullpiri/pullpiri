/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::monitoringserver::monitoring_server_connection_server::MonitoringServerConnection;
use common::monitoringserver::{
    ContainerList, NodeInfo, SendContainerListResponse, SendNodeInfoResponse,
    StressMonitoringMetric, StressMonitoringMetricResponse,
};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

use serde::Deserialize;
use serde_json;
use std::fmt;

/// JSON types for StressMonitoringMetric payload
#[derive(Debug, Deserialize)]
pub struct CpuLoad {
    pub core_id: u32,
    pub load: f64,
}

#[derive(Debug, Deserialize)]
pub struct StressMonitoringMetricParsed {
    pub process_name: String,
    pub pid: u32,
    pub core_masking: Option<String>,
    pub core_count: Option<u32>,
    pub fps: f64,
    pub latency: u64,
    pub cpu_loads: Vec<CpuLoad>,
}

impl StressMonitoringMetricParsed {
    // If core_count was provided, return it; otherwise derive from max core_id in cpu_loads.
    pub fn effective_core_count(&self) -> u32 {
        if let Some(c) = self.core_count {
            c
        } else {
            self.cpu_loads
                .iter()
                .map(|c| c.core_id)
                .max()
                .unwrap_or(0)
                .saturating_add(1)
        }
    }
}

impl fmt::Display for StressMonitoringMetricParsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "process={} pid={} cores={} fps={} latency={}",
            self.process_name,
            self.pid,
            self.effective_core_count(),
            self.fps,
            self.latency
        )
    }
}

pub fn parse_stress_metric_json(
    s: &str,
) -> Result<StressMonitoringMetricParsed, serde_json::Error> {
    serde_json::from_str(s)
}

/// MonitoringServer gRPC service handler
#[derive(Clone)]
pub struct MonitoringServerReceiver {
    pub tx_container: mpsc::Sender<ContainerList>,
    pub tx_node: mpsc::Sender<NodeInfo>,
    pub tx_stress: mpsc::Sender<String>,
}

#[tonic::async_trait]
impl MonitoringServerConnection for MonitoringServerReceiver {
    /// Handle a ContainerList message from nodeagent
    ///
    /// Receives a ContainerList from nodeagent and forwards it to the MonitoringServer manager for processing.
    async fn send_container_list<'life>(
        &'life self,
        request: Request<ContainerList>,
    ) -> Result<Response<SendContainerListResponse>, Status> {
        let req: ContainerList = request.into_inner();

        match self.tx_container.send(req).await {
            Ok(_) => Ok(tonic::Response::new(SendContainerListResponse {
                resp: "Successfully processed ContainerList".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send container list: {}", e),
            )),
        }
    }

    /// Handle a NodeInfo message from nodeagent
    ///
    /// Receives a NodeInfo from nodeagent and forwards it to the MonitoringServer manager for processing.
    async fn send_node_info<'life>(
        &'life self,
        request: Request<NodeInfo>,
    ) -> Result<Response<SendNodeInfoResponse>, Status> {
        let req: NodeInfo = request.into_inner();

        match self.tx_node.send(req).await {
            Ok(_) => Ok(tonic::Response::new(SendNodeInfoResponse {
                resp: "Successfully processed NodeInfo".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send node info: {}", e),
            )),
        }
    }

    /// Handle a StressMonitoringMetric message (single JSON string) from App Data Provider
    ///
    /// Parses the JSON payload to validate format, then forwards the original JSON string to the manager via channel.
    async fn send_stress_monitoring_metric<'life>(
        &'life self,
        request: Request<StressMonitoringMetric>,
    ) -> Result<Response<StressMonitoringMetricResponse>, Status> {
        let req: StressMonitoringMetric = request.into_inner();
        // validate JSON by parsing to struct
        parse_stress_metric_json(&req.json)
            .map_err(|e| Status::invalid_argument(format!("invalid stress metric json: {}", e)))?;

        match self.tx_stress.send(req.json).await {
            Ok(_) => Ok(Response::new(StressMonitoringMetricResponse {
                resp: "Successfully processed StressMonitoringMetric".to_string(),
            })),
            Err(e) => Err(Status::new(
                tonic::Code::Unavailable,
                format!("cannot send stress metric: {}", e),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::monitoringserver::{ContainerList, NodeInfo, StressMonitoringMetric};
    use tokio::sync::mpsc;
    use tokio::time::{timeout, Duration};
    use tonic::{Code, Request};

    fn sample_node(name: &str, ip: &str) -> NodeInfo {
        NodeInfo {
            node_name: name.to_string(),
            ip: ip.to_string(),
            cpu_usage: 42.0,
            cpu_count: 2,
            gpu_count: 1,
            used_memory: 1024,
            total_memory: 2048,
            mem_usage: 50.0,
            rx_bytes: 100,
            tx_bytes: 200,
            read_bytes: 300,
            write_bytes: 400,
            arch: "x86_64".to_string(),
            os: "linux".to_string(),
        }
    }

    fn sample_container_list(node_name: &str) -> ContainerList {
        ContainerList {
            node_name: node_name.to_string(),
            containers: vec![],
        }
    }

    fn sample_stress_json() -> String {
        r#"{
            "process_name":"example_process",
            "pid":12345,
            "core_masking":"0x0000F",
            "core_count":20,
            "fps":58.7,
            "latency":38,
            "cpu_loads":[
                {"core_id":0,"load":23.5},
                {"core_id":1,"load":45.2},
                {"core_id":2,"load":12.8}
            ]
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn test_send_container_list_success() {
        let (tx, mut rx) = mpsc::channel(1);
        let dummy_tx_node = mpsc::channel::<NodeInfo>(1).0;
        let dummy_stress = mpsc::channel::<String>(1).0;
        let receiver = MonitoringServerReceiver {
            tx_container: tx,
            tx_node: dummy_tx_node,
            tx_stress: dummy_stress,
        };
        let req = Request::new(sample_container_list("node1"));
        let resp = receiver.send_container_list(req).await.unwrap();
        assert_eq!(resp.get_ref().resp, "Successfully processed ContainerList");
        // Ensure the message was sent
        let received = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_send_container_list_failure() {
        // Drop the receiver so send will fail
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let dummy_tx = mpsc::channel(1).0;
        let dummy_stress = mpsc::channel(1).0;
        let receiver = MonitoringServerReceiver {
            tx_container: tx,
            tx_node: dummy_tx,
            tx_stress: dummy_stress,
        };
        let req = Request::new(sample_container_list("node1"));
        let resp = receiver.send_container_list(req).await;
        assert!(resp.is_err());
        let status = resp.err().unwrap();
        assert_eq!(status.code(), Code::Unavailable);
    }

    #[tokio::test]
    async fn test_send_node_info_success() {
        let (tx, mut rx) = mpsc::channel(1);
        let dummy_tx_container = mpsc::channel::<ContainerList>(1).0;
        let dummy_stress = mpsc::channel::<String>(1).0;
        let receiver = MonitoringServerReceiver {
            tx_container: dummy_tx_container,
            tx_node: tx,
            tx_stress: dummy_stress,
        };
        let req = Request::new(sample_node("node1", "192.168.10.201"));
        let resp = receiver.send_node_info(req).await.unwrap();
        assert_eq!(resp.get_ref().resp, "Successfully processed NodeInfo");
        // Ensure the message was sent
        let received = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_send_node_info_failure() {
        // Drop the receiver so send will fail
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let dummy_tx = mpsc::channel(1).0;
        let dummy_stress = mpsc::channel(1).0;
        let receiver = MonitoringServerReceiver {
            tx_container: dummy_tx,
            tx_node: tx,
            tx_stress: dummy_stress,
        };
        let req = Request::new(sample_node("node1", "192.168.10.201"));
        let resp = receiver.send_node_info(req).await;
        assert!(resp.is_err());
        let status = resp.err().unwrap();
        assert_eq!(status.code(), Code::Unavailable);
    }

    #[tokio::test]
    async fn test_send_stress_metric_success() {
        let (tx, mut rx) = mpsc::channel(1);
        let dummy_tx_container = mpsc::channel::<ContainerList>(1).0;
        let dummy_tx_node = mpsc::channel::<NodeInfo>(1).0;
        let receiver = MonitoringServerReceiver {
            tx_container: dummy_tx_container,
            tx_node: dummy_tx_node,
            tx_stress: tx,
        };
        let req = Request::new(StressMonitoringMetric {
            json: sample_stress_json(),
        });
        let resp = receiver.send_stress_monitoring_metric(req).await.unwrap();
        assert_eq!(
            resp.get_ref().resp,
            "Successfully processed StressMonitoringMetric"
        );
        let received = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_send_stress_metric_roundtrip() {
        use crate::etcd_storage;
        use crate::manager;

        // create channels: tx -> receiver, rx -> manager
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(4);
        let (tx_node, rx_node) = mpsc::channel::<NodeInfo>(4);
        let (tx_stress, rx_stress) = mpsc::channel::<String>(8);

        // create and spawn the real manager (it will consume rx_stress and call etcd)
        let mgr = manager::MonitoringServerManager::new(rx_container, rx_node, rx_stress).await;
        let mgr_handle = tokio::spawn(async move {
            // run will spawn internal tasks and block until channels are closed
            let _ = mgr.run().await;
        });

        // construct receiver with the tx side of the channels
        let receiver = MonitoringServerReceiver {
            tx_container: tx_container.clone(),
            tx_node: tx_node.clone(),
            tx_stress: tx_stress.clone(),
        };

        // send the stress metric via gRPC handler (synchronous call)
        let req = Request::new(StressMonitoringMetric {
            json: sample_stress_json(),
        });
        let resp = receiver.send_stress_monitoring_metric(req).await.unwrap();
        assert_eq!(
            resp.get_ref().resp,
            "Successfully processed StressMonitoringMetric"
        );

        // wait briefly for manager to process and store to etcd
        tokio::time::sleep(Duration::from_millis(300)).await;
        // verify etcd has at least one stress metric with expected process_name
        // NOTE: requires working etcd and correct common::etcd configuration
        let metrics = etcd_storage::get_all_stress_metrics().await;
        assert!(
            metrics.is_ok(),
            "failed to list stress metrics from etcd: {:?}",
            metrics.err()
        );
        let items = metrics.unwrap();
        let found = items.iter().any(|v| {
            v.get("process_name")
                .and_then(|s| s.as_str())
                .map(|s| s == "example_process")
                .unwrap_or(false)
        });
        assert!(
            found,
            "stored stress metric with process_name 'example_process' not found in etcd"
        );

        // cleanup: drop tx to allow manager tasks to exit, then await manager handle
        drop(tx_container);
        drop(tx_node);
        drop(tx_stress);

        // give manager a moment to finish
        let _ = tokio::time::timeout(Duration::from_secs(1), mgr_handle).await;
    }
}
