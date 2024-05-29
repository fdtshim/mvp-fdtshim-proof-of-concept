#!/usr/bin/env bash

case "$RUST_TARGET" in
	x86_64*) qemu=qemu-system-x86_64 ;;
	aarch64*) qemu=qemu-system-aarch64 ;;
esac

ARGS=(
	--bios-path="${OVMF}/OVMF.fd"
	--boot "target/${RUST_TARGET}/debug/uefi-hello-world.efi"
	--add-file "target/${RUST_TARGET}/debug/uefi-hello-world.efi:EFI/Boot/BootAA64.efi"
	--qemu-path "$qemu"
	--
	#-serial stdio
	-m 512M
	-nographic # CTRL+A then X to quit
)
if [[ $RUST_TARGET == "aarch64-unknown-uefi" ]]; then
	ARGS+=(-cpu cortex-a57)
	ARGS+=(-M virt)
fi

exec uefi-run "${ARGS[@]}"
