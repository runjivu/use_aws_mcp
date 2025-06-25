use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("AWS CLI execution error: {0}")]
    AwsCli(String),
    
    #[error("Tool execution error: {0}")]
    ToolExecution(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

pub type Result<T> = std::result::Result<T, McpError>; 