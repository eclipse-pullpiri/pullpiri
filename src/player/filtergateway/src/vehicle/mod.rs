pub mod dds;

use common::Result;

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
    pub fn new() -> Self {
        Self {
            dds_manager: dds::DdsManager::new(),
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
        // TODO: Implementation
        self.dds_manager = dds::DdsManager::new();
        self.set_domain_id(100); // Set default domain ID
                                 // Initialize DDS system

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
            .create_listener(topic_name, data_type_name)
            .await?;
        Ok(())
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
    pub fn set_domain_id(&mut self, domain_id: u32) {
        self.dds_manager.set_domain_id(domain_id);
    }
}
