/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use std::error::Error;

mod downloader;
mod parser;
mod old_file_handler;
mod decompress;

pub async fn handle_package(name: &str) {
    let base_url = common::get_conf("DOC_REGISTRY");
    let full_url: String = format!("{}/packages/{}.tar", base_url, name);

    let save_path: String = common::get_conf("YAML_STORAGE");
    let full_save_path = format!("{}/scenarios/{}.tar", save_path, name);

    decompress::decompress(&full_save_path);
    downloader::download(&full_url, &full_save_path);

    let parsing_path = format!("{}/scenarios/{}",save_path, name);
    let package = parser::package::package_parse(&parsing_path);
    //decompress 호출,, //경로는 일단 full_save_path에다가 그대로
    //폴더 안에 내용들 parsing해 
    //각각의 내용들을 하나의 yaml로 합치는 과정 필요.
    //합친 yaml파일로 pod.yaml, .kube파일을 systemd에 생성
    //parsing된 내용 구조체로 저장후 return

    // TODO
    // 1. download tar file (/root/piccolo_yaml/ ~~.tar)
    // 2. decompress tar file
    // 3. parsing - model, networ
    // 4. merge parsing data to yaml file
    // ***** make pod.yaml .kube
    // 4. send result (name, model, network, volume)
}

pub async fn handle_scenario(name: &str) -> Result<parser::scenario::Scenario, Box<dyn std::error::Error>> {
    let base_url = common::get_conf("DOC_REGISTRY");
    let full_url = format!("{}/scenarios/{}.yaml", base_url, name);

    let save_path: String = common::get_conf("YAML_STORAGE");
    let full_save_path = format!("{}/scenarios/{}.yaml", save_path, name);

    downloader::download(&full_url, &full_save_path);

    let scenario: Result<parser::scenario::Scenario, Box<dyn Error>> = parser::scenario::scenario_parse(&full_save_path);

    Ok(scenario?)
}

/*
#[cfg(test)]
mod tests {
    #[test]
    fn parsing_update_scenario() {
        let path = std::path::PathBuf::from(
            "/root/work/projects-rust/piccolo/examples/version-display/scenario/update-scenario.yaml",
        );

        let result = crate::parser::scenario_parse(&path);
        println!("{:#?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn parsing_rollback_scenario() {
        let path = std::path::PathBuf::from(
            "/root/work/projects-rust/piccolo/examples/version-display/scenario/rollback-scenario.yaml",
        );

        let result = crate::parser::scenario_parse(&path);
        println!("{:#?}", result);
        assert!(result.is_ok());
    }
}
*/

/*
use crate::file_handler;
use crate::grpc::sender::apiserver;
use crate::parser;
use common::apiserver::scenario::Scenario;
use common::yamlparser::connection_server::Connection;
use common::yamlparser::{SendRequest, SendResponse};
use tonic::{Code, Request, Response, Status};

#[derive(Default)]
pub struct YamlparserGrpcServer {}

#[tonic::async_trait]
impl Connection for YamlparserGrpcServer {
    async fn send(&self, request: Request<SendRequest>) -> Result<Response<SendResponse>, Status> {
        let req = request.into_inner();
        let scenario = match handle_msg(&req.request) {
            Ok(scenario) => scenario,
            Err(e) => return Err(Status::new(Code::InvalidArgument, e.to_string())),
        };

        match apiserver::send_msg_to_apiserver(scenario).await {
            Ok(response) => Ok(tonic::Response::new(SendResponse {
                response: response.into_inner().resp,
            })),
            Err(e) => Err(Status::new(Code::Unavailable, e.to_string())),
        }
    }
}

pub fn handle_msg(path: &str) -> Result<Scenario, Box<dyn std::error::Error>> {
    let absolute_path = file_handler::get_absolute_file_path(path)?;
    let scenario = parser::scenario_parse(&absolute_path)?;
    Ok(scenario)
}
*/
