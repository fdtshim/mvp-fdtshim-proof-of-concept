#!/usr/bin/env bash

set -e
set -u
PS4=" $ "

(
set -x
cargo build --target "$RUST_TARGET"
dtc -q -I dts mapping.dts -o mapping.dtb
)

case "$RUST_TARGET" in
	x86_64*) qemu=qemu-system-x86_64 ;;
	aarch64*) qemu=qemu-system-aarch64 ;;
esac

BIOS="${BIOS:-"${OVMF}/OVMF.fd"}"

ARGS=(
	--bios-path="${BIOS}"
	--boot "target/${RUST_TARGET}/debug/fdtshim.efi"
	--add-file "target/${RUST_TARGET}/debug/fdtshim.efi:EFI/Boot/BootAA64.efi"
	--qemu-path "$qemu"
)
QEMU_ARGS=(
	#-serial stdio
	-m 512M
	-nographic # CTRL+A then X to quit
	"$@"
)

if [[ $RUST_TARGET == "aarch64-unknown-uefi" ]]; then
	# Going "simpler" takes less time to boot than `max`.
	QEMU_ARGS+=(-cpu cortex-a53)
	QEMU_ARGS+=(-m 2G)
	QEMU_ARGS+=(-machine virt)
	QEMU_ARGS+=(-netdev user,id=net_id)
	QEMU_ARGS+=(-device driver=virtio-net,netdev=net_id)
	#QEMU_ARGS+=(-display gtk,gl=on)
	#QEMU_ARGS+=(-device virtio-gpu-pci)
	# Dumping the dtb
	qemu-system-aarch64 "${QEMU_ARGS[@]}" -machine dumpdtb=test.dtb
	fdtput test.dtb --type s / model "SUCCESSFUL TEST virtual system"
	# This pair of node **from the dtb dumped by qemu** will crash the kernel.
	fdtput test.dtb --remove /gpio-keys /pl061@9030000
	#QEMU_ARGS+=(-dtb test.dtb)
	#ARGS+=(--add-file "test.dtb:EFI/Boot/virt.dtb")
	ARGS+=(--add-file "test.dtb:dtbs/virt/linux-dummy-virt.dtb")
	ARGS+=(--add-file "mapping.dtb:dtbs/mapping.dtb")
fi

ARGS+=(
	--
	"${QEMU_ARGS[@]}"
)

set -x

exec uefi-run "${ARGS[@]}"
