{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }: let
    inherit (nixpkgs) lib;
    forAllSystems = lib.genAttrs lib.systems.flakeExposed;
  in {
    devShells = forAllSystems (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in {
        default = pkgs.mkShell {
          packages = with pkgs; [
            # Rust toolchain
            rust-bin.stable.latest.default
            cargo-watch
            valkey
            process-compose

            pkg-config
            openssl
          ];
          shellHook = ''
            mkdir -p .valkey-data
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
          '';
        };
      }
    );
  };
}
