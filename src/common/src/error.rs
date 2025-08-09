use thiserror::Error;

/// Main error type for the Pullpiri system
#[derive(Debug, Error, Clone)]
pub enum PullpiriError {
    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    /// gRPC communication errors
    #[error("gRPC error: {message}")]
    Grpc { message: String },
    
    /// ETCD related errors
    #[error("ETCD error: {message}")]
    Etcd { message: String },
    
    /// File I/O errors
    #[error("I/O error: {message}")]
    Io { message: String },
    
    /// Parsing/Serialization errors
    #[error("Parsing error: {message}")]
    Parse { message: String },
    
    /// Runtime errors
    #[error("Runtime error: {message}")]
    Runtime { message: String },
    
    /// Timeout errors
    #[error("Timeout error: operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    
    /// Internal system errors
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl PullpiriError {
    /// Create a new configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Configuration { message: message.into() }
    }
    
    /// Create a new gRPC error
    pub fn grpc<S: Into<String>>(message: S) -> Self {
        Self::Grpc { message: message.into() }
    }
    
    /// Create a new ETCD error
    pub fn etcd<S: Into<String>>(message: S) -> Self {
        Self::Etcd { message: message.into() }
    }
    
    /// Create a new I/O error
    pub fn io<S: Into<String>>(message: S) -> Self {
        Self::Io { message: message.into() }
    }
    
    /// Create a new parsing error
    pub fn parse<S: Into<String>>(message: S) -> Self {
        Self::Parse { message: message.into() }
    }
    
    /// Create a new runtime error
    pub fn runtime<S: Into<String>>(message: S) -> Self {
        Self::Runtime { message: message.into() }
    }
    
    /// Create a new timeout error
    pub fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }
    
    /// Create a new internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal { message: message.into() }
    }
}

/// Convenient conversion from anyhow::Error
impl From<anyhow::Error> for PullpiriError {
    fn from(err: anyhow::Error) -> Self {
        PullpiriError::Internal { message: err.to_string() }
    }
}

/// Convenient conversion from std::io::Error
impl From<std::io::Error> for PullpiriError {
    fn from(err: std::io::Error) -> Self {
        PullpiriError::Io { message: err.to_string() }
    }
}

/// Convenient conversion from serde_yaml::Error
impl From<serde_yaml::Error> for PullpiriError {
    fn from(err: serde_yaml::Error) -> Self {
        PullpiriError::Parse { message: err.to_string() }
    }
}

/// Convenient conversion from serde_json::Error
impl From<serde_json::Error> for PullpiriError {
    fn from(err: serde_json::Error) -> Self {
        PullpiriError::Parse { message: err.to_string() }
    }
}

/// Convenient conversion from tonic::Status
impl From<tonic::Status> for PullpiriError {
    fn from(err: tonic::Status) -> Self {
        PullpiriError::Grpc { message: err.to_string() }
    }
}

/// Convenient conversion from dbus::Error
impl From<dbus::Error> for PullpiriError {
    fn from(err: dbus::Error) -> Self {
        PullpiriError::Runtime { message: err.to_string() }
    }
}

/// Convenient conversion from String
impl From<String> for PullpiriError {
    fn from(message: String) -> Self {
        PullpiriError::Runtime { message }
    }
}

/// Convenient conversion from &str
impl From<&str> for PullpiriError {
    fn from(message: &str) -> Self {
        PullpiriError::Runtime { message: message.to_string() }
    }
}

/// Convenient conversion from tonic::transport::Error
impl From<tonic::transport::Error> for PullpiriError {
    fn from(err: tonic::transport::Error) -> Self {
        PullpiriError::Grpc { message: err.to_string() }
    }
}

/// Convenient conversion from etcd_client::Error
impl From<etcd_client::Error> for PullpiriError {
    fn from(err: etcd_client::Error) -> Self {
        PullpiriError::Etcd { message: err.to_string() }
    }
}

/// Result type alias using our custom error type
pub type Result<T> = std::result::Result<T, PullpiriError>;

/// Error reporting channel message
#[derive(Debug, Clone)]
pub struct ErrorReport {
    pub error: String,
    pub component: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: Option<String>,
}

impl ErrorReport {
    pub fn new<S: Into<String>>(error: S, component: S) -> Self {
        Self {
            error: error.into(),
            component: component.into(),
            timestamp: chrono::Utc::now(),
            context: None,
        }
    }
    
    pub fn with_context<S: Into<String>>(mut self, context: S) -> Self {
        self.context = Some(context.into());
        self
    }
}
