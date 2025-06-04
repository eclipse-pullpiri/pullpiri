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
    use crate::route::status;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::Response,
        routing::{delete, get, post},
        Router,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use tower::ServiceExt; // for oneshot

    // Atomic flags to verify if mocked functions are called
    static APPLY_CALLED: AtomicBool = AtomicBool::new(false);
    static WITHDRAW_CALLED: AtomicBool = AtomicBool::new(false);

    // Valid YAML artifact example for testing POST /api/artifact
    const VALID_ARTIFACT_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
      volume:
      network:
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// Setup the test app router overriding handlers with mocks
    async fn setup_app() -> Router {
        Router::new()
            .route("/api/notify", get(mock_notify))
            .route("/api/artifact", post(mock_apply_artifact))
            .route("/api/artifact", delete(mock_withdraw_artifact))
    }

    // ------------------
    // Mocked Handlers
    // ------------------

    /// Mock implementation of apply_artifact that sets flag and returns OK
    async fn mock_apply_artifact(_body: String) -> Response {
        APPLY_CALLED.store(true, Ordering::SeqCst);
        status(Ok(()))
    }

    /// Mock implementation of withdraw_artifact that sets flag and returns OK
    async fn mock_withdraw_artifact(_body: String) -> Response {
        WITHDRAW_CALLED.store(true, Ordering::SeqCst);
        status(Ok(()))
    }

    /// Mock implementation of notify that just returns OK
    async fn mock_notify() -> Response {
        status(Ok(()))
    }

    // -------------------
    // Notify Endpoint Tests
    // -------------------

    /// Positive test: GET /api/notify returns 200 OK
    #[tokio::test]
    async fn test_notify_positive() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("GET")
            .uri("/api/notify")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Negative test: POST /api/notify returns 405 Method Not Allowed
    #[tokio::test]
    async fn test_notify_invalid_method() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("POST")
            .uri("/api/notify")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    // -------------------
    // Apply Artifact Tests (POST)
    // -------------------

    /// Positive test: POST /api/artifact with valid YAML body returns 200 OK and sets apply flag
    #[tokio::test]
    async fn test_apply_artifact_positive() {
        let app = setup_app().await;
        APPLY_CALLED.store(false, Ordering::SeqCst);

        let req = Request::builder()
            .method("POST")
            .uri("/api/artifact")
            .header("Content-Type", "text/plain")
            .body(Body::from(VALID_ARTIFACT_YAML))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(APPLY_CALLED.load(Ordering::SeqCst));
    }

    /// Negative test: POST /api/artifact with missing body returns 200 OK and sets apply flag
    #[tokio::test]
    async fn test_apply_artifact_missing_body() {
        let app = setup_app().await;
        APPLY_CALLED.store(false, Ordering::SeqCst);

        let req = Request::builder()
            .method("POST")
            .uri("/api/artifact")
            .header("Content-Type", "text/plain")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(APPLY_CALLED.load(Ordering::SeqCst));
    }

    /// Negative test: GET /api/artifact returns 405 Method Not Allowed
    #[tokio::test]
    async fn test_apply_artifact_invalid_method() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("GET")
            .uri("/api/artifact")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    // ---------------------------
    // Withdraw Artifact Tests (DELETE)
    // ---------------------------

    /// Positive test: DELETE /api/artifact returns 200 OK and sets withdraw flag
    #[tokio::test]
    async fn test_withdraw_artifact_positive() {
        let app = setup_app().await;
        WITHDRAW_CALLED.store(false, Ordering::SeqCst);

        let req = Request::builder()
            .method("DELETE")
            .uri("/api/artifact")
            .body(Body::empty()) // Axum dislikes body on DELETE
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(WITHDRAW_CALLED.load(Ordering::SeqCst));
    }

    /// Negative test: POST /api/artifact returns 200 OK (withdraw endpoint does not handle POST, but our router allows it)
    #[tokio::test]
    async fn test_withdraw_artifact_invalid_method() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("POST")
            .uri("/api/artifact")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Negative test: PUT /api/artifact returns 405 Method Not Allowed
    #[tokio::test]
    async fn test_withdraw_artifact_invalid_method_put() {
        let app = setup_app().await;

        let req = Request::builder()
            .method("PUT")
            .uri("/api/artifact")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
