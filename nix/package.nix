{pkgs}: let
  fs = pkgs.lib.fileset;
in
  pkgs.rustPlatform.buildRustPackage {
    pname = "eko";
    version = "0.1.0";

    # only the workspace sources, so local `target/` and docs don't end up in the store
    src = fs.toSource {
      root = ../.;
      fileset = fs.unions [
        ../Cargo.toml
        ../Cargo.lock
        ../.sqlx
        ../eko
        ../eko-messenger
      ];
    };

    cargoLock = {
      lockFile = ../Cargo.lock;
    };

    cargoBuildFlags = ["-p" "eko"];

    SQLX_OFFLINE = "true";
    doCheck = false;

    meta.mainProgram = "eko";
  }
