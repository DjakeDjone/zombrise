#!/bin/bash
# Quick setup script for Dragon Queen server with nginx UDP proxy

set -e

echo "🐉 Dragon Queen Server Setup"
echo "=============================="
echo ""

# Check if nginx is installed
if ! command -v nginx &> /dev/null; then
    echo "❌ nginx is not installed. Please install it first:"
    echo "   sudo apt install nginx  # Ubuntu/Debian"
    echo "   sudo yum install nginx  # CentOS/RHEL"
    exit 1
fi

echo "✓ nginx is installed"

# Check if nginx has stream module
if ! nginx -V 2>&1 | grep -q "stream"; then
    echo "⚠️  Warning: nginx may not have stream module compiled in"
    echo "   Most distributions include it by default"
fi

PROJECT_DIR="/home/benjamin-f/Documents/dev/game/dragon_queen_3d"

# Add stream config to nginx.conf if not already present
if ! grep -q "nginx-stream.conf" /etc/nginx/nginx.conf; then
    echo ""
    echo "📝 Adding stream configuration to nginx.conf..."
    echo "   You'll need to manually add this line to /etc/nginx/nginx.conf:"
    echo ""
    echo "   include $PROJECT_DIR/nginx-stream.conf;"
    echo ""
    echo "   Add it at the TOP LEVEL (outside the http block)"
    echo ""
    read -p "Press Enter when you've added it..."
else
    echo "✓ Stream configuration already included"
fi

# Test nginx config
echo ""
echo "🧪 Testing nginx configuration..."
if sudo nginx -t; then
    echo "✓ nginx configuration is valid"
else
    echo "❌ nginx configuration has errors. Please fix them before continuing."
    exit 1
fi

# Reload nginx
echo ""
echo "🔄 Reloading nginx..."
sudo systemctl reload nginx
echo "✓ nginx reloaded"

# Check firewall
echo ""
echo "🔥 Checking firewall..."
if command -v ufw &> /dev/null; then
    if sudo ufw status | grep -q "443/udp"; then
        echo "✓ Port 443/udp is open"
    else
        echo "⚠️  Port 443/udp is not open. Opening it now..."
        sudo ufw allow 443/udp
        echo "✓ Port 443/udp opened"
    fi
elif command -v firewall-cmd &> /dev/null; then
    if sudo firewall-cmd --list-ports | grep -q "443/udp"; then
        echo "✓ Port 443/udp is open"
    else
        echo "⚠️  Port 443/udp is not open. Opening it now..."
        sudo firewall-cmd --permanent --add-port=443/udp
        sudo firewall-cmd --reload
        echo "✓ Port 443/udp opened"
    fi
else
    echo "⚠️  Could not detect firewall. Make sure port 443/udp is open manually."
fi

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "1. Start the game server: cd $PROJECT_DIR && cargo run --bin dragon_queen_server --release"
echo "2. Connect from client using: gameserver.fri3dl.dev:443"
echo ""
echo "To check if everything is working:"
echo "  sudo netstat -ulnp | grep :443   # nginx should be listening"
echo "  sudo netstat -ulnp | grep :5000  # game server should be listening"
