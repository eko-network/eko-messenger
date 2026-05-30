{
  description = "Rust development environment with Postgres";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    devenv.url = "github:cachix/devenv";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        inputs.devenv.flakeModule
      ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem = {pkgs, ...}: let
        port = 3000;
      in {
        devenv.shells.default = {
          packages = with pkgs; [sqlx-cli];
          languages.rust.enable = true;

          services.postgres = {
            enable = true;
            listen_addresses = "127.0.0.1";
            initialDatabases = [
              {
                name = "my_dev_db";
                user = "postgres";
              }
            ];
            initialScript = ''
              ALTER ROLE postgres WITH SUPERUSER;
            '';
          };

          env.DATABASE_URL = "postgres://postgres@127.0.0.1:5432/my_dev_db";
          env.JWT_JWKS_URL = "http://127.0.0.1:54321/auth/v1/.well-known/jwks.json";
          env.SUPABASE_DB_URL = "postgresql://postgres:postgres@127.0.0.1:54322/postgres";
          env.RUST_LOG = "debug";
          env.LISTEN_ADDR = "0.0.0.0";
          env.PORT = port;
          env.DOMAIN = "http://127.0.0.1:${toString port}";
        };
      };
    };
}
