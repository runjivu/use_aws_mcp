use use_aws_mcp::use_aws::UseAws;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a sample AWS CLI command
    let cmd = UseAws {
        service_name: "s3".to_string(),
        operation_name: "list-buckets".to_string(),
        parameters: Some(std::collections::HashMap::from([
            ("max-items".to_string(), serde_json::Value::String("10".to_string())),
            ("query".to_string(), serde_json::Value::String("Buckets[].Name".to_string())),
        ])),
        region: "us-west-2".to_string(),
        profile_name: Some("development".to_string()),
        label: Some("List S3 buckets with query".to_string()),
    };

    // Generate and display the human-readable description
    let mut output = Vec::new();
    cmd.queue_description(&mut output)?;
    
    let description = String::from_utf8(output)?;
    println!("{}", description);

    // Show whether this command requires user acceptance
    if cmd.requires_acceptance() {
        println!("\n⚠️  This command requires user acceptance (write operation)");
    } else {
        println!("\n✅ This command is read-only (no acceptance required)");
    }

    Ok(())
} 