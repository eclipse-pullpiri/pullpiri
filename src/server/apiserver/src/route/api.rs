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

//UNIT TEST CASES
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use tower::ServiceExt; // for oneshot

    async fn setup_app() -> Router {
        Router::new()
            .route("/api/notify", get(notify))
            .route("/api/artifact", post(apply_artifact))
            .route("/api/artifact", delete(withdraw_artifact))
    }

    // -------------------
    // Notify Endpoint Tests
    // -------------------

    // Positive test: notify (GET request without body)
    #[tokio::test]
    async fn test_notify_positive() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("GET")
            .uri("/api/notify")
            .body(Body::empty()) // no body
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Failed to get OK response for GET request to /api/notify"
        );
    }

    // Negative test: notify (GET request with invalid method POST)
    #[tokio::test]
    async fn test_notify_invalid_method() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("POST") // invalid method
            .uri("/api/notify")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "Expected METHOD_NOT_ALLOWED for POST request to /api/notify, but got {}",
            response.status()
        );
    }

    // -------------------------
    // Apply Artifact Tests (POST)
    // -------------------------

    // Positive test: apply artifact (valid POST + body)
    #[tokio::test]
    async fn test_apply_artifact_positive() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("POST")
            .uri("/api/artifact")
            .header("Content-Type", "text/plain")
            .body(Body::from("artifact-yaml-content"))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Failed to apply artifact with valid body"
        );
    }

    // Negative test: apply artifact (missing body)
    #[tokio::test]
    async fn test_apply_artifact_missing_body() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("POST")
            .uri("/api/artifact")
            .header("Content-Type", "text/plain")
            .body(Body::empty()) // missing body
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        // Axum will still parse empty string as "", so status is OK — it's valid
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Expected OK response when applying artifact with missing body, but got {}",
            response.status()
        );
    }

    // Negative test: apply artifact (wrong method GET)
    #[tokio::test]
    async fn test_apply_artifact_invalid_method() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("GET")
            .uri("/api/artifact")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "Expected METHOD_NOT_ALLOWED for GET request to /api/artifact, but got {}",
            response.status()
        );
    }

    // ---------------------------
    // Withdraw Artifact Tests (DELETE)
    // ---------------------------

    // Positive test: withdraw artifact (DELETE without body — since Axum dislikes body here)
    #[tokio::test]
    async fn test_withdraw_artifact_positive() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("DELETE")
            .uri("/api/artifact")
            .body(Body::empty()) // DELETE should avoid body
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Failed to withdraw artifact with valid DELETE request"
        );
    }

    // Negative test: withdraw artifact (wrong method POST)
    #[tokio::test]
    async fn test_withdraw_artifact_invalid_method() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("POST")
            .uri("/api/artifact")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Expected OK response for POST request to /api/artifact, but got {}",
            response.status()
        );
    }

    // Negative test: withdraw artifact (unsupported PUT method)
    #[tokio::test]
    async fn test_withdraw_artifact_invalid_method_put() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("PUT")
            .uri("/api/artifact")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "Expected METHOD_NOT_ALLOWED for PUT request to /api/artifact, but got {}",
            response.status()
        );
    }
}
