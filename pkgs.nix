{ pkgs }:

let
  pins = import ./npins;
  fenix = import pins.fenix {
    inherit pkgs;
    inherit (pkgs.stdenv) system;
  };
  overlays = pkgs.appendOverlays [(final: super: {
  })];
  rustToolchain = with fenix;
    combine [
      stable.rustc
      stable.cargo
      stable.clippy
      stable.rustfmt
      # FIXME: support more than x86_64
      targets.x86_64-unknown-uefi.stable.rust-std
      stable.rust-src
    ]
  ;
in
{
  inherit (overlays) hello;
  inherit fenix;
  shell = pkgs.mkShell {
    RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/";
    OVMF = pkgs.OVMF.fd;

    buildInputs = with pkgs; [
      rustToolchain
      uefi-run
      qemu
    ];
  };
}
