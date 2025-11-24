#!/bin/bash
# Script to verify nginx UDP proxy setup for Dragon Queen game server

echo "=== Checking nginx stream module ==="
nginx -V 2>&1 | grep stream

echo -e "\n=== Checking nginx.conf for stream block ==="
sudo grep -A 5 "stream {" /etc/nginx/nginx.conf

echo -e "\n=== Checking if nginx is listening on port 443 UDP ==="
sudo ss -ulnp | grep :443

echo -e "\n=== Checking if game server is listening on port 5000 UDP ==="
sudo ss -ulnp | grep :5000

echo -e "\n=== Testing nginx configuration ==="
sudo nginx -t

echo -e "\n=== Nginx status ==="
sudo systemctl status nginx --no-pager
