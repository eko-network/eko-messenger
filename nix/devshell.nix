{ pkgs }:

pkgs.mkShell {
  packages = with pkgs; [
    rust-bin.stable.latest.default
    cargo-watch
    process-compose
    postgresql
    pkg-config
    openssl
    sqlx-cli
  ];
  
  shellHook = ''
    export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
  '';
}