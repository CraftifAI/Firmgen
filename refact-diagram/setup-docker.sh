#!/bin/bash

echo "🐳 Building Refact Diagram Docker image..."
echo ""

# Build the image
docker build -t refact-diagram .

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Image built successfully!"
    echo ""
    echo "🚀 To run the application:"
    echo "   docker run -p 3000:80 refact-diagram"
    echo ""
    echo "🌐 Then open: http://localhost:3000"
    echo ""
    echo "📦 To share with others:"
    echo "   docker save refact-diagram > refact-diagram.tar"
    echo ""
    echo "📋 Recipients can load and run with:"
    echo "   docker load < refact-diagram.tar"
    echo "   docker run -p 3000:80 refact-diagram"
    echo ""
    echo "🎯 Or use docker-compose:"
    echo "   docker-compose up -d"
else
    echo "❌ Build failed!"
    exit 1
fi


