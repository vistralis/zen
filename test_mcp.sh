#!/bin/bash
# Quick test for zen MCP server — uses relative paths and $PWD

echo "Testing MCP server..."
(
  # Initialize
  echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}},"id":1}'
  sleep 0.5
  # List tools
  echo '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}'
  sleep 0.5
  # Test get_default_environment with current directory
  echo "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"get_default_environment\",\"arguments\":{\"project_path\":\"$PWD\"}},\"id\":3}"
  sleep 0.5
  # List environments
  echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"list_environments","arguments":{}},"id":4}'
  sleep 1
) | zen mcp
