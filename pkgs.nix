{ pkgs }:

#/*
let pkgs' = pkgs; in # XXX
let pkgs = pkgs'.pkgsCross.aarch64-multiplatform; in # XXX
/* */

let
  inherit (pkgs)
    lib
    stdenv
  ;
  pins = import ./npins;
  fenix = import pins.fenix {
    pkgs = pkgs.buildPackages;
    inherit (stdenv) system;
  };
  rustTarget = {
    "x86_64-linux" = "x86_64-unknown-uefi";
    "aarch64-linux" = "aarch64-unknown-uefi";
  }.${stdenv.system};
  rustToolchain = with fenix;
    combine (
      [
        stable.rustc
        stable.cargo
        stable.clippy
        stable.rustfmt
        stable.rust-src
        targets.${rustTarget}.stable.rust-std
      ]
    )
  ;

  overlays = pkgs.appendOverlays [(final: super: {
      # XXX doesn't cross-compile on Nixpkgs :(
      OVMF = {
        fd = final.callPackage (
          { runCommand
          , stdenv
          , fetchurl
          }:
          runCommand "OVMF-prebuilt" {
            src = fetchurl {
              url = "https://github.com/rust-osdev/ovmf-prebuilt/releases/download/edk2-stable202402-r1/edk2-stable202402-r1-bin.tar.xz";
              hash = "sha256-kfMUjvFGeUJBx3gQpJz6PpJcg+tVxcyQ80cYzBsQ6es=";
            };
            arch =
              if stdenv.hostPlatform.efiArch == "aa64"
              then "aarch64"
              else stdenv.hostPlatform.efiArch
            ;
          } ''
            (
            PS4=" $ " ; set -x
            tar xf $src
            cd */
            mkdir -p $out
            mv -t "$out" $arch/*
            cd $out
            if [[ "$arch" == "x64" ]]; then
              cat vars.fd code.fd > OVMF.fd
            else
              cp code.fd OVMF.fd
            fi
            )
          ''
        ) {};
      };
  })];
in
{
  inherit (overlays) OVMF;
  inherit pkgs;
  inherit stdenv;
  inherit (overlays) hello;
  inherit fenix;
  shell = overlays.callPackage (
    { mkShell
    , rustToolchain
    , rustTarget
    , OVMF
    , uefi-run
    , qemu
    , dtc
    }:
    mkShell {
      RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/";
      OVMF = OVMF.fd;
      RUST_TARGET = rustTarget;

      depsBuildBuild = [
        rustToolchain
        uefi-run
        qemu
        dtc
      ];
    }
  ) {
    inherit rustToolchain;
    inherit rustTarget;
  };
}
