pub use clap_mcp_derive::McpMode;

pub mod test_client;

use clap::Subcommand;
use rmcp::{
    handler::server::ServerHandler,
    model::*,
    service::{RequestContext, RoleServer},
    Error as McpError,
};
use serde_json::json;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;

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

impl<T: Subcommand + Send + Sync + Clone + 'static> Default for McpServer<T> {
    fn default() -> Self {
        Self {
            handler: None,
            _phantom: PhantomData,
        }
    }
}

impl<T: Subcommand + Send + Sync + Clone + 'static> McpServer<T> {
    pub fn new() -> Self {
        Self::default()
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

        let server =
            axum::serve(listener, router.into_make_service()).with_graceful_shutdown(async move {
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
            let description = subcommand
                .get_about()
                .map(|s| s.to_string())
                .unwrap_or_default();

            let mut properties = HashMap::new();
            let mut required = Vec::new();

            // Extract arguments
            let mut positional_count = 0;
            for arg in subcommand.get_arguments() {
                if arg.is_hide_set() || arg.get_id() == "help" || arg.get_id() == "version" {
                    continue;
                }

                let arg_name = arg.get_id().to_string();
                let is_positional = arg.get_long().is_none() && arg.get_short().is_none();

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

                // Add metadata to indicate positional arguments
                if is_positional {
                    schema["x-positional"] = json!(true);
                    // Use the index if available, otherwise use a counter
                    let position = arg.get_index().unwrap_or_else(|| {
                        let pos = positional_count;
                        positional_count += 1;
                        pos
                    });
                    schema["x-position"] = json!(position);
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

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = Self::extract_subcommands();
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_name = request.name.to_string();
        let arguments = request.arguments.unwrap_or_default();

        // Get the tool definition to check which arguments are positional
        let tools = Self::extract_subcommands();
        let tool = tools.iter().find(|t| t.name == tool_name);

        // Build command line arguments
        // First arg should be the program name, then the subcommand
        let mut args = vec!["mcp".to_string(), tool_name.clone()];

        // Separate positional and named arguments
        let mut positional_args: Vec<(String, serde_json::Value, usize)> = Vec::new();
        let mut named_args = HashMap::new();

        for (key, value) in arguments {
            // Check if this argument is positional by looking at the tool schema
            let is_positional = tool
                .and_then(|t| {
                    t.input_schema
                        .get("properties")
                        .and_then(|props| props.get(&key))
                        .and_then(|schema| schema.get("x-positional"))
                        .and_then(|v| v.as_bool())
                })
                .unwrap_or(false);

            if is_positional {
                let position =
                    tool.and_then(|t| {
                        t.input_schema
                            .get("properties")
                            .and_then(|props| props.get(&key))
                            .and_then(|schema| schema.get("x-position"))
                            .and_then(|v| v.as_u64())
                    })
                    .unwrap_or(positional_args.len() as u64) as usize;

                positional_args.push((key, value, position));
            } else {
                named_args.insert(key, value);
            }
        }

        // Sort positional arguments by their position
        positional_args.sort_by_key(|&(_, _, pos)| pos);

        // Add positional arguments first (without -- prefix)
        for (_, value, _) in positional_args {
            match value {
                serde_json::Value::String(s) => args.push(s),
                serde_json::Value::Number(n) => args.push(n.to_string()),
                serde_json::Value::Bool(b) => args.push(b.to_string()),
                _ => args.push(value.to_string()),
            }
        }

        // Then add named arguments with -- prefix
        for (key, value) in named_args {
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
                                Ok(output) => {
                                    Ok(CallToolResult::success(vec![Content::text(output)]))
                                }
                                Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
                            }
                        } else {
                            Ok(CallToolResult::error(vec![Content::text(
                                    "No command handler provided. The CLI must provide a handler function to execute commands in MCP mode."
                                )]))
                        }
                    }
                    Err(e) => Err(McpError::invalid_params(
                        format!("Failed to parse subcommand: {}", e),
                        None,
                    )),
                }
            }
            Err(e) => Err(McpError::invalid_params(
                format!("Invalid arguments: {}", e),
                None,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Subcommand;
    use rmcp::transport::sse_server::{SseServer, SseServerConfig};
    use serde_json::json;
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    #[derive(Subcommand, Clone)]
    enum TestCommands {
        /// Add two numbers
        Add {
            /// First number
            #[arg(short, long)]
            a: i32,
            /// Second number
            #[arg(short, long)]
            b: i32,
        },
        /// Subtract two numbers
        Subtract {
            /// Number to subtract from
            #[arg(long)]
            minuend: i32,
            /// Number to subtract
            #[arg(long)]
            subtrahend: i32,
        },
        /// Multiply two numbers
        Multiply {
            /// First value
            #[arg(long)]
            value1: i32,
            /// Second value
            #[arg(long)]
            value2: i32,
        },
        /// Divide two numbers
        Divide {
            /// Number to divide
            #[arg(long)]
            dividend: i32,
            /// Number to divide by
            #[arg(long)]
            divisor: i32,
        },
        /// Say hello to someone
        Hello {
            /// Name to greet
            #[arg(long)]
            name: String,
            /// Whether to be excited
            #[arg(long)]
            excited: bool,
        },
    }

    fn execute_test_command(cmd: TestCommands) -> Result<String, String> {
        match cmd {
            TestCommands::Add { a, b } => Ok(format!("{} + {} = {}", a, b, a + b)),
            TestCommands::Subtract {
                minuend,
                subtrahend,
            } => Ok(format!(
                "{} - {} = {}",
                minuend,
                subtrahend,
                minuend - subtrahend
            )),
            TestCommands::Multiply { value1, value2 } => {
                Ok(format!("{} * {} = {}", value1, value2, value1 * value2))
            }
            TestCommands::Divide { dividend, divisor } => {
                if divisor == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(format!(
                        "{} ÷ {} = {}",
                        dividend,
                        divisor,
                        dividend / divisor
                    ))
                }
            }
            TestCommands::Hello { name, excited } => {
                if excited {
                    Ok(format!("Hello, {}!!!", name))
                } else {
                    Ok(format!("Hello, {}.", name))
                }
            }
        }
    }

    // Positional arguments test structures
    #[derive(Subcommand, Clone)]
    enum PositionalCommands {
        /// Convert text from UTF-8
        FromUtf8 {
            /// The text to convert
            text: String,

            /// Optional second positional argument
            optional: Option<String>,
        },

        /// Example with mixed args
        Mixed {
            /// First positional
            input: String,

            /// A flag
            #[arg(short, long)]
            verbose: bool,

            /// Second positional  
            output: String,
        },
    }

    fn execute_positional_command(cmd: PositionalCommands) -> Result<String, String> {
        match cmd {
            PositionalCommands::FromUtf8 { text, optional } => {
                let hex = text
                    .chars()
                    .map(|c| format!("{:02x}", c as u8))
                    .collect::<String>();
                Ok(format!("0x{} (optional: {:?})", hex, optional))
            }
            PositionalCommands::Mixed {
                input,
                verbose,
                output,
            } => Ok(format!(
                "Input: {}, Output: {}, Verbose: {}",
                input, output, verbose
            )),
        }
    }

    /// Get an available port
    async fn get_available_port() -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    /// Start an in-process MCP server
    async fn start_in_process_server<T: Subcommand + Send + Sync + Clone + 'static>(
        handler: CommandHandler<T>,
    ) -> Result<(CancellationToken, u16), Box<dyn std::error::Error>> {
        let port = get_available_port().await;
        let addr = format!("127.0.0.1:{}", port).parse()?;
        let handler = ClapMcpHandler::<T>::new(Some(handler));

        let config = SseServerConfig {
            bind: addr,
            sse_path: "/sse".to_string(),
            post_path: "/message".to_string(),
            ct: CancellationToken::new(),
            sse_keep_alive: None,
        };

        let (sse_server, router) = SseServer::new(config);
        let ct = sse_server.config.ct.clone();

        let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;

        let server_ct = ct.child_token();
        let server =
            axum::serve(listener, router.into_make_service()).with_graceful_shutdown(async move {
                server_ct.cancelled().await;
            });

        tokio::spawn(async move {
            if let Err(e) = server.await {
                eprintln!("MCP SSE server error: {}", e);
            }
        });

        let _service_ct = sse_server.with_service(move || handler.clone());

        // Wait for server to be ready by attempting connection
        let addr_str = format!("127.0.0.1:{}", port);
        for _ in 0..50 {
            match tokio::net::TcpStream::connect(&addr_str).await {
                Ok(_) => break,
                Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
            }
        }

        Ok((ct, port))
    }

    #[tokio::test]
    async fn test_calculator_mcp() {
        use crate::test_client::McpTestClient;

        // Start server
        let (ct, port) = start_in_process_server::<TestCommands>(Box::new(execute_test_command))
            .await
            .expect("Failed to start server");

        // Connect to server
        let client = McpTestClient::connect(&format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect to server");

        // List tools
        let tools = client.list_tools().await.expect("Failed to list tools");
        assert_eq!(tools.len(), 5); // add, subtract, multiply, divide, hello

        // Test add command
        let result = client
            .call_tool("add", Some(json!({ "a": 10, "b": 32 })))
            .await
            .expect("Failed to call add");
        let text = McpTestClient::extract_text(&result).expect("No text in result");
        assert_eq!(text, "10 + 32 = 42");

        // Test multiply command
        let result = client
            .call_tool("multiply", Some(json!({ "value1": 7, "value2": 6 })))
            .await
            .expect("Failed to call multiply");
        let text = McpTestClient::extract_text(&result).expect("No text in result");
        assert_eq!(text, "7 * 6 = 42");

        // Test divide command with error
        let result = client
            .call_tool("divide", Some(json!({ "dividend": 10, "divisor": 0 })))
            .await
            .expect("Failed to call divide");
        assert!(result.is_error.unwrap_or(false));
        let text = McpTestClient::extract_text(&result).expect("No text in error");
        assert!(text.contains("Division by zero"));

        // Test hello command
        let result = client
            .call_tool("hello", Some(json!({ "name": "Test", "excited": true })))
            .await
            .expect("Failed to call hello");
        let text = McpTestClient::extract_text(&result).expect("No text in result");
        assert_eq!(text, "Hello, Test!!!");

        // Shutdown
        client.shutdown().await.expect("Failed to shutdown client");
        ct.cancel();
    }

    #[tokio::test]
    async fn test_missing_arguments() {
        use crate::test_client::McpTestClient;

        // Start server
        let (ct, port) = start_in_process_server::<TestCommands>(Box::new(execute_test_command))
            .await
            .expect("Failed to start server");

        let client = McpTestClient::connect(&format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect to server");

        // Try calling add without any arguments - the call itself succeeds but returns an error result
        let result = client.call_tool("add", Some(json!({}))).await;

        // The call should succeed but return an error in the result
        match result {
            Ok(call_result) => {
                // We expect this to be an error response (missing required arguments)
                assert!(
                    call_result.is_error.unwrap_or(false),
                    "Expected error for missing arguments"
                );
            }
            Err(e) => {
                // This is actually expected - the MCP error for invalid arguments
                assert!(
                    e.to_string().contains("Invalid arguments")
                        || e.to_string().contains("required arguments"),
                    "Unexpected error: {}",
                    e
                );
            }
        }

        // Try calling add with only one argument
        let result = client.call_tool("add", Some(json!({ "a": 5 }))).await;
        match result {
            Ok(call_result) => {
                assert!(
                    call_result.is_error.unwrap_or(false),
                    "Expected error for missing b argument"
                );
            }
            Err(e) => {
                assert!(
                    e.to_string().contains("Invalid arguments")
                        || e.to_string().contains("required arguments"),
                    "Unexpected error: {}",
                    e
                );
            }
        }

        // Shutdown
        client.shutdown().await.expect("Failed to shutdown client");
        ct.cancel();
    }

    #[tokio::test]
    async fn test_positional_args() {
        use crate::test_client::McpTestClient;

        // Start server
        let (ct, port) =
            start_in_process_server::<PositionalCommands>(Box::new(execute_positional_command))
                .await
                .expect("Failed to start server");

        let client = McpTestClient::connect(&format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect to server");

        // Test from-utf8 with required positional argument
        let result = client
            .call_tool("from-utf8", Some(json!({ "text": "hello" })))
            .await
            .expect("Failed to call from-utf8");
        let text = McpTestClient::extract_text(&result).expect("No text in result");
        assert_eq!(text, "0x68656c6c6f (optional: None)");

        // Test from-utf8 with optional positional argument
        let result = client
            .call_tool(
                "from-utf8",
                Some(json!({ "text": "hello", "optional": "world" })),
            )
            .await
            .expect("Failed to call from-utf8");
        let text = McpTestClient::extract_text(&result).expect("No text in result");
        assert_eq!(text, "0x68656c6c6f (optional: Some(\"world\"))");

        // Test mixed command with multiple positionals and flags
        let result = client
            .call_tool(
                "mixed",
                Some(json!({ "input": "foo.txt", "output": "bar.txt", "verbose": true })),
            )
            .await
            .expect("Failed to call mixed");
        let text = McpTestClient::extract_text(&result).expect("No text in result");
        assert_eq!(text, "Input: foo.txt, Output: bar.txt, Verbose: true");

        // Shutdown
        client.shutdown().await.expect("Failed to shutdown client");
        ct.cancel();
    }

    #[tokio::test]
    async fn test_http_client_operations() {
        use crate::test_client::McpTestClient;

        // Start server
        let (ct, port) = start_in_process_server::<TestCommands>(Box::new(execute_test_command))
            .await
            .expect("Failed to start server");

        let client = McpTestClient::connect(&format!("127.0.0.1:{}", port))
            .await
            .expect("Failed to connect to server");

        // List tools and verify count
        let tools = client.list_tools().await.expect("Failed to list tools");
        assert_eq!(tools.len(), 5); // add, subtract, multiply, divide, hello

        // Verify tool metadata
        let add_tool = tools
            .iter()
            .find(|t| t.name == "add")
            .expect("Add tool not found");
        assert!(add_tool
            .description
            .as_ref()
            .unwrap()
            .contains("Add two numbers"));

        // Test a sequence of operations
        let operations = vec![
            ("add", json!({ "a": 100, "b": 200 }), "100 + 200 = 300"),
            (
                "subtract",
                json!({ "minuend": 50, "subtrahend": 20 }),
                "50 - 20 = 30",
            ),
            (
                "multiply",
                json!({ "value1": 11, "value2": 11 }),
                "11 * 11 = 121",
            ),
            (
                "divide",
                json!({ "dividend": 100, "divisor": 4 }),
                "100 ÷ 4 = 25",
            ),
        ];

        for (op, args, expected) in operations {
            let result = client
                .call_tool(op, Some(args))
                .await
                .unwrap_or_else(|_| panic!("Failed to call {}", op));
            let text = McpTestClient::extract_text(&result)
                .unwrap_or_else(|| panic!("No text in {} result", op));
            assert_eq!(text, expected);
        }

        // Shutdown
        client.shutdown().await.expect("Failed to shutdown client");
        ct.cancel();
    }
}
