/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::method_bluechi::send_dbus;
use common::etcd;
use common::statemanager::connection_server::Connection;
use common::statemanager::{Action, Response};
use ssh2::Session;
use std::error::Error;
use std::{net::TcpStream, thread, time::Duration};

const SYSTEMD_PATH: &str = "/etc/containers/systemd/";

#[derive(Default)]
pub struct StateManagerGrpcServer {}

#[tonic::async_trait]
impl Connection for StateManagerGrpcServer {
    async fn send_action(
        &self,
        request: tonic::Request<Action>,
    ) -> Result<tonic::Response<Response>, tonic::Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let req = request.into_inner();
        let command = req.action;
        println!("{}", command);

        match make_action_for_scenario(&command).await {
            Ok(v) => Ok(tonic::Response::new(Response { resp: v })),
            Err(e) => {
                println!("{:?}", e);
                Err(tonic::Status::new(tonic::Code::Unavailable, e.to_string()))
            }
        }
    }
}

async fn make_action_for_scenario(key: &str) -> Result<String, Box<dyn Error>> {
    let key_action = format!("scenario/{key}/actions");
    let key_target = format!("scenario/{key}/targets");
    let value_action = etcd::get(&key_action).await?;
    let value_target = etcd::get(&key_target).await?;
    let action: common::spec::scenario::Action = serde_yaml::from_str(&value_action)?;
    let target: common::spec::scenario::Target = serde_yaml::from_str(&value_target)?;

    let operation = &*action.get_operation();
    let target_name = target.get_name();

    let key_models = format!("package/{}/models", &target_name);
    let kvs_model = common::etcd::get_all_with_prefix(&key_models).await?;

    for kv in kvs_model {
        let model: common::spec::package::model::Model = serde_yaml::from_str(&kv.value)?;
        let model_name = model.get_name();
        let node = common::etcd::get(&format!("package/{target_name}/nodes/{model_name}")).await?;

        println!("model : {model_name}\noperation : {operation}\nnode : {node}\n");
        handle_operation(operation, &model_name, &target_name, &node).await?;
    }

    Ok(format!("Done : {}\n", operation))
}

async fn handle_operation(
    operation: &str,
    model_name: &str,
    target_name: &str,
    node_name: &str,
) -> Result<(), Box<dyn Error>> {
    match operation {
        // "launch" => {
        //     // make symlink & reload
        //     make_symlink_and_reload(node_name, model_name, target_name).await?;
        //     // start service
        //     try_service(node_name, model_name, "START").await?;
        // }
        "terminate" => {
            // stop service
            try_service(node_name, model_name, "STOP").await?;
            thread::sleep(Duration::from_secs(5));
            // delete symlink & reload
            delete_symlink_and_reload(model_name).await?;
        }
        // TODO - temporary routing. launch -> update
        "launch" | "update" | "rollback" => {
            // stop previous service
            let _ = try_service(&common::get_config().host.name, model_name, "STOP").await;
            if let Some(guests) = &common::get_config().guest {
                for guest in guests {
                    let _ = try_service(&guest.name, model_name, "STOP").await;
                }
            }
            // delete symlink & reload
            let _ = delete_symlink_and_reload(model_name).await;
            // make symlink & reload
            let _ = make_symlink_and_reload(node_name, model_name, target_name).await;
            // restart service
            let _ = try_service(node_name, model_name, "RESTART").await;
        }
        "download" => {
            println!("do something");
        }
        _ => {
            return Err("not supported operation".into());
        }
    }

    Ok(())
}

async fn delete_symlink_and_reload(model_name: &str) -> Result<(), Box<dyn Error>> {
    // host node
    let kube_symlink_path = format!("{}{}.kube", SYSTEMD_PATH, model_name);
    let _ = std::fs::remove_file(&kube_symlink_path);

    // guest node
    if let Some(guests) = &common::get_config().guest {
        for guest in guests {
            let guest_ssh_ip = format!("{}:{}", guest.ip, guest.ssh_port);
            let tcp = TcpStream::connect(guest_ssh_ip)?;

            let mut session = Session::new()?;
            session.set_tcp_stream(tcp);
            session.handshake()?;
            session.userauth_password(&guest.id, &guest.pw)?;
            if !session.authenticated() {
                println!("auth failed to remote node");
                reload_all_node().await?;
                return Err("auth failed".into());
            }

            let mut channel = session.channel_session()?;
            let command = format!("sudo rm -rf {kube_symlink_path}");
            channel.exec(&command)?;
            channel.wait_eof()?;
            channel.wait_close()?;
        }
    }

    reload_all_node().await?;
    Ok(())
}

async fn make_symlink_and_reload(
    node_name: &str,
    model_name: &str,
    target_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let original = format!(
        "{0}/packages/{1}/{2}/{2}.kube",
        common::get_config().yaml_storage,
        target_name,
        model_name
    );
    let link = format!("{}{}.kube", SYSTEMD_PATH, model_name);

    if node_name == common::get_config().host.name {
        std::os::unix::fs::symlink(original, link)?;
    } else if let Some(guests) = &common::get_config().guest {
        for guest in guests {
            if node_name != guest.name {
                continue;
            }
            let guest_ssh_ip = format!("{}:{}", guest.ip, guest.ssh_port);
            let tcp = TcpStream::connect(guest_ssh_ip)?;

            let mut session = Session::new()?;
            session.set_tcp_stream(tcp);
            session.handshake()?;
            session.userauth_password(&guest.id, &guest.pw)?;
            if !session.authenticated() {
                println!("auth failed to remote node");
                return Err("auth failed".into());
            }

            let mut channel = session.channel_session()?;
            let command = format!("sudo ln -s {original} {link}");
            channel.exec(&command)?;
            channel.wait_eof()?;
            channel.wait_close()?;
            break;
        }
    } else {
        return Err("there is no guest nodes".into());
    }

    reload_all_node().await?;
    Ok(())
}

async fn reload_all_node() -> Result<(), Box<dyn std::error::Error>> {
    send_dbus(vec!["DAEMON_RELOAD"]).await?;
    thread::sleep(Duration::from_millis(100));

    Ok(())
}

async fn try_service(
    node_name: &str,
    model_name: &str,
    act: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    send_dbus(vec![act, node_name, &format!("{}.service", model_name)]).await?;
    thread::sleep(Duration::from_millis(100));

    Ok(())
}
