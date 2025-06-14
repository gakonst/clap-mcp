use clap::{Parser, Subcommand};
use clap_mcp::McpMode;

#[derive(Parser, McpMode)]
#[command(name = "calculator")]
#[command(about = "A simple calculator CLI that can also run as an MCP server")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run as MCP server instead of CLI
    #[arg(long)]
    #[mcp(mode_flag)]
    mcp: bool,

    /// Port to run MCP HTTP server on (if not specified, uses stdio)
    #[arg(long, value_name = "PORT")]
    mcp_port: Option<u16>,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Addsss two numbers
    Add {
        /// First number
        #[arg(short, long)]
        a: f64,
        /// Second number
        #[arg(short, long)]
        b: f64,
    },

    /// Subtract two numbers
    Subtract {
        /// First number
        #[arg(short, long)]
        x: f64,
        /// Second number
        #[arg(short, long)]
        y: f64,
    },

    /// Multiply two numbers
    Multiply {
        /// First number
        #[arg(long)]
        value1: f64,
        /// Second number
        #[arg(long)]
        value2: f64,
    },

    /// Divide two numbers
    Divide {
        /// Dividend
        #[arg(long)]
        dividend: f64,
        /// Divisor
        #[arg(long)]
        divisor: f64,
    },

    /// Say hello to someone
    Hello {
        /// Name to greet
        #[arg(short, long)]
        name: String,
        /// Use enthusiastic greeting
        #[arg(short, long, default_value = "false")]
        excited: bool,
    },
}

fn execute_command(cmd: Commands) -> Result<String, String> {
    match cmd {
        Commands::Add { a, b } => Ok(format!("{} + {} = {}", a, b, a + b)),
        Commands::Subtract { x, y } => Ok(format!("{} - {} = {}", x, y, x - y)),
        Commands::Multiply { value1, value2 } => {
            Ok(format!("{} * {} = {}", value1, value2, value1 * value2))
        }
        Commands::Divide { dividend, divisor } => {
            if divisor == 0.0 {
                Err("Error: Division by zero!".to_string())
            } else {
                Ok(format!(
                    "{} / {} = {}",
                    dividend,
                    divisor,
                    dividend / divisor
                ))
            }
        }
        Commands::Hello { name, excited } => {
            if excited {
                Ok(format!("Hello, {}!!!", name))
            } else {
                Ok(format!("Hello, {}.", name))
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.mcp {
        // Run as MCP server
        if let Some(port) = cli.mcp_port {
            let addr = format!("127.0.0.1:{}", port).parse()?;
            cli.run_mcp_server_http_with_handler(addr, execute_command)?;
        } else {
            cli.run_mcp_server_with_handler(execute_command)?;
        }
    } else {
        // Run as normal CLI
        match execute_command(
            cli.command
                .expect("Subcommand required when not in MCP mode"),
        ) {
            Ok(output) => println!("{}", output),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
