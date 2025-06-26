use std::collections::HashMap;
use std::io::Write;
use std::process::Stdio;

use bstr::ByteSlice;
use convert_case::{Case, Casing};
use crossterm::{
    queue,
    style,
};
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

use crate::{InvokeOutput, MAX_TOOL_RESPONSE_SIZE, OutputKind};

const READONLY_OPS: [&str; 6] = ["get", "describe", "list", "ls", "search", "batch_get"];

/// The environment variable name where we set additional metadata for the AWS CLI user agent.
const USER_AGENT_ENV_VAR: &str = "AWS_EXECUTION_ENV";
const USER_AGENT_APP_NAME: &str = "UseAws-MCP-Server";
const USER_AGENT_VERSION_KEY: &str = "Version";
const USER_AGENT_VERSION_VALUE: &str = env!("CARGO_PKG_VERSION");

/// The main UseAws struct that handles AWS CLI operations
#[derive(Debug, Clone, Deserialize)]
pub struct UseAws {
    pub service_name: String,
    pub operation_name: String,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub region: String,
    pub profile_name: Option<String>,
    pub label: Option<String>,
}

/// Request structure for MCP tool calls
#[derive(Debug, Clone, Deserialize)]
pub struct UseAwsRequest {
    pub service_name: String,
    pub operation_name: String,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub region: String,
    pub profile_name: Option<String>,
    pub label: Option<String>,
}

/// Response structure for MCP tool calls
#[derive(Debug, Serialize)]
pub struct UseAwsResponse {
    pub exit_status: String,
    pub stdout: String,
    pub stderr: String,
}

impl UseAws {
    pub fn requires_acceptance(&self) -> bool {
        !READONLY_OPS.iter().any(|op| self.operation_name.starts_with(op))
    }

    pub async fn invoke(&self) -> Result<InvokeOutput> {
        let mut command = tokio::process::Command::new("aws");

        // Set up environment variables
        let mut env_vars: std::collections::HashMap<String, String> = std::env::vars().collect();

        // Set up additional metadata for the AWS CLI user agent
        let user_agent_metadata_value = format!(
            "{} {}/{}",
            USER_AGENT_APP_NAME, USER_AGENT_VERSION_KEY, USER_AGENT_VERSION_VALUE
        );

        // If the user agent metadata env var already exists, append to it, otherwise set it
        if let Some(existing_value) = env_vars.get(USER_AGENT_ENV_VAR) {
            if !existing_value.is_empty() {
                env_vars.insert(
                    USER_AGENT_ENV_VAR.to_string(),
                    format!("{} {}", existing_value, user_agent_metadata_value),
                );
            } else {
                env_vars.insert(USER_AGENT_ENV_VAR.to_string(), user_agent_metadata_value);
            }
        } else {
            env_vars.insert(USER_AGENT_ENV_VAR.to_string(), user_agent_metadata_value);
        }

        command.envs(env_vars).arg("--region").arg(&self.region);
        if let Some(profile_name) = self.profile_name.as_deref() {
            command.arg("--profile").arg(profile_name);
        }
        command.arg(&self.service_name).arg(&self.operation_name);
        if let Some(parameters) = self.cli_parameters() {
            for (name, val) in parameters {
                command.arg(name);
                if !val.is_empty() {
                    command.arg(val);
                }
            }
        }
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .wrap_err_with(|| format!("Unable to spawn command '{:?}'", self))?
            .wait_with_output()
            .await
            .wrap_err_with(|| format!("Unable to spawn command '{:?}'", self))?;
        let status = output.status.code().unwrap_or(0).to_string();
        let stdout = output.stdout.to_str_lossy();
        let stderr = output.stderr.to_str_lossy();

        let stdout = format!(
            "{}{}",
            &stdout[0..stdout.len().min(MAX_TOOL_RESPONSE_SIZE / 3)],
            if stdout.len() > MAX_TOOL_RESPONSE_SIZE / 3 {
                " ... truncated"
            } else {
                ""
            }
        );

        let stderr = format!(
            "{}{}",
            &stderr[0..stderr.len().min(MAX_TOOL_RESPONSE_SIZE / 3)],
            if stderr.len() > MAX_TOOL_RESPONSE_SIZE / 3 {
                " ... truncated"
            } else {
                ""
            }
        );

        if status.eq("0") {
            Ok(InvokeOutput {
                output: OutputKind::Json(serde_json::json!({
                    "exit_status": status,
                    "stdout": stdout,
                    "stderr": stderr.clone()
                })),
            })
        } else {
            Err(eyre::eyre!(stderr))
        }
    }

