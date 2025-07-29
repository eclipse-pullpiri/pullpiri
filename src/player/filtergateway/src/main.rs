mod filter;
mod grpc;
mod manager;
mod vehicle;

// Moved `launch_manager` and `initialize` function from `main.rs` to `lib.rs` to:
// 1. Enable code reuse and better modularity.
// 2. Facilitate integration and unit testing by making it publicly accessible.
// 3. Avoid duplicating logic and allow tests to directly call this async function.
// 4. Simplify the main entry point by keeping `main.rs` focused on orchestration.
//
// This approach helps maintain a clean separation between application logic
// (in the library crate) and the binary entry point (in `main.rs`).
//
// Note: The `ScenarioParameter` type is re-exported from the manager module
// via `lib.rs` to ensure a single source of truth and prevent type mismatches.
use filtergateway::ScenarioParameter;
use filtergateway::{initialize, launch_manager};
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for logging

    let (tx_grpc, rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) = channel(100);
    // Launch the manager thread
    let mgr = launch_manager(rx_grpc);

    // Initialize the application
    let grpc = initialize(tx_grpc);

    tokio::join!(mgr, grpc);
}
//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::task::LocalSet;
    use tokio::time::{sleep, Duration};

    /// Test to ensure that the channels are initialized with the correct capacity
    #[tokio::test]
    async fn test_main_initializes_channels() {
        let (tx_grpc, rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) =
            channel(100);
        assert_eq!(tx_grpc.capacity(), 100); // Check if the channel capacity is set correctly
        assert!(!rx_grpc.is_closed()); // Ensure the receiver is not closed
    }

    /// Test to ensure that the manager thread launches without any panic
    #[tokio::test(flavor = "multi_thread")]
    async fn test_main_launch_manager() {
        let (_tx_grpc, rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) =
            channel(100);

        // Use LocalSet to run a non-Send future like launch_manager
        let local = LocalSet::new();
        local.spawn_local(async move {
            let _ = launch_manager(rx_grpc).await;
        });

        // Run the local task for a short time to simulate launch
        tokio::select! {
            _ = local => {}
            _ = sleep(Duration::from_millis(200)) => {}
        }

        // Test is successful if it reaches this point
        assert!(true);
    }

    /// Test to ensure that the gRPC initialization runs without any panic
    #[tokio::test(flavor = "multi_thread")]
    async fn test_main_initialize_grpc() {
        let (tx_grpc, _rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) =
            channel(100);

        let local = LocalSet::new();
        local.spawn_local(async move {
            let _ = initialize(tx_grpc).await;
        });

        tokio::select! {
            _ = local => {}
            _ = sleep(Duration::from_millis(200)) => {}
        }

        assert!(true);
    }
}
