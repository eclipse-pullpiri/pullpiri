// SPDX-License-Identifier: Apache-2.0

use axum::{
    response::Response,
    routing::{delete, get, post},
    Router,
};

pub fn router() -> Router {
    Router::new()
        .route("/api/notify", get(notify))
        .route("/api/artifact", post(apply_artifact))
        .route("/api/artifact", delete(withdraw_artifact))
}

/// Notify of new artifact release in the cloud
///
/// ### Parametets
/// * `artifact_name` - name of the newly released artifact
async fn notify(artifact_name: String) -> Response {
    println!("{}", artifact_name);
    super::status_ok()
}

/// Apply the new artifacts (scenario, package, etc...)
///
/// ### Parameters
/// * `body` - the string in yaml format
async fn apply_artifact(body: String) -> Response {
    let result = crate::manager::apply_artifact(&body).await;

    if let Err(msg) = result {
        super::status_err(&msg.to_string())
    } else {
        super::status_ok()
    }
}

/// Withdraw the applied scenario
///
/// ### Parameters
/// * `artifact_name` - name of the artifact to be deleted
async fn withdraw_artifact(body: String) -> Response {
    let result = crate::manager::withdraw_artifact(&body).await;

    if let Err(msg) = result {
        super::status_err(&msg.to_string())
    } else {
        super::status_ok()
    }
}
