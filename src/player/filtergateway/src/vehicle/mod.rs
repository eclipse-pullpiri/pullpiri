pub mod dds;

use common::Result;
use dds::DdsData ;
use tokio::sync::mpsc:: Sender;

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
    pub fn new(tx: Sender<DdsData> ) -> Self {
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
            Ok(_) => {},
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
        self.dds_manager.create_typed_listener(topic_name, data_type_name).await?;
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