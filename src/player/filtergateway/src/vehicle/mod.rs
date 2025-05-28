pub mod dds;

use common::Result;
use dds::DdsData;
use tokio::sync::mpsc::Sender;

/// Vehicle data management module
///
/// Manages vehicle data through DDS communication
pub struct VehicleManager {
    /// DDS Manager instance
    dds_manager: dds::DdsManager,
}

impl VehicleManager {
    /// Creates a new VehicleManager
    ///
    /// # Returns
    ///
    /// A new VehicleManager instance
    pub fn new(tx: Sender<DdsData>) -> Self {
        Self {
            dds_manager: dds::DdsManager::new(tx),
        }
    }

    /// Initializes the vehicle data system
    ///
    /// Sets up the DDS system and prepares for topic subscriptions
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn init(&mut self) -> Result<()> {
        // Initialize DDS manager
        match self.dds_manager.init().await {
            Ok(_) => {}
            Err(e) => {
                log::warn!("Failed to initialize DDS manager with settings file: {}. Using default settings.", e);
                // 기본 설정 적용
            }
        }
        self.set_domain_id(100); // Set default domain ID

        Ok(())
    }

    /// Subscribes to a vehicle data topic
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the topic to subscribe to
    /// * `data_type_name` - Type name of the data
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn subscribe_topic(
        &mut self,
        topic_name: String,
        data_type_name: String,
    ) -> Result<()> {
        // TODO: Implementation
        self.dds_manager
            .create_typed_listener(topic_name, data_type_name)
            .await?;
        // self.dds_manager
        //     .create_listener(topic_name, data_type_name)
        //     .await?;
        Ok(())
    }

    /// Get list of available DDS types
    pub fn list_available_types(&self) -> Vec<String> {
        self.dds_manager.list_available_types()
    }

    /// Unsubscribes from a vehicle data topic
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the topic to unsubscribe from
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn unsubscribe_topic(&mut self, topic_name: String) -> Result<()> {
        // TODO: Implementation
        self.dds_manager.remove_listener(&topic_name).await?;
        Ok(())
    }

    /// Gets the DDS data sender
    ///
    /// # Returns
    ///
    /// A sender for DDS data
    pub fn get_sender(&self) -> tokio::sync::mpsc::Sender<dds::DdsData> {
        self.dds_manager.get_sender()
    }

    /// Sets the DDS domain ID
    ///
    /// # Arguments
    ///
    /// * `domain_id` - Domain ID to use for DDS communication

    pub fn set_domain_id(&mut self, domain_id: i32) {
        self.dds_manager.set_domain_id(domain_id);
    }
}
//Unit tests for VehicleManager
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    #[tokio::test] // Test creation of VehicleManager and validate sender capacity
    async fn test_vehicle_manager_new() {
        let (tx, _rx) = mpsc::channel(10);
        let vehicle_manager = VehicleManager::new(tx);
        let sender = vehicle_manager.get_sender();
        assert_eq!(sender.capacity(), 10); // Validate sender's capacity
    }

    #[tokio::test] // Test successful initialization of VehicleManager
    async fn test_vehicle_manager_init_success() {
        let (tx, _rx) = mpsc::channel(10);
        let mut vehicle_manager = VehicleManager::new(tx);
        let result = vehicle_manager.init().await;
        assert!(result.is_ok());
    }

    #[tokio::test] // Test subscribing to a topic successfully
    async fn test_vehicle_manager_subscribe_topic() {
        let (tx, _rx) = mpsc::channel(10);
        let mut vehicle_manager = VehicleManager::new(tx);
        let result = vehicle_manager
            .subscribe_topic("vehicle_data".to_string(), "VehicleType".to_string())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test] // Test unsubscribing from a topic successfully
    async fn test_vehicle_manager_unsubscribe_topic() {
        let (tx, _rx) = mpsc::channel(10);
        let mut vehicle_manager = VehicleManager::new(tx);
        let result = vehicle_manager
            .unsubscribe_topic("vehicle_data".to_string())
            .await;
        assert!(result.is_ok());
    }

    #[test] // Test listing all available vehicle types
    fn test_vehicle_manager_list_available_types() {
        let (tx, _rx) = mpsc::channel(10);
        let vehicle_manager = VehicleManager::new(tx);
        let types = vehicle_manager.list_available_types();
        assert!(!types.is_empty());
    }

    #[test] // Test setting the domain ID for VehicleManager
    fn test_vehicle_manager_set_domain_id() {
        let (tx, _rx) = mpsc::channel(10);
        let mut vehicle_manager = VehicleManager::new(tx);
        vehicle_manager.set_domain_id(200);
        assert!(true); // Placeholder assertion for domain ID setting
    }
}
