#!/bin/bash

# Convert FBX files in assets_to_convert to a single GLB file
# Usage: ./scripts/convert_assets.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

blender --background --python "$SCRIPT_DIR/convert_assets.py"
