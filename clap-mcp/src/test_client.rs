//! Test utilities for clap-mcp

use rmcp::{model::*, transport::SseClientTransport, RoleClient, ServiceExt};
use serde_json::Value;

/// An MCP test client for testing MCP servers
pub struct McpTestClient {
    client: rmcp::service::RunningService<RoleClient, ClientInfo>,
}

impl McpTestClient {
    /// Connect to an MCP server at the given address
    pub async fn connect(addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let sse_url = format!("http://{}/sse", addr);
        let transport = SseClientTransport::start(sse_url).await?;

        let client_info = ClientInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "test-client".to_string(),
                version: "1.0".to_string(),
            },
        };

        let client = client_info.serve(transport).await?;

        Ok(Self { client })
    }

    /// List all available tools
    pub async fn list_tools(&self) -> Result<Vec<Tool>, Box<dyn std::error::Error>> {
        let result = self.client.list_tools(None).await?;
        Ok(result.tools)
    }

    /// Call a tool with optional arguments
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult, Box<dyn std::error::Error>> {
        let args = arguments.and_then(|v| v.as_object().cloned());
        let result = self
            .client
            .call_tool(CallToolRequestParam {
                name: name.to_string().into(),
                arguments: args,
            })
            .await?;
        Ok(result)
    }

    /// Extract text content from a tool result
    pub fn extract_text(result: &CallToolResult) -> Option<String> {
        result.content.first().and_then(|content| {
            if let RawContent::Text(text) = &content.raw {
                Some(text.text.clone())
            } else {
                None
            }
        })
    }

    /// Shutdown the client
    pub async fn shutdown(self) -> Result<(), Box<dyn std::error::Error>> {
        self.client.cancel().await?;
        Ok(())
    }
}

#[cfg(test)]
pub mod test_utils {
    use std::process::{Child, Command, Stdio};
    use std::time::Duration;

    /// Start a server process for testing
    pub fn start_test_server(exe_path: &str, args: &[&str]) -> std::io::Result<Child> {
        Command::new(exe_path)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }

    /// Wait for a server to be ready
    pub async fn wait_for_server(addr: &str, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            if let Ok(client) = super::McpTestClient::connect(addr).await {
                let _ = client.shutdown().await;
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }
}
