#!/bin/bash

echo "📦 Creating shareable Docker image package..."
echo ""

# Save the Docker image to a tar file
echo "Saving Docker image to refact-diagram.tar..."
docker save refact-diagram > refact-diagram.tar

if [ $? -eq 0 ]; then
    echo "✅ Docker image saved successfully!"
    echo ""
    echo "📁 File created: refact-diagram.tar"
    echo ""
    echo "📊 File size:"
    ls -lh refact-diagram.tar
    echo ""
    echo "🚀 Instructions for recipients:"
    echo "   1. Load the image: docker load < refact-diagram.tar"
    echo "   2. Run the container: docker run -p 3000:80 refact-diagram"
    echo "   3. Open browser: http://localhost:3000"
    echo ""
    echo "🎯 Alternative sharing options:"
    echo "   • Share the .tar file directly"
    echo "   • Upload to your organization's Docker registry"
    echo "   • Use docker-compose with the provided files"
else
    echo "❌ Failed to save Docker image!"
    exit 1
fi


