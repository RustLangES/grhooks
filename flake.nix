{
  description = "Powered Hardware test tool written in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        toolchain = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);

      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ cargo-typify toolchain ];
        };
      }
    );
}
