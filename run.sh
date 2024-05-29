#!/usr/bin/env bash

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
	qemu-system-aarch64 -machine virt -cpu cortex-a57 -machine dumpdtb=test.dtb
	ARGS+=(--add-file "test.dtb:EFI/Boot/virt.dtb")
	QEMU_ARGS+=(-cpu cortex-a57)
	QEMU_ARGS+=(-machine virt)
	QEMU_ARGS+=(-dtb test.dtb)
fi

ARGS+=(
	--
	"${QEMU_ARGS[@]}"
)

PS4=" $ "
set -x

exec uefi-run "${ARGS[@]}"
