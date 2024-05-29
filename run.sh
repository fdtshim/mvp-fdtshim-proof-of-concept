#!/usr/bin/env bash

ARGS=(
	--bios-path="${OVMF}/FV/OVMF.fd"
	--boot target/x86_64-unknown-uefi/debug/uefi-hello-world.efi
	--
	-serial stdio
)

exec uefi-run "${ARGS[@]}"
