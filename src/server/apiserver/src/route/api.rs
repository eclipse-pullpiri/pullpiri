// SPDX-License-Identifier: Apache-2.0

use axum::{
    response::Response,
    routing::{delete, get, post},
    Router,
};

pub fn get_route() -> Router {
    Router::new()
        .route("/api/notify", get(notify))
        .route("/api/artifact", post(apply_artifact))
        .route("/api/artifact", delete(withdraw_artifact))
}

/// Notify of new artifact release in the cloud
///
/// # parametets
/// * `artifact_name` - name of the newly released artifact
async fn notify(_artifact_name: String) -> Response {
    super::status_ok()
}

/// Apply the new artifacts (scenario, package, etc...)
///
/// # parameters
/// * `body` - the string in yaml format
async fn apply_artifact(_body: String) -> Response {
    super::status_ok()
}

/// Withdraw the applied scenario
///
/// # parameters
/// * `artifact_name` - name of the artifact to be deleted
async fn withdraw_artifact(_artifact_name: String) -> Response {
    super::status_err("Not implemented")
}
