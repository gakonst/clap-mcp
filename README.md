# clap-mcp

Turn any Clap CLI into an MCP server with a simple derive macro.

## Installation

```toml
[dependencies]
clap-mcp = "0.1"
```

## Quick Start

```rust
use clap::{Parser, Subcommand};
use clap_mcp::McpMode;

#[derive(Parser, McpMode)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Run as MCP server
    #[arg(long)]
    #[mcp(mode_flag)]
    mcp: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Multiply two numbers
    Mul { a: f64, b: f64 },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    if cli.mcp {
        cli.run_mcp_server()?;
    } else {
        match cli.command {
            Commands::Mul { a, b } => println!("{}", a * b),
        }
    }
    
    Ok(())
}
```

Now your CLI works as both a regular CLI and an MCP server:

```bash
# CLI mode
$ mycli mul --a 3 --b 4
12

# MCP server mode
$ mycli --mcp
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