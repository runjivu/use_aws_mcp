pub mod error;
pub mod mcp_server;
pub mod use_aws;

pub use error::McpError;
pub use mcp_server::AwsMcpServer;
pub use use_aws::{UseAws, UseAwsRequest, UseAwsResponse};

/// Maximum size for tool response output
pub const MAX_TOOL_RESPONSE_SIZE: usize = 100_000;

/// Output kind for tool responses
#[derive(Debug, Clone)]
pub enum OutputKind {
    Text(String),
    Json(serde_json::Value),
}

impl Default for OutputKind {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

/// Tool invocation output
#[derive(Debug, Default)]
pub struct InvokeOutput {
    pub output: OutputKind,
}

impl InvokeOutput {
    pub fn as_str(&self) -> &str {
        match &self.output {
            OutputKind::Text(s) => s.as_str(),
            OutputKind::Json(j) => j.as_str().unwrap_or_default(),
        }
    }
} 