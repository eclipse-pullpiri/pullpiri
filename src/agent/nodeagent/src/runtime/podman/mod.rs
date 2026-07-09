/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

pub mod container;

use common::nodeagent::fromactioncontroller::WorkloadCommand;
use hyper::{Body, Client, Method, Request, Uri};
use hyperlocal::{UnixConnector, Uri as UnixUri};
use once_cell::sync::Lazy;

// Modify this if you want to run without root authorization
// or if you have a different socket path.
// For example, if you run Podman as root, you might use:
// "/var/run/podman/podman.sock"
// Or if you run it as a user, you might use:
// "/run/user/1000/podman/podman.sock"
const PODMAN_SOCKET: &str = "/var/run/podman/podman.sock";

// A single `hyper::Client` is cheap to clone and manages its own connection
// pool internally, so it is created once and reused for every request
// instead of being rebuilt on each call to `get`/`post`/`delete`.
static PODMAN_CLIENT: Lazy<Client<UnixConnector, Body>> =
    Lazy::new(|| Client::builder().build::<_, Body>(UnixConnector));

pub async fn get(path: &str) -> Result<hyper::body::Bytes, hyper::Error> {
    let uri: Uri = UnixUri::new(PODMAN_SOCKET, path).into();

    let res = PODMAN_CLIENT.get(uri).await?;
    hyper::body::to_bytes(res).await
}

pub async fn post(path: &str, body: Body) -> Result<hyper::body::Bytes, hyper::Error> {
    let uri: Uri = UnixUri::new(PODMAN_SOCKET, path).into();

    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .body(body)
        .unwrap();

    let res = PODMAN_CLIENT.request(req).await?;
    hyper::body::to_bytes(res).await
}

pub async fn delete(path: &str) -> Result<hyper::body::Bytes, hyper::Error> {
    let uri: Uri = UnixUri::new(PODMAN_SOCKET, path).into();

    let req = Request::builder()
        .method(Method::DELETE)
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let res = PODMAN_CLIENT.request(req).await?;
    hyper::body::to_bytes(res).await
}

pub async fn handle_workload(
    command: i32,
    pod: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    println!(
        "handle_workload called with command: {} for model(pod)",
        command
    );
    match command {
        x if x == WorkloadCommand::Start as i32 => {
            let container_ids = container::start(pod).await?;
            return Ok(container_ids);
        }
        x if x == WorkloadCommand::Stop as i32 => {
            container::stop(pod).await?;
        }
        x if x == WorkloadCommand::Restart as i32 => {
            container::restart(pod).await?;
        }
        _ => {
            // Do nothing for unimplemented commands
            return Err("unimplemented command".into());
        }
    };

    Ok(vec![])
}

//Unit tets cases
#[cfg(test)]
mod tests {
    use super::get;
    use hyper::body::Bytes;
    use hyper::Error;
    use tokio;

    #[tokio::test]
    async fn test_get_with_valid_path() {
        let result: Result<Bytes, Error> = get("/v1.0/version").await;
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }
}
