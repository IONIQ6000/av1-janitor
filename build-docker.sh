#!/bin/bash
# Build script for Docker image

set -e

echo "Building AV1 Re-encoding Daemon Docker image..."

# Build the image
docker build -t av1-reencoder:latest .

echo ""
echo "Build complete!"
echo ""
echo "Image: av1-reencoder:latest"
echo ""
echo "Next steps:"
echo "  1. Edit config.toml with your settings"
echo "  2. Edit docker-compose.yml with your media paths"
echo "  3. Run: docker-compose up -d"
echo "  4. Monitor: docker exec -it av1d av1top"
echo ""
