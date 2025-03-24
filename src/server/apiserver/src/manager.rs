use common::filtergateway::{Action, HandleScenarioRequest};

pub async fn initialize() {
    tokio::join!(
        crate::route::launch_tcp_listener(),
        crate::artifact::data::reload()
    );
}

async fn send_download_request() {}

/// Apply downloaded artifact
///
/// # parametets
/// * `body` - whole yaml string of piccolo artifact
/// # description
/// write artifact in etcd
/// -> (optional) make yaml, kube files for Bluechi
/// -> send a gRPC message to gateway
pub async fn apply_artifact(body: &str) -> common::Result<()> {
    let scenario = crate::artifact::apply(body).await?;

    let req = HandleScenarioRequest {
        action: Action::Apply.into(),
        scenario,
    };
    crate::grpc::sender::filtergateway::send(req).await?;

    Ok(())
}

/// Withdraw downloaded artifact
///
/// # parametets
/// * `body` - whole yaml string of piccolo artifact
/// # description
/// delete artifact in etcd
/// -> (optional) delete yaml, kube files for Bluechi
/// -> send a gRPC message to gateway
pub async fn withdraw_artifact(body: &str) -> common::Result<()> {
    let scenario = crate::artifact::withdraw(body).await?;

    let req = HandleScenarioRequest {
        action: Action::Withdraw.into(),
        scenario,
    };
    crate::grpc::sender::filtergateway::send(req).await?;

    Ok(())
}
