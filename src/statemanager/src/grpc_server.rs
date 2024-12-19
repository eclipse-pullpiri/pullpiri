/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::etcd;
use common::statemanager::connection_server::Connection;
use common::statemanager::{Action, Response};
use ssh2::Session;
use std::error::Error;
use std::{net::TcpStream, thread, time::Duration};

type BluechiSender = tokio::sync::mpsc::Sender<crate::bluechi::BluechiCmd>;
type Command = crate::bluechi::Command;
const SYSTEMD_PATH: &str = "/etc/containers/systemd/";

pub struct StateManagerGrpcServer {
    pub tx: BluechiSender,
}

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

        match self.make_action_for_scenario(&command).await {
            Ok(v) => Ok(tonic::Response::new(Response { resp: v })),
            Err(e) => {
                println!("{:?}", e);
                Err(tonic::Status::new(tonic::Code::Unavailable, e.to_string()))
            }
        }
    }
}

impl StateManagerGrpcServer {
    async fn make_action_for_scenario(&self, key: &str) -> Result<String, Box<dyn Error>> {
        let key_action = format!("scenario/{key}/action");
        let key_target = format!("scenario/{key}/target");
        let action = etcd::get(&key_action).await?;
        let target = etcd::get(&key_target).await?;

        let key_models = format!("package/{}/models", target);
        let kvs_model = common::etcd::get_all_with_prefix(&key_models).await?;

        for kv in kvs_model {
            let model_name = &kv.value;
            let node = common::etcd::get(&format!("package/{target}/nodes/{model_name}")).await?;

            println!("model : {model_name}\noperation : {action}\nnode : {node}\n");
            self.handle_operation(&action, model_name, &target, &node)
                .await?;
        }

        Ok(format!("Done : {}\n", action))
    }

    async fn handle_operation(
        &self,
        operation: &str,
        model_name: &str,
        target_name: &str,
        node_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        match operation {
            "launch" => {
                // make symlink & reload
                self.make_symlink_and_reload(node_name, model_name, target_name)
                    .await?;
                // start service
                self.try_service(node_name, model_name, Command::UnitStart)
                    .await?;
            }
            "terminate" => {
                // stop service
                self.try_service(node_name, model_name, Command::UnitStop)
                    .await?;
                thread::sleep(Duration::from_secs(5));
                // delete symlink & reload
                self.delete_symlink_and_reload(model_name).await?;
            }
            "update" | "rollback" => {
                // stop previous service
                let _ = self
                    .try_service(
                        &common::get_config().host.name,
                        model_name,
                        Command::UnitStop,
                    )
                    .await;
                if let Some(guests) = &common::get_config().guest {
                    for guest in guests {
                        let _ = self
                            .try_service(&guest.name, model_name, Command::UnitStop)
                            .await;
                    }
                }
                // delete symlink & reload
                let _ = self.delete_symlink_and_reload(model_name).await;
                // make symlink & reload
                let _ = self
                    .make_symlink_and_reload(node_name, model_name, target_name)
                    .await;
                // restart service
                let _ = self
                    .try_service(node_name, model_name, Command::UnitRestart)
                    .await;
            }
            "download" => {
                println!("TBD - do something");
            }
            _ => {
                return Err("not supported operation".into());
            }
        }

        Ok(())
    }

    async fn delete_symlink_and_reload(&self, model_name: &str) -> Result<(), Box<dyn Error>> {
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
                    self.reload_all_node().await?;
                    return Err("auth failed".into());
                }

                let mut channel = session.channel_session()?;
                let command = format!("sudo rm -rf {kube_symlink_path}");
                channel.exec(&command)?;
                channel.wait_eof()?;
                channel.wait_close()?;
            }
        }

        self.reload_all_node().await?;
        Ok(())
    }

    async fn make_symlink_and_reload(
        &self,
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

        self.reload_all_node().await?;
        Ok(())
    }

    async fn reload_all_node(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cmd = crate::bluechi::BluechiCmd {
            command: Command::ControllerReloadAllNodes,
            node: None,
            unit: None,
        };
        self.tx.send(cmd).await?;
        thread::sleep(Duration::from_millis(100));

        Ok(())
    }

    async fn try_service(
        &self,
        node_name: &str,
        model_name: &str,
        act: Command,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cmd = crate::bluechi::BluechiCmd {
            command: act,
            node: Some(node_name.to_string()),
            unit: Some(format!("{}.service", model_name)),
        };
        self.tx.send(cmd).await?;
        thread::sleep(Duration::from_millis(100));

        Ok(())
    }
}
