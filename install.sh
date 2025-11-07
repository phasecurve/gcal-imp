#!/bin/bash

set -e

echo "Building gcal-imp..."
make prod-build

echo "Installing gcal to local bin directory..."
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp target/release/gcal-imp "$INSTALL_DIR/gcal"

echo "Installation complete! Run 'gcal' to start the application."
if [ ":$PATH:" != *":$HOME/.local/bin:"* ]; then
    echo "Note: Add ~/.local/bin to your PATH if not already present"
fi
