# Remote Server Deployment Checklist

## Prerequisites
- [ ] SSH access to gameserver.fri3dl.dev
- [ ] Root/sudo access on the remote server
- [ ] Rust toolchain installed on remote server

## Step 1: Deploy the Game Server Binary

### On your local machine:
```bash
# Build the release binary
cargo build --release --bin dragon_queen_server

# Copy to remote server
scp target/release/dragon_queen_server user@gameserver.fri3dl.dev:~/
```

### On the remote server (via SSH):
```bash
# SSH into the server
ssh user@gameserver.fri3dl.dev

# Make the binary executable
chmod +x ~/dragon_queen_server

# Test run (should see "Server started on 0.0.0.0:5000")
./dragon_queen_server
```

## Step 2: Configure Nginx on Remote Server

### On the remote server:

```bash
# 1. Check if nginx has stream module
nginx -V 2>&1 | grep stream

# 2. Create the stream configuration
sudo nano /etc/nginx/streams-available/dragon_queen.conf
```

Add this content:
```nginx
# UDP proxy for Dragon Queen game server
server {
    listen 443 udp;
    proxy_pass 127.0.0.1:5000;
    
    # UDP-specific settings
    # UDP-specific settings
    proxy_timeout 60s;
    # proxy_responses 1; # REMOVED: This closes connection after 1 packet!
    
    # Buffer size for UDP packets
    proxy_buffer_size 16k;
}
```

```bash
# 3. Enable the stream configuration
sudo mkdir -p /etc/nginx/streams-enabled
sudo ln -s /etc/nginx/streams-available/dragon_queen.conf /etc/nginx/streams-enabled/

# 4. Edit main nginx config
sudo nano /etc/nginx/nginx.conf
```

Add this **outside** the `http` block (at the top level):
```nginx
stream {
    include /etc/nginx/streams-enabled/*.conf;
}
```

```bash
# 5. Test nginx configuration
sudo nginx -t

# 6. Reload nginx
sudo systemctl reload nginx
```

## Step 3: Configure Firewall

```bash
# For UFW (Ubuntu/Debian)
sudo ufw allow 443/udp
sudo ufw status

# For firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-port=443/udp
sudo firewall-cmd --reload
sudo firewall-cmd --list-all
```

## Step 4: Test Connection

### Test UDP port 443 is open:
```bash
# From another machine
nc -u -v gameserver.fri3dl.dev 443
```

### Check if nginx is listening on 443:
```bash
# On the remote server
sudo netstat -tulpn | grep :443
# or
sudo ss -tulpn | grep :443
```

## Step 5: Run Game Server as a Service

Create systemd service:
```bash
sudo nano /etc/systemd/system/dragon-queen-server.service
```

Add:
```ini
[Unit]
Description=Dragon Queen 3D Game Server
After=network.target

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/home/YOUR_USERNAME
ExecStart=/home/YOUR_USERNAME/dragon_queen_server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable dragon-queen-server
sudo systemctl start dragon-queen-server
sudo systemctl status dragon-queen-server

# View logs
sudo journalctl -u dragon-queen-server -f
```

## Step 6: Verify Everything Works

### On remote server:
```bash
# Check game server is running
ps aux | grep dragon_queen_server

# Check nginx is running
sudo systemctl status nginx

# Check port 443 is listening
sudo netstat -tulpn | grep :443

# Monitor logs
sudo journalctl -u dragon-queen-server -f
sudo tail -f /var/log/nginx/error.log
```

### From your client machine:
```bash
cargo run --bin dragon_queen_client
# Should connect to gameserver.fri3dl.dev:443
```

## Troubleshooting

### Connection refused:
- Check if game server is running: `ps aux | grep dragon_queen`
- Check if nginx is forwarding: `sudo journalctl -u nginx -n 50`

### Timeout:
- Check firewall: `sudo ufw status` or `sudo firewall-cmd --list-all`
- Check if port 443/UDP is forwarded from your router (if behind NAT)
- Verify nginx stream config: `sudo nginx -t`

### Nginx won't start:
- Check if stream module is compiled in: `nginx -V 2>&1 | grep stream`
- If not, reinstall nginx with stream: `sudo apt install nginx-full`

### Game server crashes:
- Check logs: `sudo journalctl -u dragon-queen-server -n 100`
- Run manually to see errors: `./dragon_queen_server`
