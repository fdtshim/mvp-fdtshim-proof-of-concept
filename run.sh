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
	--boot "target/${RUST_TARGET}/debug/uefi-hello-world.efi"
	--add-file "target/${RUST_TARGET}/debug/uefi-hello-world.efi:EFI/Boot/BootAA64.efi"
	--qemu-path "$qemu"
)
QEMU_ARGS=(
	#-serial stdio
	-m 512M
	-nographic # CTRL+A then X to quit
)

if [[ $RUST_TARGET == "aarch64-unknown-uefi" ]]; then
	QEMU_ARGS+=(-cpu cortex-a57)
	QEMU_ARGS+=(-machine virt)
	# Dumping the dtb
	qemu-system-aarch64 "${QEMU_ARGS[@]}" -machine dumpdtb=test.dtb
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
