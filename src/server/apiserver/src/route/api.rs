/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Handler functions of Piccolo REST API

use axum::{
    response::Response,
    routing::{delete, get, post},
    Router,
};

/// Make router type for composing handler and Piccolo service
///
/// ### Parametets
/// None
pub fn router() -> Router {
    Router::new()
        .route("/api/notify", get(notify))
        .route("/api/artifact", post(apply_artifact))
        .route("/api/artifact", delete(withdraw_artifact))
}

/// Notify of new artifact release in the cloud
///
/// ### Parametets
/// * `artifact_name: String` - name of the newly released artifact
async fn notify(artifact_name: String) -> Response {
    println!("{}", artifact_name);

    super::status(Ok(()))
}

/// Apply the new artifacts (scenario, package, etc...)
///
/// ### Parameters
/// * `body: String` - the string in yaml format
async fn apply_artifact(body: String) -> Response {
    let result = crate::manager::apply_artifact(&body).await;

    super::status(result)
}

/// Withdraw the applied scenario
///
/// ### Parameters
/// * `body: String` - name of the artifact to be deleted
async fn withdraw_artifact(body: String) -> Response {
    let result = crate::manager::withdraw_artifact(&body).await;

    super::status(result)
}
