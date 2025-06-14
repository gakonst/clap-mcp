# Contributing to clap-mcp

Thanks for your interest in contributing to clap-mcp!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/clap-mcp.git`
3. Create a feature branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests: `cargo test --workspace`
6. Submit a pull request

## Development

```bash
# Run tests
cargo test --workspace

# Run examples
cargo run --example calculator -- add --a 5 --b 3
cargo run --example calculator -- --mcp

# Check formatting
cargo fmt --all -- --check

# Run lints
cargo clippy --all-features --all-targets
```

## Guidelines

- Add tests for new functionality
- Update documentation and examples
- Follow existing code style
- Keep commits focused and atomic
- Write clear commit messages

## Testing MCP Functionality

The `test_calculator_mcp.sh` script provides a basic test of MCP functionality:

```bash
./test_calculator_mcp.sh
```

## Project Structure

- `clap-mcp/`: Main library crate
- `clap-mcp-derive/`: Derive macro implementation
- `examples/`: Example CLIs that use clap-mcp

## Questions?

Feel free to open an issue for questions or discussions.