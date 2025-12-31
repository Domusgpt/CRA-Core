#!/bin/bash
set -e

echo "=== CRA Comparative Test ==="
echo ""

# Build CRA MCP server
echo "Building CRA MCP server..."
cargo build --release -p cra-mcp

# Create output dirs
mkdir -p without-cra/output with-cra/output

# Copy task to both
cp TASK.md without-cra/
cp TASK.md with-cra/

echo ""
echo "=== Running WITHOUT CRA ==="
echo "Starting agent in without-cra/ directory..."
cd without-cra
# Claude Code would run here with no MCP
# claude --print "Complete the task in TASK.md"
cd ..

echo ""
echo "=== Running WITH CRA ==="
echo "Starting agent in with-cra/ directory..."
cd with-cra
# Claude Code would run here WITH CRA MCP
# claude --print "Complete the task in TASK.md. Use CRA tools."
cd ..

echo ""
echo "=== Compare Results ==="
echo "Without CRA output:"
ls -la without-cra/output/ 2>/dev/null || echo "  No output yet"
echo ""
echo "With CRA output:"
ls -la with-cra/output/ 2>/dev/null || echo "  No output yet"
