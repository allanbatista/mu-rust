#!/bin/bash

# MU Online Connect Server - Quick Start Script

set -e

echo "🚀 MU Online Connect Server - Starting..."
echo ""

# Check if we're in the right directory
if [ ! -f "server/config/servers.toml" ]; then
    echo "❌ Error: Must be run from the rust/ directory"
    echo "   Current directory: $(pwd)"
    echo "   Expected: /path/to/mu-rust/rust/"
    exit 1
fi

# Set default environment variables
export CONFIG_PATH="${CONFIG_PATH:-server/config/servers.toml}"
export MONGODB_URI="${MONGODB_URI:-mongodb://localhost:27017}"
export DATABASE_NAME="${DATABASE_NAME:-mu_online}"
export SERVER_HOST="${SERVER_HOST:-0.0.0.0}"
export SERVER_PORT="${SERVER_PORT:-8080}"
export SESSION_EXPIRY_HOURS="${SESSION_EXPIRY_HOURS:-24}"
export RUST_LOG="${RUST_LOG:-info}"

echo "📋 Configuration:"
echo "   Config file: $CONFIG_PATH"
echo "   MongoDB: $MONGODB_URI"
echo "   Database: $DATABASE_NAME"
echo "   Server: $SERVER_HOST:$SERVER_PORT"
echo "   Log level: $RUST_LOG"
echo ""

# Check if MongoDB is accessible
echo "🔍 Checking MongoDB connection..."
if command -v mongosh &> /dev/null; then
    if mongosh "$MONGODB_URI" --eval "db.adminCommand('ping')" --quiet &> /dev/null; then
        echo "✅ MongoDB is accessible"
    else
        echo "⚠️  Warning: Cannot connect to MongoDB at $MONGODB_URI"
        echo "   The server will fail to start without MongoDB"
        echo ""
        echo "   To start MongoDB with Docker:"
        echo "   docker run -d -p 27017:27017 --name mongodb mongo:latest"
        echo ""
    fi
else
    echo "⚠️  mongosh not found, skipping MongoDB check"
fi

echo ""
echo "🏗️  Building and running server..."
echo ""

# Run the server
cargo run --manifest-path server/Cargo.toml
