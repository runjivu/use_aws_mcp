use std::io::{BufRead, BufReader, Write};
use serde::{Deserialize, Serialize};

use crate::error::{McpError, Result};
use crate::use_aws::{UseAws, UseAwsRequest, UseAwsResponse};

/// JSON-RPC message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP Server implementation
pub struct AwsMcpServer {
    stdin: std::io::Stdin,
    stdout: std::io::Stdout,
}

impl AwsMcpServer {
    pub fn new() -> Self {
        Self {
            stdin: std::io::stdin(),
            stdout: std::io::stdout(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let reader = BufReader::new(self.stdin.lock());
        
        for line in reader.lines() {
            let line = line.map_err(|e| McpError::Io(e))?;
            if line.trim().is_empty() {
                continue;
            }

            let message: JsonRpcMessage = serde_json::from_str(&line)
                .map_err(|e| McpError::Serialization(e))?;

            let response = self.handle_message(message).await?;
            
            if let Some(response) = response {
                let response_str = serde_json::to_string(&response)
                    .map_err(|e| McpError::Serialization(e))?;
                writeln!(self.stdout, "{}", response_str)
                    .map_err(|e| McpError::Io(e))?;
                self.stdout.flush().map_err(|e| McpError::Io(e))?;
            }
        }

        Ok(())
    }

    async fn handle_message(&mut self, message: JsonRpcMessage) -> Result<Option<JsonRpcResponse>> {
        match message {
            JsonRpcMessage::Request(request) => {
                let response = self.handle_request(request).await?;
                Ok(Some(response))
            }
            JsonRpcMessage::Notification(notification) => {
                self.handle_notification(notification).await?;
                Ok(None)
            }
            JsonRpcMessage::Response(_) => {
                // We don't send requests, so we shouldn't receive responses
                Ok(None)
            }
        }
    }

    async fn handle_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/call" => self.handle_tool_call(request).await,
            "tools/list" => self.handle_tools_list(request).await,
            _ => {
                let error = JsonRpcError {
                    code: -32601, // Method not found
                    message: format!("Method '{}' not found", request.method),
                    data: None,
                };
                Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(error),
                })
            }
        }
    }

    async fn handle_initialize(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let capabilities = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": true
                }
            },
            "serverInfo": {
                "name": "use_aws",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(capabilities),
            error: None,
        })
    }

    async fn handle_tools_list(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Read the tools schema from schema.json at the project root
        let schema_path = std::path::Path::new("schema.json");
        let tools_json = match std::fs::read_to_string(schema_path) {
            Ok(contents) => match serde_json::from_str::<serde_json::Value>(&contents) {
                Ok(json) => json,
                Err(e) => {
                    let error = JsonRpcError {
                        code: -32603,
                        message: format!("Failed to parse schema.json: {}", e),
                        data: None,
                    };
                    return Ok(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(error),
                    });
                }
            },
            Err(e) => {
                let error = JsonRpcError {
                    code: -32603,
                    message: format!("Failed to read schema.json: {}", e),
                    data: None,
                };
                return Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(error),
                });
            }
        };

        // The MCP client expects the result to be { "tools": [...] }
        let tools = match tools_json.get("tools") {
            Some(tools) => serde_json::json!({ "tools": tools }),
            None => {
                let error = JsonRpcError {
                    code: -32603,
                    message: "schema.json does not contain a 'tools' key".to_string(),
                    data: None,
                };
                return Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(error),
                });
            }
        };

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(tools),
            error: None,
        })
    }

    async fn handle_tool_call(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let params = request.params.ok_or_else(|| {
            McpError::InvalidRequest("Missing params for tools/call".to_string())
        })?;

        let tool_call: ToolCall = serde_json::from_value(params)
            .map_err(|e| McpError::Serialization(e))?;

        if tool_call.name != "use_aws" {
            let error = JsonRpcError {
                code: -32601,
                message: format!("Tool '{}' not found", tool_call.name),
                data: None,
            };
            return Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(error),
            });
        }

        let use_aws_request: UseAwsRequest = serde_json::from_value(tool_call.arguments)
            .map_err(|e| McpError::Serialization(e))?;

        // Generate a human-readable description of the command
        let use_aws = UseAws::from(use_aws_request.clone());
        let mut description_output = Vec::new();
        if let Err(e) = use_aws.queue_description(&mut description_output) {
            tracing::warn!("Failed to generate command description: {}", e);
        }

        let result = use_aws.invoke().await;

        match result {
            Ok(invoke_output) => {
                let response: UseAwsResponse = invoke_output.into();
                
                // Include the description in the response if available
                let description = if !description_output.is_empty() {
                    String::from_utf8(description_output).unwrap_or_default()
                } else {
                    String::new()
                };

                let content = serde_json::json!([
                    {
                        "type": "text",
                        "text": format!("{}\n\nResult:\n{}", 
                            description,
                            serde_json::to_string(&response).unwrap_or_default()
                        )
                    }
                ]);

                let tool_result = serde_json::json!({
                    "content": content
                });

                Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(tool_result),
                    error: None,
                })
            }
            Err(e) => {
                let error = JsonRpcError {
                    code: -32000,
                    message: format!("Tool execution failed: {}", e),
                    data: None,
                };
                Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(error),
                })
            }
        }
    }

    async fn handle_notification(&self, notification: JsonRpcNotification) -> Result<()> {
        match notification.method.as_str() {
            "notifications/initialized" => {
                // Server is initialized, we can start handling requests
                Ok(())
            }
            _ => {
                // Ignore unknown notifications
                Ok(())
            }
        }
    }

    /// Generate a human-readable description of a tool call
    pub fn generate_tool_description(&self, tool_call: &ToolCall) -> Result<String> {
        if tool_call.name != "use_aws" {
            return Ok(format!("Unknown tool: {}", tool_call.name));
        }

        let use_aws_request: UseAwsRequest = serde_json::from_value(tool_call.arguments.clone())
            .map_err(|e| McpError::Serialization(e))?;

        let use_aws = UseAws::from(use_aws_request);
        let mut output = Vec::new();
        use_aws.queue_description(&mut output)
            .map_err(|e| McpError::ToolExecution(e.to_string()))?;

        String::from_utf8(output)
            .map_err(|e| McpError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

impl Default for AwsMcpServer {
    fn default() -> Self {
        Self::new()
    }
} 