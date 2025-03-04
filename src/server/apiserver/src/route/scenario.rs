// SPDX-License-Identifier: Apache-2.0

use axum::{
    extract::Path,
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use common::Result;
use importer::parser::scenario::ScenarioEtcd;
use std::collections::HashMap;

pub fn get_route() -> Router {
    Router::new()
        .route("/scenario", get(list_scenario))
        .route("/scenario/:scenario_name/:file_name", get(inspect_scenario))
        .route("/scenario", post(handle_post_path))
        .route("/scenario/yaml", post(handle_post_yaml))
        .route("/scenario/:scenario_name", delete(handle_delete))
        // temporary
        .route("/scenario/reset", get(reset_all))
}

async fn list_scenario() -> Json<Vec<ScenarioInfo>> {
    use std::collections::HashSet;

    let kvs = common::etcd::get_all_with_prefix("scenario")
        .await
        .unwrap_or_default();
    let mut scenarios: Vec<ScenarioInfo> = Vec::new();

    let mut exist: HashSet<&str> = HashSet::new();
    for kv in kvs.iter() {
        let split: Vec<&str> = kv.key.split('/').collect();
        let name = match split.get(1) {
            Some(&item) => item,
            None => continue,
        };
        if !exist.insert(name) {
            continue;
        }

        let status = common::etcd::get(&format!("scenario/{name}/status"))
            .await
            .unwrap_or_default();

        let mut metric_condition = HashMap::new();
        let condition_str = common::etcd::get(&format!("scenario/{name}/condition"))
            .await
            .unwrap_or_default();
        if let Ok(condition) =
            serde_yaml::from_str::<common::spec::scenario::Condition>(&condition_str)
        {
            metric_condition.insert(
                condition.get_operand_name(),
                capitalize_first_letter(&condition.get_value()),
            );
        }

        let action = common::etcd::get(&format!("scenario/{name}/target"))
            .await
            .unwrap_or_default();

        scenarios.push(ScenarioInfo {
            name: String::from(name),
            status,
            condition: metric_condition,
            action,
        });
    }
    Json(scenarios)
}

async fn inspect_scenario(Path((_scenario_name, _file_name)): Path<(String, String)>) -> Response {
    // TODO
    super::status_ok()
}

async fn handle_post_path(body: String) -> Response {
    println!("\nPOST : scenario {body} is called.");
    let result = import_scenario_from_path(body).await;

    if let Err(msg) = result {
        super::status_err(&msg.to_string())
    } else {
        super::status_ok()
    }
}

async fn handle_post_yaml(body: String) -> Response {
    println!("\nPOST : maked scenario is called.");
    let result = import_scenario_from_yaml(body).await;

    if let Err(msg) = result {
        super::status_err(&msg.to_string())
    } else {
        super::status_ok()
    }
}

async fn import_scenario_from_path(path: String) -> Result<()> {
    let scenario = importer::get_scenario_from_file(&path).await?;
    let scenario_file = path.split('/').collect::<Vec<&str>>()[1];
    internal_import_scenario(&scenario, scenario_file).await
}

async fn import_scenario_from_yaml(yaml: String) -> Result<()> {
    let scenario = importer::get_scenario_from_yaml(&yaml).await?;
    internal_import_scenario(&scenario, &scenario.name).await
}

async fn internal_import_scenario(s: &ScenarioEtcd, file_name: &str) -> Result<()> {
    write_scenario_info_in_etcd(s, file_name).await?;
    let condition = common::filtergateway::RegisterScenarioRequest {
        scenario_name: file_name.to_string(),
    };
    crate::grpc::sender::filtergateway::send(condition).await?;

    Ok(())
}

async fn handle_delete(Path(file_name): Path<String>) -> Response {
    println!("\nDELETE : scenario {file_name} is called.");
    let result = delete_scenario(&file_name).await;

    if let Err(msg) = result {
        super::status_err(&msg.to_string())
    } else {
        super::status_ok()
    }
}

async fn delete_scenario(file_name: &str) -> Result<()> {
    delete_scenario_info_in_etcd(file_name).await?;

    let condition = common::filtergateway::RegisterScenarioRequest {
        scenario_name: file_name.to_string(),
    };
    crate::grpc::sender::filtergateway::send(condition).await?;

    Ok(())
}

async fn write_scenario_info_in_etcd(s: &ScenarioEtcd, file_name: &str) -> Result<()> {
    //let key_origin = format!("scenario/{}", s.name);
    let key_origin = format!("scenario/{}", file_name);
    //common::etcd::put(&format!("{key_origin}/file"), file_name).await?;
    common::etcd::put(&format!("{key_origin}/action"), &s.action).await?;
    common::etcd::put(&format!("{key_origin}/condition"), &s.condition).await?;
    common::etcd::put(&format!("{key_origin}/status"), "inactive").await?;
    common::etcd::put(&format!("{key_origin}/target"), &s.target).await?;

    Ok(())
}

async fn delete_scenario_info_in_etcd(name: &str) -> Result<()> {
    let key_prefix = format!("scenario/{}", name);
    common::etcd::delete_all_with_prefix(&key_prefix).await?;

    Ok(())
}

// emergency reset button
async fn reset_all() -> Response {
    // TODO - run shell script?
    super::status_ok()

    // use std::process::Command;
    // let result = Command::new("sh")
    //     .arg("-C")
    //     .arg("/etc/containers/systemd/piccolo/reset_piccolo.sh")
    //     .spawn();

    // if let Err(msg) = result {
    //     super::status_err(&msg.to_string())
    // } else {
    //     super::status_ok()
    // }
}

fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
struct ScenarioInfo {
    name: String,
    status: String,
    condition: HashMap<String, String>,
    action: String,
}
