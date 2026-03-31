{
  description = "dev-flake for micromusic";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {inherit system overlays;};
      rust-with-exts = (pkgs.rust-bin.stable.latest.default.override {
        extensions = ["rust-src" "cargo" "rustc" ];
      });
    in {
      devShells.default = with pkgs;
        mkShell rec {
          buildInputs = [
            # rust support
            clang
            glib
            rust-with-exts
            rust-analyzer
            rustfmt
            clippy

            # libs
            SDL2
            SDL2_image
            SDL2_ttf
            xz
          ];
          RUST_SRC_PATH = "${rust-with-exts}/lib/rustlib/src/rust/library";
          nativeBuildInputs = [
            pkg-config
          ];
          LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath (buildInputs ++ nativeBuildInputs);
        };
    });
}
