#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

if [ ! -f cloudos.iso ]; then
    echo "cloudos.iso not found in the project folder."
    echo "Download the cloudos-iso artifact from GitHub Actions first."
    exit 1
fi

echo "Starting CloudOS headless QEMU."
echo "Serial log: qemu-serial.log"
echo "Stop with Ctrl-C."

exec qemu-system-x86_64 \
    -boot d \
    -cdrom cloudos.iso \
    -display none \
    -serial file:qemu-serial.log \
    -monitor none \
    -no-reboot \
    -no-shutdown
