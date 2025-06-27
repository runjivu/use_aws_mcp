# use_aws MCP Server

amazon-q-cli is great, and it is great because it has use_aws MCP tool to interact with aws API.
Wouldn't it be greater if this use_aws was portable, and use it across different AI tools, whichever you're currently using?

A standalone Model Context Protocol (MCP) server that provides AWS CLI functionality through a standardized interface. 
This server replicates the functionality of the `use_aws` tool from the Amazon Q Developer CLI.

## Demo

## Features

- **AWS CLI Integration**: Execute AWS CLI commands with proper parameter handling
- **Safety Checks**: Automatic detection of read-only vs. write operations
- **User Agent Management**: Proper AWS CLI user agent setup for tracking
- **Parameter Formatting**: Automatic conversion of parameters to kebab-case for CLI compatibility
- **Error Handling**: Comprehensive error handling and output formatting
- **MCP Protocol**: Full Model Context Protocol compliance
- **Human-Readable Descriptions**: Rich command descriptions using terminal formatting

## Installation

### Prerequisites

- Rust (1.70 or later), Cargo
- AWS CLI installed and configured
- AWS credentials configured (via AWS CLI, environment variables, or IAM roles)

### Building

```bash
cargo build --release
```

The binary will be available at `target/release/use_aws`.

## Usage

### Running the MCP Server

```bash
./target/release/use_aws_mcp
```

The server communicates via stdin/stdout using JSON-RPC protocol.

### MCP Client Integration

To use this server with an MCP client, first install it using Cargo:

```sh
cargo install use_aws
```

Then configure your MCP client with:

```json
{
  "name": "use_aws_mcp",
  "command": "use_aws_mcp",
  "env": {},
  "timeout": 30000
}
```

### Command Descriptions

The server provides human-readable descriptions of AWS CLI commands. You can see this in action by running the example:

```bash
cargo run --example description_demo
```

This will output something like:
```
Running aws cli command:

Service name: s3
Operation name: list-buckets
Parameters: 
- max-items: "10"
- query: "Buckets[].Name"
Profile name: development
Region: us-west-2
Label: List S3 buckets with query

✅ This command is read-only (no acceptance required)
```

## Tool Specification

The server provides a single tool called `use_aws` with the following schema:

```json
{
  "name": "use_aws",
  "description": "Execute AWS CLI commands with proper parameter handling and safety checks",
  "inputSchema": {
    "type": "object",
    "properties": {
      "service_name": {
        "type": "string",
        "description": "AWS service name (e.g., s3, ec2, lambda)"
      },
      "operation_name": {
        "type": "string",
        "description": "AWS CLI operation name (e.g., list-buckets, describe-instances)"
      },
      "parameters": {
        "type": "object",
        "description": "Optional parameters for the AWS CLI command",
        "additionalProperties": true
      },
      "region": {
        "type": "string",
        "description": "AWS region (e.g., us-west-2, eu-west-1)"
      },
      "profile_name": {
        "type": "string",
        "description": "Optional AWS profile name"
      },
      "label": {
        "type": "string",
        "description": "Optional label for the operation"
      }
    },
    "required": ["service_name", "operation_name", "region"]
  }
}
```

## Examples

### List S3 Buckets

```json
{
  "name": "use_aws",
  "arguments": {
    "service_name": "s3",
    "operation_name": "ls",
    "region": "us-west-2"
  }
}
```

### Describe EC2 Instances

```json
{
  "name": "use_aws",
  "arguments": {
    "service_name": "ec2",
    "operation_name": "describe-instances",
    "region": "us-west-2",
    "parameters": {
      "instance-ids": "i-1234567890abcdef0"
    }
  }
}
```

### List Lambda Functions with Profile

```json
{
  "name": "use_aws",
  "arguments": {
    "service_name": "lambda",
    "operation_name": "list-functions",
    "region": "us-west-2",
    "profile_name": "development"
  }
}
```

## Safety Features

### Read-Only Operation Detection

The server automatically detects read-only operations based on the operation name prefix:

- **Read-only prefixes**: `get`, `describe`, `list`, `ls`, `search`, `batch_get`
- **Write operations**: All other operations require explicit user acceptance

### Output Truncation

Large outputs are automatically truncated to prevent memory issues, with a maximum response size of 100KB.

## Development

### Running Tests

```bash
cargo test
```

### Building for Development

```bash
cargo build
```

### Running with Logging

```bash
RUST_LOG=use_aws=debug cargo run
```

### Examples

```bash
# Run the description demo
cargo run --example description_demo
```

## Architecture

The project is structured as follows:

- `src/lib.rs`: Core library with types and constants
- `src/error.rs`: Error handling types
- `src/use_aws.rs`: Core AWS CLI functionality (replicated from original)
- `src/mcp_server.rs`: MCP server implementation
- `src/main.rs`: Binary entry point
- `examples/description_demo.rs`: Example demonstrating command descriptions

## Dependencies

If you do not have Cargo (the Rust package manager) installed, you can get it by installing Rust using [rustup](https://rustup.rs/):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the on-screen instructions to complete the installation. After installation, restart your terminal and ensure Cargo is available by running:

```sh
cargo --version
```

You should see the installed Cargo version printed.

This project is distributed as a Rust crate. The following dependencies are managed automatically by Cargo:

- `tokio`
- `serde`
- `serde_json`
- `eyre`
- `bstr`
- `convert_case`
- `async-trait`
- `thiserror`
- `tracing`
- `tracing-subscriber`
- `crossterm`

test/dev dependencies:
- `tokio-test`

You do not need to install these manually; Cargo will handle them during installation.

## License

MIT, Apache-2.0

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## Security

This server executes AWS CLI commands, which may have security implications:

- Ensure proper AWS credentials and permissions
- Review all commands before execution
- Use read-only operations when possible
- Consider running in a restricted environment

## Troubleshooting

### Common Issues

1. **AWS CLI not found**: Ensure AWS CLI is installed and in PATH
2. **Permission denied**: Check AWS credentials and permissions
3. **Invalid region**: Verify the region name is correct
4. **Parameter errors**: Check parameter names and values

### Debug Mode

Run with debug logging to see detailed information:

```bash
RUST_LOG=use_aws=debug ./target/release/use_aws
```
