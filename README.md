# clap-mcp

Turn any Clap CLI into an MCP server with a simple derive macro.

## Installation

```toml
[dependencies]
clap-mcp = "0.1"
```

## Quick Start

Here's a complete calculator example that works as both a CLI and MCP server:

```rust
use clap::{Parser, Subcommand};
use clap_mcp::McpMode;

#[derive(Parser, McpMode)]
#[command(name = "calculator")]
#[command(about = "A simple calculator CLI that can also run as an MCP server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Run as MCP server instead of CLI
    #[arg(long)]
    #[mcp(mode_flag)]
    mcp: bool,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Add two numbers
    Add {
        #[arg(short, long)]
        a: f64,
        #[arg(short, long)]
        b: f64,
    },
    
    /// Multiply two numbers
    Multiply {
        #[arg(long)]
        value1: f64,
        #[arg(long)]
        value2: f64,
    },
}

fn execute_command(cmd: Commands) -> Result<String, String> {
    match cmd {
        Commands::Add { a, b } => {
            Ok(format!("{} + {} = {}", a, b, a + b))
        }
        Commands::Multiply { value1, value2 } => {
            Ok(format!("{} * {} = {}", value1, value2, value1 * value2))
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    if cli.mcp {
        // Run as MCP server - automatically exposes all commands as tools
        cli.run_mcp_server_with_handler(execute_command)?;
    } else {
        // Run as normal CLI
        match execute_command(cli.command.expect("Subcommand required")) {
            Ok(output) => println!("{}", output),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

Now your CLI works both ways:

```bash
# Traditional CLI mode
$ calculator add -a 10 -b 32
10 + 32 = 42

$ calculator multiply --value1 7 --value2 6
7 * 6 = 42

# MCP server mode (stdio)
$ calculator --mcp
# Server is now running, ready for MCP clients to connect
```

## Real Example: Cast

Here's how you'd make Cast (Foundry's CLI) work as an MCP server:

```rust
#[derive(Parser, McpMode)]
struct Cast {
    #[command(subcommand)]
    cmd: CastCommand,
    
    #[arg(long)]
    #[mcp(mode_flag)]
    mcp: bool,
}

// That's it. All Cast commands are now available as MCP tools.
```

## How It Works

The `#[derive(McpMode)]` macro:
- Adds a `run_mcp_server()` method to your CLI
- Converts each subcommand into an MCP tool
- Maps CLI arguments to tool parameters with proper types
- Preserves all existing CLI functionality

## License

MIT OR Apache-2.0