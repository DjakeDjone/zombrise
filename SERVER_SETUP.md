# Dragon Queen Server Setup Guide

## Architecture Overview

- **Game Server**: Runs internally on `127.0.0.1:5000` (UDP)
- **Nginx Stream Proxy**: Listens on `0.0.0.0:443` (UDP) and forwards to `127.0.0.1:5000`
- **Nginx HTTP**: Listens on port 80 for static assets (optional)
- **Client Connection**: `gameserver.fri3dl.dev:443`

## Setup Steps

### 1. Configure Nginx Stream Module

The stream configuration must be included at the **top level** of nginx.conf (NOT inside the http block).

Edit `/etc/nginx/nginx.conf` and add this line near the top (before or after the http block):

```nginx
include /home/benjamin-f/Documents/dev/game/dragon_queen_3d/nginx-stream.conf;
```

Your `/etc/nginx/nginx.conf` should look something like this:

```nginx
user www-data;
worker_processes auto;

# Include stream configuration for UDP proxying
include /home/benjamin-f/Documents/dev/game/dragon_queen_3d/nginx-stream.conf;

http {
    # ... existing http configuration ...
    include /etc/nginx/sites-enabled/*;
}
```

### 2. Set Up HTTP Configuration (Optional)

If you want to serve static assets via HTTP:

```bash
sudo ln -s /home/benjamin-f/Documents/dev/game/dragon_queen_3d/nginx.conf /etc/nginx/sites-enabled/dragon_queen
```

### 3. Test Nginx Configuration

```bash
sudo nginx -t
```

You should see: "configuration file /etc/nginx/nginx.conf test is successful"

### 4. Reload Nginx

```bash
sudo systemctl reload nginx
```

### 5. Configure Firewall

Make sure port 443 UDP is open:

```bash
# For UFW (Ubuntu/Debian)
sudo ufw allow 443/udp

# For firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-port=443/udp
sudo firewall-cmd --reload

# Verify
sudo ufw status  # or: sudo firewall-cmd --list-all
```

### 6. Start the Game Server

```bash
cd /home/benjamin-f/Documents/dev/game/dragon_queen_3d
cargo run --bin dragon_queen_server --release
```

Or set up as a systemd service (recommended for production).

### 7. Test Connection

From the client, connect to: `gameserver.fri3dl.dev:443`

## Troubleshooting

### Check if nginx is listening on port 443 UDP:
```bash
sudo netstat -ulnp | grep :443
# or
sudo ss -ulnp | grep :443
```

### Check if game server is running on port 5000:
```bash
sudo netstat -ulnp | grep :5000
# or
sudo ss -ulnp | grep :5000
```

### Check nginx error logs:
```bash
sudo tail -f /var/log/nginx/error.log
```

### Test UDP connectivity:
```bash
# From another machine
nc -u gameserver.fri3dl.dev 443
```

## Running as a Systemd Service (Recommended)

Create `/etc/systemd/system/dragon-queen-server.service`:

```ini
[Unit]
Description=Dragon Queen Game Server
After=network.target

[Service]
Type=simple
User=benjamin-f
WorkingDirectory=/home/benjamin-f/Documents/dev/game/dragon_queen_3d
ExecStart=/home/benjamin-f/.cargo/bin/cargo run --bin dragon_queen_server --release
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

## Notes

- Port 443 requires sudo/root privileges for nginx
- The game server itself runs as your user on port 5000 (no special privileges needed)
- UDP proxying requires nginx to be compiled with the stream module (included by default in most distributions)
