# Dragon Queen Server - Simple Deployment (No Nginx Required)

## Quick Setup

Since your nginx doesn't have the stream module, we'll run the game server directly on port 443.

### 1. Build the server (truly headless - zero GUI dependencies!)

The server has been optimized to build without any graphics libraries (no Wayland, X11, or rendering dependencies).

```bash
cd /root/dragon_queen_3d

# Pull latest changes
git pull

# Clean build to ensure no old dependencies
cargo clean

# Build the server in release mode
cargo build --release --bin dragon_queen_server
```

This should complete successfully on a headless server without needing to install any graphics packages!

### 2. Allow the binary to bind to privileged ports
sudo setcap 'cap_net_bind_service=+ep' target/release/dragon_queen_server
```

### 2. Run the server

```bash
# Now you can run without sudo
./target/release/dragon_queen_server
```

### 3. Open firewall

```bash
sudo ufw allow 443/udp
```

### 4. Connect from client

Use: `gameserver.fri3dl.dev:443`

## Running as a Systemd Service

Create `/etc/systemd/system/dragon-queen-server.service`:

```ini
[Unit]
Description=Dragon Queen Game Server
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/root/dragon_queen_3d
ExecStart=/root/dragon_queen_3d/target/release/dragon_queen_server
Restart=always
RestartSec=10

# Grant capability to bind to port 443
AmbientCapabilities=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
```

Then:
```bash
sudo systemctl daemon-reload
sudo systemctl enable dragon-queen-server
sudo systemctl start dragon-queen-server
sudo systemctl status dragon-queen-server
```

## Check if it's working

```bash
# Server should be listening on port 443
sudo ss -ulnp | grep :443

# Check logs
sudo journalctl -u dragon-queen-server -f
```

## Notes

- No nginx required!
- Server runs directly on port 443 UDP
- Uses Linux capabilities instead of running as root
- Simpler and more efficient than proxying through nginx
