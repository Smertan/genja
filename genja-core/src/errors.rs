use std::fmt;

#[derive(Debug, Clone)]
pub enum InventoryLoadError {
    Message(String),
}

impl fmt::Display for InventoryLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InventoryLoadError::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for InventoryLoadError {}

impl From<String> for InventoryLoadError {
    fn from(value: String) -> Self {
        InventoryLoadError::Message(value)
    }
}

impl From<&str> for InventoryLoadError {
    fn from(value: &str) -> Self {
        InventoryLoadError::Message(value.to_string())
    }
}
