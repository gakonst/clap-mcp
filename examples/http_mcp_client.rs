use clap_mcp::test_client::McpTestClient;
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("Connecting to MCP server at http://127.0.0.1:8080...");
    
    // Connect to server
    let client = McpTestClient::connect("127.0.0.1:8080").await?;

    // List available tools
    println!("\nListing available tools...");
    let tools = client.list_tools().await?;
    println!("Available tools:");
    for tool in &tools {
        println!("Tool {{");
        println!("  name: {}", tool.name);
        println!(
            "  description: {}",
            tool.description
                .as_ref()
                .map(|d| d.as_ref())
                .unwrap_or("None")
        );
        println!("  input_schema: {{");
        let schema = &tool.input_schema;
        println!(
            "    type: {}",
            schema
                .get("type")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string())
        );
        println!(
            "    required: {}",
            schema
                .get("required")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string())
        );
        if let Some(properties) = schema.get("properties") {
            println!("    properties: {{");
            if let Some(props_obj) = properties.as_object() {
                for (prop_name, prop_schema) in props_obj {
                    println!("      {}: {{", prop_name);
                    if let Some(prop_obj) = prop_schema.as_object() {
                        for (key, value) in prop_obj {
                            println!("        {}: {}", key, value);
                        }
                    }
                    println!("      }}");
                }
            }
            println!("    }}");
        }
        println!(
            "    additionalProperties: {}",
            schema
                .get("additionalProperties")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string())
        );
        println!("  }}");
        println!("}}");
        println!();
    }

    // Call the add tool
    println!("\n=== Testing add(10, 32) ===");
    let result = client
        .call_tool("add", Some(json!({ "a": 10, "b": 32 })))
        .await?;

    if result.is_error.unwrap_or(false) {
        println!("Error in result");
    } else {
        if let Some(text) = McpTestClient::extract_text(&result) {
            println!("Result: {}", text);
        }
    }

    // Call multiply
    println!("\n=== Testing multiply(7, 6) ===");
    let result = client
        .call_tool("multiply", Some(json!({ "value1": 7, "value2": 6 })))
        .await?;

    if result.is_error.unwrap_or(false) {
        println!("Error in result");
    } else {
        if let Some(text) = McpTestClient::extract_text(&result) {
            println!("Result: {}", text);
        }
    }

    // Call hello
    println!("\n=== Testing hello(MCP User, excited=true) ===");
    let result = client
        .call_tool("hello", Some(json!({ "name": "MCP User", "excited": true })))
        .await?;

    if result.is_error.unwrap_or(false) {
        println!("Error in result");
    } else {
        if let Some(text) = McpTestClient::extract_text(&result) {
            println!("Result: {}", text);
        }
    }

    println!("\nAll tests complete!");

    // Clean shutdown
    client.shutdown().await?;

    Ok(())
}
