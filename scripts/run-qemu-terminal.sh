#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

if [ ! -f cloudos.iso ]; then
    echo "cloudos.iso not found in the project folder."
    echo "Download the cloudos-iso artifact from GitHub Actions first."
    exit 1
fi

exec qemu-system-x86_64 \
    -boot d \
    -cdrom cloudos.iso \
    -display curses \
    -monitor none \
    -no-reboot \
    -no-shutdown
