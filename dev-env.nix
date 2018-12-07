let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  rustChannel = pkgs.rustChannelOf { channel = "stable"; };
in
  with pkgs;
  stdenv.mkDerivation {
    name = "rust-dev-env";
    buildInputs = [
      rustChannel.rust
      cmake
      pkgconfig
      libudev
    ];
}
