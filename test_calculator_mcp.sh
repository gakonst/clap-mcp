#!/bin/bash

# Test calling calculator functions via MCP

# Call add function: 10 + 32
echo "=== Testing calculator add via MCP ==="
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"initialized"}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"add","arguments":{"a":"10","b":"32"}},"id":2}\n' | ./target/debug/examples/calculator --mcp 2>&1 | grep -E "(result|error)" | tail -1

echo ""
echo "=== Testing calculator multiply via MCP ==="
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"initialized"}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"multiply","arguments":{"value1":"7","value2":"6"}},"id":2}\n' | ./target/debug/examples/calculator --mcp 2>&1 | grep -E "(result|error)" | tail -1

echo ""
echo "=== Testing hello via MCP ==="
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"initialized"}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"hello","arguments":{"name":"MCP User","excited":"true"}},"id":2}\n' | ./target/debug/examples/calculator --mcp 2>&1 | grep -E "(result|error)" | tail -1