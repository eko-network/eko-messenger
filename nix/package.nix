{
  pkgs,
  authFeature ? null,
}:
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

  buildNoDefaultFeatures = true;

  buildFeatures =
    if authFeature != null
    then [authFeature]
    else ["auth-oidc"];

  SQLX_OFFLINE = "true";
  doCheck = false;
}
