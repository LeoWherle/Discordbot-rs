#!/bin/bash

# Variables
SSH_USER="ubuntu"
SSH_HOST="xenap.dev"
SSH_PORT=22
LOCAL_FILE="/home/leor/Documents/projects/git/github.com/LeoWherle/Discordbot-rs/target/release/mc-discord-bot"
if [ -z "$LOCAL_FILE" ]; then
  echo "Usage: $0 <local_file_path>"
  exit 1
fi
REMOTE_PATH="/home/ubuntu/documents/discord-bot"

# Upload file using scp
scp -P $SSH_PORT "$LOCAL_FILE" "$SSH_USER@$SSH_HOST:$REMOTE_PATH"