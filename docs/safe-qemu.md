# Safe QEMU Testing

CloudOS should only be tested inside QEMU on this machine.

Safest command for a borrowed laptop because it does not grab the mouse:

```sh
./scripts/run-qemu-terminal.sh
```

GUI command:

```sh
qemu-system-x86_64 -boot d -cdrom cloudos.iso -usb -device usb-tablet -no-reboot -no-shutdown
```

Safety rules:

- Do not pass `/dev/disk*` to QEMU.
- Do not use `-drive file=/dev/...`.
- Do not format or install anything to the host disk.
- Use `Control + Option + G` on macOS if QEMU grabs the mouse or keyboard.
- Close the QEMU window to stop the virtual machine.
- Prefer `scripts/run-qemu-terminal.sh` when testing keyboard input because it uses the terminal instead of a Cocoa window.

The `-cdrom cloudos.iso` option makes QEMU read only the ISO file. It does not boot from, write to, or repartition the laptop disk.
