pub mod listener;

use common::Result;
use tokio::sync::mpsc;

/// DDS data structure
///
/// Represents data received from DDS topics
#[derive(Debug, Clone)]
pub struct DdsData {
    /// Name of the topic
    pub name: String,
    /// Value of the data
    pub value: String,
}

/// Initializes the DDS module
///
/// Sets up the DDS domain participant and core entities
///
/// # Arguments
///
/// * `tx` - Channel sender for DDS data
///
/// # Returns
///
/// * `Result<()>` - Success or error result
pub async fn run(tx: mpsc::Sender<DdsData>) -> Result<()> {
    // Create a DdsManager instance using the provided channel
    let mut manager = DdsManager {
        listeners: Vec::new(),
        domain_id: 100, // Default domain ID
        tx,
    };

    // Configure the manager as needed
    // manager.set_domain_id(1);  // Optional: use a different domain

    // Add required listeners based on application needs
    // Example: manager.create_listener("vehicle_status".to_string(), "Status".to_string()).await?;

    // Start all listeners
    manager.start_all().await?;
    Ok(())
}

/// DDS manager for handling listeners
///
/// Manages DDS topic listeners and data flow
pub struct DdsManager {
    /// Collection of active listeners
    listeners: Vec<listener::TopicListener>,
    /// Domain ID for DDS communications
    domain_id: u32,
    /// Channel sender for DDS data
    tx: mpsc::Sender<DdsData>,
}

impl DdsManager {
    /// Creates a new DDS manager
    ///
    /// # Returns
    ///
    /// A new DdsManager instance
    pub fn new() -> Self {
        let (tx, _) = mpsc::channel::<DdsData>(10);
        Self {
            listeners: Vec::new(),
            domain_id: 0, // Default domain ID
            tx,
        }
    }

    /// Sets the domain ID
    ///
    /// # Arguments
    ///
    /// * `domain_id` - DDS domain ID to use
    pub fn set_domain_id(&mut self, domain_id: u32) {
        self.domain_id = domain_id;
    }

    /// Gets the sender channel
    ///
    /// # Returns
    ///
    /// A clone of the sender channel
    pub fn get_sender(&self) -> mpsc::Sender<DdsData> {
        self.tx.clone()
    }

    /// Creates a listener for a topic
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the topic to listen for
    /// * `data_type_name` - Type name of the data
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn create_listener(
        &mut self,
        topic_name: String,
        data_type_name: String,
    ) -> Result<()> {
        // TODO: Implementation
        // Create a new listener with the given topic and data type
        let listener = listener::TopicListener::new(
            topic_name.clone(),
            data_type_name.clone(),
            self.tx.clone(),
            self.domain_id,
        );

        // Add the listener to our collection
        self.listeners.push(listener);
        Ok(())
    }

    /// Removes a listener for a topic
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the topic
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn remove_listener(&mut self, topic_name: &str) -> Result<()> {
        // TODO: Implementation
        // Find and remove listeners with the matching topic name
        let initial_len = self.listeners.len();
        self.listeners
            .retain(|listener| listener.topic_name != topic_name);

        // Check if any listeners were removed
        if self.listeners.len() == initial_len {
            // Optional: You could log a warning here that no listener was found
            // log::warn!("No listener found for topic: {}", topic_name);
        }
        Ok(())
    }

    /// Starts all listeners
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn start_all(&mut self) -> Result<()> {
        // TODO: Implementation
        for listener in &mut self.listeners {
            listener.start().await?;
        }
        Ok(())
    }

    /// Stops all listeners
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn stop_all(&mut self) -> Result<()> {
        // TODO: Implementation
        for listener in &mut self.listeners {
            listener.stop().await?;
        }
        Ok(())
    }
}
