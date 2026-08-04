/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::external::timpani::fault_service_server::FaultService;
use common::external::timpani::{FaultInfo, FaultType, Response as TimpaniResponse};
use common::policymanager::{FaultType as PolicyFaultType, ReportFaultRequest};
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct TimpaniReceiver {}

#[tonic::async_trait]
impl FaultService for TimpaniReceiver {
    async fn notify_fault(
        &self,
        info: Request<FaultInfo>,
    ) -> Result<Response<TimpaniResponse>, Status> {
        let info = info.into_inner();
        common::logd!(
            4,
            "[StateManager] Received fault notification from Timpani: workload={}, node={}, task={}, type={:?}",
            info.workload_id,
            info.node_id,
            info.task_name,
            info.r#type
        );

        // Convert Timpani FaultType to PolicyManager FaultType
        let fault_type = match FaultType::try_from(info.r#type) {
            Ok(FaultType::Dmiss) => PolicyFaultType::FaultDeadlineMiss,
            _ => PolicyFaultType::FaultUnknown,
        };

        // Forward to PolicyManager
        let request = ReportFaultRequest {
            workload_id: info.workload_id.clone(),
            node_id: info.node_id.clone(),
            task_name: info.task_name.clone(),
            fault_type: fault_type as i32,
        };

        match crate::grpc::sender::report_fault_to_policymanager(request).await {
            Ok(response) => {
                let resp = response.into_inner();
                common::logd!(
                    3,
                    "[StateManager] Forwarded fault to PolicyManager: processed={}, message={}",
                    resp.processed,
                    resp.message
                );
            }
            Err(e) => {
                common::logd!(
                    5,
                    "[StateManager] Failed to forward fault to PolicyManager: {}",
                    e.message()
                );
            }
        }

        // Process the fault information and generate a response
        let response = TimpaniResponse { status: 0 };
        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_notify_fault_returns_success() {
        // Set test mode to bypass actual PolicyManager connection
        std::env::set_var("PULLPIRI_TEST_MODE", "1");

        let receiver = TimpaniReceiver::default();

        // Use default FaultInfo (prost types implement Default)
        let info = FaultInfo::default();

        let req = Request::new(info);
        let resp = receiver.notify_fault(req).await;

        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.get_ref().status, 0);

        std::env::remove_var("PULLPIRI_TEST_MODE");
    }

    #[tokio::test]
    async fn test_notify_fault_concurrent_calls() {
        // Set test mode to bypass actual PolicyManager connection
        std::env::set_var("PULLPIRI_TEST_MODE", "1");

        let receiver = TimpaniReceiver::default();

        // Spawn multiple concurrent notify_fault calls to ensure no panics and consistent responses
        let mut handles = Vec::new();
        for _ in 0..8 {
            handles.push(tokio::spawn(async move {
                let r = TimpaniReceiver::default();
                let info = FaultInfo::default();
                let req = Request::new(info);
                let res = r.notify_fault(req).await;
                res
            }));
        }

        for h in handles {
            let res = h.await.expect("task panicked");
            assert!(res.is_ok());
            let out = res.unwrap();
            assert_eq!(out.get_ref().status, 0);
        }

        std::env::remove_var("PULLPIRI_TEST_MODE");
    }
}
