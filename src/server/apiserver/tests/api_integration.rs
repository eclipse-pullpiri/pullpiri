use apiserver::route::api::router;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

// Correct multi-document YAML artifact (Scenario + Package + Model)
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

// Test: GET /api/notify with artifact_name query param
#[tokio::test]
async fn test_notify_with_query_param() {
    let app = router();

    let req = Request::builder()
        .method("GET")
        .uri("/api/notify?artifact_name=helloworld")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// Test: GET /api/notify without query param (should still succeed)
#[tokio::test]
async fn test_notify_without_query_param() {
    let app = router();

    let req = Request::builder()
        .method("GET")
        .uri("/api/notify")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// Test: POST method on /api/notify (not allowed)
#[tokio::test]
async fn test_notify_invalid_method_post() {
    let app = router();

    let req = Request::builder()
        .method("POST")
        .uri("/api/notify")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// Test: POST /api/artifact with valid multi-document YAML
#[tokio::test]
async fn test_apply_artifact_valid_body() {
    let app = router();

    let req = Request::builder()
        .method("POST")
        .uri("/api/artifact")
        .header("Content-Type", "text/plain")
        .body(Body::from(VALID_ARTIFACT_YAML))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// Test: POST /api/artifact with empty body (should be rejected)
#[tokio::test]
async fn test_apply_artifact_missing_body() {
    let app = router();

    let req = Request::builder()
        .method("POST")
        .uri("/api/artifact")
        .header("Content-Type", "text/plain")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED); // Manager likely returns an error for empty string
}

// Test: GET /api/artifact (method not allowed)
#[tokio::test]
async fn test_apply_artifact_invalid_method_get() {
    let app = router();

    let req = Request::builder()
        .method("GET")
        .uri("/api/artifact")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}

//Test: DELETE /api/artifact with valid artifact body
#[tokio::test]
async fn test_withdraw_artifact_valid_body() {
    let app = router();

    let req = Request::builder()
        .method("DELETE")
        .uri("/api/artifact")
        .body(Body::from(VALID_ARTIFACT_YAML)) // even if it's full YAML, it tests general body handling
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// Test: DELETE /api/artifact with empty body (should be rejected)
#[tokio::test]
async fn test_withdraw_artifact_empty_body() {
    let app = router();

    let req = Request::builder()
        .method("DELETE")
        .uri("/api/artifact")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// Test: PUT /api/artifact (not allowed method)
#[tokio::test]
async fn test_withdraw_artifact_invalid_method_put() {
    let app = router();

    let req = Request::builder()
        .method("PUT")
        .uri("/api/artifact")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}
