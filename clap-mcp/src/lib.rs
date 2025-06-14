pub use clap_mcp_derive::McpMode;

use clap::Subcommand;
use rmcp::{
    model::*,
    handler::server::ServerHandler,
    service::{RequestContext, RoleServer},
    Error as McpError,
};
use serde_json::json;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::future::Future;
use std::sync::Arc;
use std::net::SocketAddr;

/// Configuration for MCP server transport
pub enum McpTransport {
    /// Standard I/O (stdin/stdout)
    Stdio,
    /// HTTP Server-Sent Events (SSE) on specified address
    Http(SocketAddr),
}

/// Handler function that processes a subcommand and returns output
pub type CommandHandler<T> = Box<dyn Fn(T) -> Result<String, String> + Send + Sync>;

pub struct McpServer<T: Subcommand> {
    handler: Option<CommandHandler<T>>,
    _phantom: PhantomData<T>,
}

impl<T: Subcommand + Send + Sync + Clone + 'static> McpServer<T> {
    pub fn new() -> Self {
        Self {
            handler: None,
            _phantom: PhantomData,
        }
    }
    
    pub fn with_handler(mut self, handler: CommandHandler<T>) -> Self {
        self.handler = Some(handler);
        self
    }
    
    pub async fn serve_stdio(self) -> Result<(), Box<dyn std::error::Error>> {
        let handler = ClapMcpHandler::<T>::new(self.handler);
        rmcp::serve_server(handler, rmcp::transport::stdio()).await?;
        Ok(())
    }
    
    pub async fn serve_http(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        use rmcp::transport::sse_server::{SseServer, SseServerConfig};
        
        let handler = ClapMcpHandler::<T>::new(self.handler);
        
        let config = SseServerConfig {
            bind: addr,
            sse_path: "/sse".to_string(),
            post_path: "/message".to_string(),
            ct: tokio_util::sync::CancellationToken::new(),
            sse_keep_alive: None,
        };
        
        let (sse_server, router) = SseServer::new(config);
        
        let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
        println!("MCP server listening on http://{}", addr);
        println!("SSE endpoint: http://{}/sse", addr);
        println!("Message endpoint: http://{}/message", addr);
        
        let ct = sse_server.config.ct.child_token();
        
        let server = axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(async move {
                ct.cancelled().await;
            });
        
        tokio::spawn(async move {
            if let Err(e) = server.await {
                eprintln!("MCP SSE server error: {}", e);
            }
        });
        
        let ct = sse_server.with_service(move || handler.clone());
        
        tokio::signal::ctrl_c().await?;
        println!("\nShutting down MCP server...");
        ct.cancel();
        Ok(())
    }
    
    pub async fn serve(self, transport: McpTransport) -> Result<(), Box<dyn std::error::Error>> {
        match transport {
            McpTransport::Stdio => self.serve_stdio().await,
            McpTransport::Http(addr) => self.serve_http(addr).await,
        }
    }
}

struct ClapMcpHandler<T> {
    handler: Option<Arc<CommandHandler<T>>>,
    _phantom: PhantomData<T>,
}

impl<T> Clone for ClapMcpHandler<T> {
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Subcommand> ClapMcpHandler<T> {
    fn new(handler: Option<CommandHandler<T>>) -> Self {
        Self {
            handler: handler.map(Arc::new),
            _phantom: PhantomData,
        }
    }
}

impl<T: Subcommand> ClapMcpHandler<T> {
    fn extract_subcommands() -> Vec<Tool> {
        let cmd = T::augment_subcommands(clap::Command::new("mcp"));
        let mut tools = Vec::new();
        
        for subcommand in cmd.get_subcommands() {
            let name = subcommand.get_name().to_string();
            let description = subcommand.get_about()
                .map(|s| s.to_string())
                .unwrap_or_default();
            
            let mut properties = HashMap::new();
            let mut required = Vec::new();
            
            // Extract arguments
            for arg in subcommand.get_arguments() {
                if arg.is_hide_set() || arg.get_id() == "help" || arg.get_id() == "version" {
                    continue;
                }
                
                let arg_name = arg.get_id().to_string();
                let arg_type = if arg.get_num_args().map(|r| r.min_values()).unwrap_or(0) == 0 {
                    "boolean"
                } else {
                    // For now, default to string. A more sophisticated type detection
                    // would require runtime information about the value parser
                    "string"
                };
                
                let mut schema = json!({
                    "type": arg_type
                });
                
                if let Some(help) = arg.get_help() {
                    schema["description"] = json!(help.to_string());
                }
                
                properties.insert(arg_name.clone(), schema);
                
                if arg.is_required_set() {
                    required.push(arg_name);
                }
            }
            
            let input_schema = json!({
                "type": "object",
                "properties": properties,
                "required": required
            });
            
            tools.push(Tool {
                name: name.into(),
                description: Some(description.into()),
                input_schema: Arc::new(object(input_schema)),
                annotations: None,
            });
        }
        
        tools
    }
}

impl<T: Subcommand + Send + Sync + 'static> ServerHandler for ClapMcpHandler<T> {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "clap-mcp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: None,
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            let tools = Self::extract_subcommands();
            Ok(ListToolsResult { 
                tools,
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            let tool_name = request.name.to_string();
            let arguments = request.arguments.unwrap_or_default();
            
            // Build command line arguments
            // First arg should be the program name, then the subcommand
            let mut args = vec!["mcp".to_string(), tool_name.clone()];
            
            for (key, value) in arguments {
                match value {
                    serde_json::Value::Bool(b) => {
                        if b {
                            args.push(format!("--{}", key));
                        }
                        // Skip false boolean flags
                    }
                    serde_json::Value::String(s) => {
                        args.push(format!("--{}", key));
                        args.push(s);
                    }
                    serde_json::Value::Number(n) => {
                        args.push(format!("--{}", key));
                        args.push(n.to_string());
                    }
                    _ => {
                        args.push(format!("--{}", key));
                        args.push(value.to_string());
                    }
                }
            }
            
            // Parse the arguments into a subcommand
            let cmd = T::augment_subcommands(clap::Command::new("mcp"));
            match cmd.try_get_matches_from(&args) {
                Ok(matches) => {
                    match T::from_arg_matches(&matches) {
                        Ok(subcommand) => {
                            // Use the handler if provided
                            if let Some(handler) = &self.handler {
                                match handler(subcommand) {
                                    Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
                                    Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
                                }
                            } else {
                                Ok(CallToolResult::error(vec![Content::text(
                                    "No command handler provided. The CLI must provide a handler function to execute commands in MCP mode."
                                )]))
                            }
                        }
                        Err(e) => {
                            Err(McpError::invalid_params(format!("Failed to parse subcommand: {}", e), None))
                        }
                    }
                }
                Err(e) => {
                    Err(McpError::invalid_params(format!("Invalid arguments: {}", e), None))
                }
            }
        }
    }
}