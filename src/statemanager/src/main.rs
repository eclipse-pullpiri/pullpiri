/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

mod grpc_server;
mod method_bluechi;

use crate::grpc_server::StateManagerGrpcServer;
use common::statemanager::connection_server::ConnectionServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() {
    let addr = common::statemanager::open_server()
        .parse()
        .expect("statemanager address parsing error");
    let state_manager_grpc_server = StateManagerGrpcServer::default();

    println!("Piccolod statemanager listening on {}", addr);

    let _ = Server::builder()
        .add_service(ConnectionServer::new(state_manager_grpc_server))
        .serve(addr)
        .await;
}

#[cfg(test)]
mod tests {
    use ssh2::Session;
    use std::path::Path;
    use std::{fs, io};

    #[test]
    fn test_ssh2_copy_foler() -> Result<(), Box<dyn std::error::Error>> {
        let tcp = std::net::TcpStream::connect("192.168.10.11:22")?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();
        session.userauth_password("id", "pw").unwrap();
        assert!(session.authenticated());

        let local_folder = Path::new("/root/demo/");
        let remote_folder = "/root/demo/";
        upload_folder(&session, local_folder, remote_folder)?;

        Ok(())
    }

    fn upload_folder(session: &Session, local_path: &Path, remote_path: &str) -> io::Result<()> {
        for entry in fs::read_dir(local_path)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let remote_file_path = format!("{}/{}", remote_path, file_name.to_string_lossy());
            let rfp = Path::new(&remote_file_path);
            if path.is_dir() {
                session.sftp()?.mkdir(rfp, 0o755)?;
                upload_folder(session, &path, &remote_file_path)?;
            } else {
                let mut remote_file = session.sftp()?.create(&rfp)?;
                let mut local_file = fs::File::open(&path)?;
                io::copy(&mut local_file, &mut remote_file)?;
            }
        }
        Ok(())
    }
}
