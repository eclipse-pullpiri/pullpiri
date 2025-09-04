#!/bin/bash
#
# Simple demonstration of PICCOLO clustering functionality
# This script shows how the clustering components work together
#

set -e

echo "🚀 PICCOLO Clustering System Demo"
echo "=================================="

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo ""
echo -e "${BLUE}Step 1: Building clustering components...${NC}"
export PATH="$HOME/.cargo/bin:$PATH"

# Build the core clustering components
echo "Building common library..."
cargo build --manifest-path=src/common/Cargo.toml --quiet

echo "Building API Server..."
cargo build --manifest-path=src/server/apiserver/Cargo.toml --quiet

echo "Building NodeAgent..."
cargo build --manifest-path=src/agent/nodeagent/Cargo.toml --quiet

echo -e "${GREEN}✓ All components built successfully${NC}"

echo ""
echo -e "${BLUE}Step 2: Running clustering tests...${NC}"

echo "Testing API Server clustering functionality..."
echo "  • Node registry tests..."
cargo test --lib --manifest-path=src/server/apiserver/Cargo.toml --quiet node::registry::tests
echo "  • Node manager tests..."  
cargo test --lib --manifest-path=src/server/apiserver/Cargo.toml --quiet node::manager::tests
echo "  • Node status tests..."
cargo test --lib --manifest-path=src/server/apiserver/Cargo.toml --quiet node::status::tests
echo "  • gRPC receiver tests..."
cargo test --lib --manifest-path=src/server/apiserver/Cargo.toml --quiet grpc::receiver::tests

echo "Testing NodeAgent clustering functionality..."
cargo test cluster --manifest-path=src/agent/nodeagent/Cargo.toml --quiet

echo -e "${GREEN}✓ All clustering tests passed${NC}"

echo ""
echo -e "${BLUE}Step 3: Validating protocol definitions...${NC}"

# Check that the proto files are properly structured
echo "Checking nodeagent.proto..."
if grep -q "rpc RegisterNode" src/common/proto/nodeagent.proto; then
    echo "✓ NodeAgent registration service found"
fi

if grep -q "rpc Heartbeat" src/common/proto/nodeagent.proto; then
    echo "✓ NodeAgent heartbeat service found"
fi

if grep -q "rpc ReportStatus" src/common/proto/nodeagent.proto; then
    echo "✓ NodeAgent status reporting service found"
fi

echo "Checking apiserver.proto..."
if grep -q "rpc GetNodes" src/common/proto/apiserver.proto; then
    echo "✓ API Server node listing service found"
fi

if grep -q "rpc RegisterNode" src/common/proto/apiserver.proto; then
    echo "✓ API Server registration service found"
fi

if grep -q "rpc GetTopology" src/common/proto/apiserver.proto; then
    echo "✓ API Server topology service found"
fi

echo ""
echo -e "${GREEN}🎉 PICCOLO Clustering Demo Complete!${NC}"
echo ""
echo "Summary of implemented features:"
echo "• Node registration and authentication"
echo "• Heartbeat mechanism for health monitoring"
echo "• Status reporting with resource metrics"
echo "• Cluster topology management"
echo "• gRPC-based communication protocol"
echo "• Comprehensive unit test coverage"
echo ""
echo "The clustering system is ready for integration!"
echo ""
echo "Next steps:"
echo "1. Integrate with main application workflows"
echo "2. Add configuration management"
echo "3. Deploy in embedded environment"
echo ""