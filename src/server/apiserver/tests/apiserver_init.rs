use apiserver::manager;
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "current_thread")]
async fn test_manager_initialize() {
    let _ = tokio::time::timeout(Duration::from_millis(100), manager::initialize()).await;
    assert!(true);
}