    pub fn queue_description(&self, updates: &mut impl Write) -> Result<()> {
        queue!(
            updates,
            style::Print("Running aws cli command:\n\n"),
            style::Print(format!("Service name: {}\n", self.service_name)),
            style::Print(format!("Operation name: {}\n", self.operation_name)),
        )?;
        if let Some(parameters) = &self.parameters {
            queue!(updates, style::Print("Parameters: \n".to_string()))?;
            for (name, value) in parameters {
                match value {
                    serde_json::Value::String(s) if s.is_empty() => {
                        queue!(updates, style::Print(format!("- {}\n", name)))?;
                    },
                    _ => {
                        queue!(updates, style::Print(format!("- {}: {}\n", name, value)))?;
                    },
                }
            }
        }

        if let Some(ref profile_name) = self.profile_name {
            queue!(updates, style::Print(format!("Profile name: {}\n", profile_name)))?;
        } else {
            queue!(updates, style::Print("Profile name: default\n".to_string()))?;
        }

        queue!(updates, style::Print(format!("Region: {}", self.region)))?;

        if let Some(ref label) = self.label {
            queue!(updates, style::Print(format!("\nLabel: {}", label)))?;
        }
        Ok(())
    }

    pub async fn validate(&mut self) -> Result<()> {
        Ok(())
    }

    /// Returns the CLI arguments properly formatted as kebab case if parameters is
    /// [Option::Some], otherwise None
    fn cli_parameters(&self) -> Option<Vec<(String, String)>> {
        if let Some(parameters) = &self.parameters {
            let mut params = vec![];
            for (param_name, val) in parameters {
                let param_name = format!("--{}", param_name.trim_start_matches("--").to_case(Case::Kebab));
                let param_val = val.as_str().map(|s| s.to_string()).unwrap_or(val.to_string());
                params.push((param_name, param_val));
            }
            Some(params)
        } else {
            None
        }
    }
}

impl From<UseAwsRequest> for UseAws {
    fn from(request: UseAwsRequest) -> Self {
        Self {
            service_name: request.service_name,
            operation_name: request.operation_name,
            parameters: request.parameters,
            region: request.region,
            profile_name: request.profile_name,
            label: request.label,
        }
    }
}

