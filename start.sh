#!/bin/bash

echo "🚀 Starting Refact ESP32 Agent..."

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker first."
    exit 1
fi

# Check if docker-compose is available
if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "❌ docker-compose not found. Please install docker-compose first."
    exit 1
fi

# Build and start
echo "📦 Building Docker image (this may take a while the first time)..."
docker-compose up --build

echo ""
echo "✅ Done! Open http://localhost:8008 in your browser"

