{pkgs}:
pkgs.rustPlatform.buildRustPackage {
  pname = "eko-messenger";
  version = "0.1.0";

  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    openssl
  ];

  SQLX_OFFLINE = "true";
  doCheck = false;
}

