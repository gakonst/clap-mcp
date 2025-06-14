use rmcp::{model::*, transport::SseClientTransport, ServiceExt};
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rmcp=debug,reqwest=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("Connecting to MCP server at http://127.0.0.1:8080...");

    // Connect to the HTTP MCP server
    let transport = SseClientTransport::start("http://127.0.0.1:8080/sse").await?;

    let client_info = ClientInfo {
        protocol_version: ProtocolVersion::V_2024_11_05,
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "http-test-client".to_string(),
            version: "1.0".to_string(),
        },
    };

    let client = client_info.serve(transport).await?;

    // Server info from initialization
    let server_info = client.peer_info();
    println!("Server info: {:?}", server_info);

    // List available tools
    println!("\nListing available tools...");
    let tools = client.list_tools(None).await?;
    println!("Available tools:");
    for tool in &tools.tools {
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
        .call_tool(CallToolRequestParam {
            name: "add".into(),
            arguments: Some(
                json!({
                    "a": 10,
                    "b": 32
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    if result.is_error.unwrap_or(false) {
        println!("Error in result");
    } else {
        for content in &result.content {
            match &content.raw {
                RawContent::Text(text_content) => println!("Result: {}", text_content.text),
                RawContent::Image(_) => println!("Result: [image]"),
                RawContent::Resource(_) => println!("Result: [resource]"),
                RawContent::Audio(_) => println!("Result: [audio]"),
            }
        }
    }

    // Call multiply
    println!("\n=== Testing multiply(7, 6) ===");
    let result = client
        .call_tool(CallToolRequestParam {
            name: "multiply".into(),
            arguments: Some(
                json!({
                    "value1": 7,
                    "value2": 6
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    if result.is_error.unwrap_or(false) {
        println!("Error in result");
    } else {
        for content in &result.content {
            match &content.raw {
                RawContent::Text(text_content) => println!("Result: {}", text_content.text),
                RawContent::Image(_) => println!("Result: [image]"),
                RawContent::Resource(_) => println!("Result: [resource]"),
                RawContent::Audio(_) => println!("Result: [audio]"),
            }
        }
    }

    // Call hello
    println!("\n=== Testing hello(MCP User, excited=true) ===");
    let result = client
        .call_tool(CallToolRequestParam {
            name: "hello".into(),
            arguments: Some(
                json!({
                    "name": "MCP User",
                    "excited": true
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    if result.is_error.unwrap_or(false) {
        println!("Error in result");
    } else {
        for content in &result.content {
            match &content.raw {
                RawContent::Text(text_content) => println!("Result: {}", text_content.text),
                RawContent::Image(_) => println!("Result: [image]"),
                RawContent::Resource(_) => println!("Result: [resource]"),
                RawContent::Audio(_) => println!("Result: [audio]"),
            }
        }
    }

    println!("\nAll tests complete!");

    // Clean shutdown
    client.cancel().await?;

    Ok(())
}
