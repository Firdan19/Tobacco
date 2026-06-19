#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

if [ ! -f tobacco.iso ]; then
    echo "tobacco.iso not found in the project folder."
    echo "Download the tobacco-iso artifact from GitHub Actions first."
    exit 1
fi

exec qemu-system-x86_64 \
    -boot d \
    -cdrom tobacco.iso \
    -usb \
    -device usb-tablet \
    -no-reboot \
    -no-shutdown
