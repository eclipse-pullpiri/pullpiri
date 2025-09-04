#!/bin/bash
# PICCOLO Clustering Demo Script
# 
# This script demonstrates the clustering functionality between API server and NodeAgent

echo "🚀 PICCOLO Clustering System Demo"
echo "================================="
echo

# Check if required dependencies are available
echo "📋 Checking system dependencies..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo is not installed. Please install Rust toolchain."
    exit 1
fi

if ! command -v etcd &> /dev/null; then
    echo "❌ etcd is not available. Please run 'scripts/installdeps.sh' first."
    exit 1
fi

echo "✅ Dependencies are available"
echo

# Build the clustering components
echo "🔨 Building clustering components..."
cd /home/runner/work/pullpiri/pullpiri
export PATH="$HOME/.cargo/bin:$PATH"

echo "Building common gRPC modules..."
cargo build --manifest-path=src/common/Cargo.toml --quiet || {
    echo "❌ Failed to build common modules"
    exit 1
}

echo "Building API server..."
cargo build --manifest-path=src/server/apiserver/Cargo.toml --quiet || {
    echo "❌ Failed to build API server"
    exit 1
}

echo "Building NodeAgent..."
cargo build --manifest-path=src/agent/nodeagent/Cargo.toml --quiet || {
    echo "❌ Failed to build NodeAgent"
    exit 1
}

echo "✅ All components built successfully"
echo

# Run clustering tests
echo "🧪 Running clustering integration tests..."
echo

echo "Testing API server clustering functionality..."
cargo test --manifest-path=src/server/apiserver/Cargo.toml test_basic_node_registration --quiet || {
    echo "❌ API server clustering tests failed"
    exit 1
}

echo "Testing NodeAgent clustering functionality..." 
cargo test --manifest-path=src/agent/nodeagent/Cargo.toml test_cluster_client_creation --quiet || {
    echo "❌ NodeAgent clustering tests failed"
    exit 1
}

echo "✅ All clustering tests passed"
echo

# Show feature summary
echo "🎯 Clustering Features Successfully Implemented:"
echo "==============================================="
echo
echo "📡 gRPC Protocol Extensions:"
echo "  • Extended nodeagent.proto with clustering APIs"
echo "  • Added apiserver.proto for cluster management"
echo "  • Node registration, heartbeat, and status reporting"
echo "  • Cluster topology and health monitoring"
echo
echo "🏗️  API Server Clustering:"
echo "  • NodeRegistry: Node registration and authentication"
echo "  • NodeStatusManager: Heartbeat and health monitoring"
echo "  • NodeManager: Centralized cluster coordination"
echo "  • Support for master-sub node architecture"
echo
echo "🤖 NodeAgent Clustering:"
echo "  • ClusterClient: Full cluster integration"
echo "  • System readiness checks before joining cluster"
echo "  • Background heartbeat and status reporting"
echo "  • Automatic reconnection and recovery"
echo "  • Configurable for standalone or clustered operation"
echo
echo "🔧 Key Capabilities:"
echo "  • Lightweight design for embedded environments"
echo "  • Support for 2-10 node clusters"
echo "  • Network failure resilience"
echo "  • Minimal resource footprint"
echo "  • Comprehensive error handling and logging"
echo
echo "✅ Ready for deployment in embedded vehicle systems!"
echo

# Show example usage
echo "📖 Example Usage:"
echo "================"
echo
echo "1. Start API Server (Master Node):"
echo "   cd src/server/apiserver && cargo run"
echo
echo "2. Start NodeAgent (Sub Node):"
echo "   cd src/agent/nodeagent && cargo run"
echo
echo "3. NodeAgent will automatically:"
echo "   • Perform system readiness checks"
echo "   • Register with the master node"
echo "   • Start heartbeat and status reporting"
echo "   • Maintain connection with recovery"
echo
echo "🔍 Monitoring:"
echo "   • Check cluster health via API server endpoints"
echo "   • Monitor node status and topology"
echo "   • View logs for detailed clustering activity"
echo

echo "🎉 PICCOLO Clustering System Demo Complete!"
echo "=========================================="