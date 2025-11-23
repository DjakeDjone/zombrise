# Dragon Queen 3D Server Deployment Guide

## Architecture

```
Client → gameserver.fri3dl.dev:443 (UDP) → nginx → localhost:5000 (Game Server)
```

## Setup Instructions

### 1. Configure Nginx Stream Module

The nginx stream module is required for UDP proxying. Check if it's enabled:

```bash
nginx -V 2>&1 | grep stream
```

If you don't see `--with-stream`, you'll need to install/recompile nginx with stream support.

### 2. Add Stream Configuration to Nginx

Edit `/etc/nginx/nginx.conf` and add this **outside** the `http` block (at the top level):

```nginx
stream {
    include /home/benjamin-f/Documents/dev/game/dragon_queen_3d/nginx-stream.conf;
}
```

Or copy the stream config to a standard location:

```bash
sudo cp nginx-stream.conf /etc/nginx/streams-enabled/dragon_queen.conf

# Then in /etc/nginx/nginx.conf add:
stream {
    include /etc/nginx/streams-enabled/*.conf;
}
```

### 3. Update the Stream Config Path (if needed)

If you move the config file, update line 9 in the main nginx.conf to point to the correct location.

### 4. Test and Reload Nginx

```bash
# Test the configuration
sudo nginx -t

# If successful, reload nginx
sudo systemctl reload nginx

# Check status
sudo systemctl status nginx
```

### 5. Build and Run the Server

```bash
# Build the server (optimized)
cargo build --release --bin dragon_queen_server

# Run the server (it will listen on port 5000)
./target/release/dragon_queen_server
```

### 6. Configure Firewall

Make sure port 443 UDP is open on your server:

```bash
# For ufw
sudo ufw allow 443/udp

# For firewalld
sudo firewall-cmd --permanent --add-port=443/udp
sudo firewall-cmd --reload
```

## DNS Configuration

Make sure `gameserver.fri3dl.dev` points to your server's IP address:

```
gameserver.fri3dl.dev → A/AAAA record → Your Server IP
```

## Testing

### From the server itself:
```bash
# Test local connection
nc -u -v 127.0.0.1 5000
```

### From a client machine:
```bash
# Test nginx proxy
nc -u -v gameserver.fri3dl.dev 443
```

### Run the game client:
```bash
cargo run --bin dragon_queen_client
```

The client should now connect to `gameserver.fri3dl.dev:443` by default.

## Troubleshooting

1. **Connection refused**: Check if the game server is running on port 5000
2. **Permission denied**: Make sure nginx is running as root (required for port 443)
3. **No response**: Check firewall rules and DNS resolution
4. **Nginx won't start**: Verify stream module is enabled (`nginx -V`)

## Running as a System Service

Create `/etc/systemd/system/dragon-queen-server.service`:

```ini
[Unit]
Description=Dragon Queen 3D Game Server
After=network.target

[Service]
Type=simple
User=benjamin-f
WorkingDirectory=/home/benjamin-f/Documents/dev/game/dragon_queen_3d
ExecStart=/home/benjamin-f/Documents/dev/game/dragon_queen_3d/target/release/dragon_queen_server
Restart=always
RestartSec=10

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

## Monitoring

View server logs:
```bash
# Game server logs
sudo journalctl -u dragon-queen-server -f

# Nginx access/error logs
sudo tail -f /var/log/nginx/error.log
```
