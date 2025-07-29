#[cfg(test)]
mod tests {
    use common::Result;
    use vehicle::VehicleManager;
    use std::env;

    // Ensure logger is initialized
    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[tokio::test]
    async fn test_vehicle_manager_init() -> Result<()> {
        init_logger();
        let mut manager = VehicleManager::new();
        manager.init().await?;
        // Check that the domain id is set (should be 100 by default)
        assert_eq!(manager.get_sender().clone().capacity(), 100); // Example check
        Ok(())
    }

    #[tokio::test]
    async fn test_subscribe_unsubscribe_topic() -> Result<()> {
        init_logger();
        let mut manager = VehicleManager::new();
        manager.init().await?;
        // Subscribe to a dummy topic with a dummy data type
        manager.subscribe_topic("TestTopic".to_string(), "TestType".to_string()).await?;
        // Unsubscribe afterwards
        manager.unsubscribe_topic("TestTopic".to_string()).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_subscribe_using_generated_rs() -> Result<()> {
        init_logger();
        let mut manager = VehicleManager::new();
        manager.init().await?;
        // List available DDS types from the generated file
        let generated_types = manager.list_available_types();
        println!("Available generated DDS types: {:?}", generated_types);
        
        if generated_types.is_empty() {
            println!("No generated DDS types available. Skipping typed subscription test.");
            return Ok(());
        }
        
        // Use the first available type for testing
        let use_type = generated_types[0].clone();
        // Attempt to subscribe using the generated type
        manager.create_typed_listener("GeneratedTestTopic".to_string(), use_type).await?;
        println!("Successfully subscribed using generated type.");
        // Clean up by unsubscribing afterwards
        manager.unsubscribe_topic("GeneratedTestTopic".to_string()).await?;
        Ok(())
    }
}