impl From<InvokeOutput> for UseAwsResponse {
    fn from(output: InvokeOutput) -> Self {
        match output.output {
            OutputKind::Json(json) => {
                let exit_status = json.get("exit_status").and_then(|v| v.as_str()).unwrap_or("0").to_string();
                let stdout = json.get("stdout").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let stderr = json.get("stderr").and_then(|v| v.as_str()).unwrap_or("").to_string();
                Self {
                    exit_status,
                    stdout,
                    stderr,
                }
            }
            OutputKind::Text(text) => Self {
                exit_status: "0".to_string(),
                stdout: text,
                stderr: "".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! use_aws {
        ($value:tt) => {
            serde_json::from_value::<UseAws>(serde_json::json!($value)).unwrap()
        };
    }

    #[test]
    fn test_requires_acceptance() {
        let cmd = use_aws! {{
            "service_name": "ecs",
            "operation_name": "list-task-definitions",
            "region": "us-west-2",
            "profile_name": "default",
            "label": ""
        }};
        assert!(!cmd.requires_acceptance());
        let cmd = use_aws! {{
            "service_name": "lambda",
            "operation_name": "list-functions",
            "region": "us-west-2",
            "profile_name": "default",
            "label": ""
        }};
        assert!(!cmd.requires_acceptance());
        let cmd = use_aws! {{
            "service_name": "s3",
            "operation_name": "put-object",
            "region": "us-west-2",
            "profile_name": "default",
            "label": ""
        }};
        assert!(cmd.requires_acceptance());
    }

    #[test]
    fn test_use_aws_deser() {
        let cmd = use_aws! {{
            "service_name": "s3",
            "operation_name": "put-object",
            "parameters": {
                "TableName": "table-name",
                "KeyConditionExpression": "PartitionKey = :pkValue"
            },
            "region": "us-west-2",
            "profile_name": "default",
            "label": ""
        }};
        let params = cmd.cli_parameters().unwrap();
        assert!(
            params.iter().any(|p| p.0 == "--table-name" && p.1 == "table-name"),
            "not found in {:?}",
            params
        );
        assert!(
            params
                .iter()
                .any(|p| p.0 == "--key-condition-expression" && p.1 == "PartitionKey = :pkValue"),
            "not found in {:?}",
            params
        );
    }

    #[test]
    fn test_queue_description() {
        let cmd = use_aws! {{
            "service_name": "s3",
            "operation_name": "list-buckets",
            "parameters": {
                "max-items": "10"
            },
            "region": "us-west-2",
            "profile_name": "development",
            "label": "List S3 buckets"
        }};
        
        let mut output = Vec::new();
        cmd.queue_description(&mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        
        println!("Generated output: '{}'", output_str);
        
        assert!(output_str.contains("Running aws cli command:"));
        assert!(output_str.contains("Service name: s3"));
        assert!(output_str.contains("Operation name: list-buckets"));
        assert!(output_str.contains("Parameters:"));
        assert!(output_str.contains("- max-items: \"10\""));
        assert!(output_str.contains("Profile name: development"));
        assert!(output_str.contains("Region: us-west-2"));
        assert!(output_str.contains("Label: List S3 buckets"));
    }

    #[test]
    fn test_queue_description_empty_parameters() {
        let cmd = use_aws! {{
            "service_name": "ec2",
            "operation_name": "describe-instances",
            "region": "us-east-1"
        }};
        
        let mut output = Vec::new();
        cmd.queue_description(&mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        
        assert!(output_str.contains("Running aws cli command:"));
        assert!(output_str.contains("Service name: ec2"));
        assert!(output_str.contains("Operation name: describe-instances"));
        assert!(output_str.contains("Profile name: default"));
        assert!(output_str.contains("Region: us-east-1"));
        assert!(!output_str.contains("Parameters:"));
    }

    #[tokio::test]
    async fn test_environment_variables_passed_through() {
        // Print current environment variables for debugging
        println!("Current environment variables:");
        for (key, value) in std::env::vars() {
            if key.contains("AWS") {
                println!("  {}: {}", key, value);
            }
        }
        println!();

        // Test the use_aws tool with a simple AWS command
        let use_aws = UseAws {
            service_name: "sts".to_string(),
            operation_name: "get-caller-identity".to_string(),
            parameters: None,
            region: "us-east-1".to_string(),
            profile_name: None, // This should use AWS_PROFILE from environment
            label: Some("Test AWS credentials".to_string()),
        };

        println!("Testing AWS credentials with use_aws tool...");
        match use_aws.invoke().await {
            Ok(output) => {
                println!("Success! Output: {:?}", output);
                // If we get here, it means the environment variables were passed through correctly
                assert!(true, "Environment variables were passed through successfully");
            }
            Err(e) => {
                println!("Error: {}", e);
                // This test will fail if credentials are not found, which indicates
                // that environment variables are not being passed through correctly
                panic!("Failed to invoke AWS command: {}", e);
            }
        }
    }
} 