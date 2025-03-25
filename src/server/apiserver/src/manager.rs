//! Controls the flow of data between each module.

use common::filtergateway::{Action, HandleScenarioRequest};

pub async fn initialize() {
    tokio::join!(crate::route::launch_tcp_listener(), reload());
}

/// Send request message to piccolo cloud
///
/// ### Parametets
/// * `name` - piccolo scenario name
/// ### Description
/// TODO
async fn send_download_request() {}

/// Reload all scenario data in etcd
///
/// ### Parametets
/// * None
/// ### Description
/// This function is called once when the apiserver starts.
async fn reload() {
    let scenarios_result = crate::artifact::data::read_all_scenario_from_etcd().await;

    if let Ok(scenarios) = scenarios_result {
        for scenario in scenarios {
            let req = HandleScenarioRequest {
                action: Action::Apply.into(),
                scenario,
            };
            if let Err(status) = crate::grpc::sender::filtergateway::send(req).await {
                println!("{:#?}", status);
            }
        }
    } else {
        println!("{:#?}", scenarios_result);
    }
}

/// Apply downloaded artifact
///
/// ### Parametets
/// * `body` - whole yaml string of piccolo artifact
/// ### Description
/// write artifact in etcd  
/// (optional) make yaml, kube files for Bluechi  
/// send a gRPC message to gateway
pub async fn apply_artifact(body: &str) -> common::Result<()> {
    let (scenario, package) = crate::artifact::apply(body).await?;

    crate::bluechi::parse(package).await?;

    let req = HandleScenarioRequest {
        action: Action::Apply.into(),
        scenario,
    };
    crate::grpc::sender::filtergateway::send(req).await?;

    Ok(())
}

/// Withdraw downloaded artifact
///
/// ### Parametets
/// * `body` - whole yaml string of piccolo artifact
/// ### Description
/// delete artifact in etcd  
/// (optional) delete yaml, kube files for Bluechi  
/// send a gRPC message to gateway
pub async fn withdraw_artifact(body: &str) -> common::Result<()> {
    let scenario = crate::artifact::withdraw(body).await?;

    let req = HandleScenarioRequest {
        action: Action::Withdraw.into(),
        scenario,
    };
    crate::grpc::sender::filtergateway::send(req).await?;

    Ok(())
}
