#!/usr/bin/env python3
import subprocess
import json

# Start calculator in MCP mode
proc = subprocess.Popen(
    ['./target/debug/examples/calculator', '--mcp'],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

# Initialize
init_req = {"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"1.0"}},"id":1}
proc.stdin.write(json.dumps(init_req) + '\n')
proc.stdin.flush()

# Read init response
init_resp = json.loads(proc.stdout.readline())
print("Server initialized:", init_resp['result']['serverInfo'])

# Send initialized notification
proc.stdin.write('{"jsonrpc":"2.0","method":"initialized"}\n')
proc.stdin.flush()

# List tools
list_req = {"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}
proc.stdin.write(json.dumps(list_req) + '\n')
proc.stdin.flush()

# Read tools response
tools_resp = json.loads(proc.stdout.readline())
print("\nAvailable tools:")
for tool in tools_resp['result']['tools']:
    print(f"  - {tool['name']}: {tool.get('description', 'No description')}")

# Call add tool
add_req = {"jsonrpc":"2.0","method":"tools/call","params":{"name":"add","arguments":{"a":"10","b":"32"}},"id":3}
proc.stdin.write(json.dumps(add_req) + '\n')
proc.stdin.flush()

# Read result
add_resp = json.loads(proc.stdout.readline())
print("\nAdd result:", add_resp['result']['content'][0]['text'])

# Call multiply
mul_req = {"jsonrpc":"2.0","method":"tools/call","params":{"name":"multiply","arguments":{"value1":"7","value2":"6"}},"id":4}
proc.stdin.write(json.dumps(mul_req) + '\n')
proc.stdin.flush()

mul_resp = json.loads(proc.stdout.readline())
print("Multiply result:", mul_resp['result']['content'][0]['text'])

proc.terminate()