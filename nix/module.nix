{self}: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.eko-messenger;

  # Database configuration
  defaultDbName = "eko_messenger";
  defaultDbUser = "eko_messenger";
  defaultDbUrl = "postgresql:///${defaultDbName}?host=/run/postgresql";
in {
  options.services.eko-messenger = {
    enable = lib.mkEnableOption "eko-messenger service";

    package = lib.mkOption {
      type = lib.types.package;
      default = let
        system = pkgs.stdenv.hostPlatform.system;
      in
        self.packages.${system}.default or (pkgs.callPackage ./package.nix {});
      description = "The eko package to use";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 3000;
      description = "Port to listen on";
    };

    domain = lib.mkOption {
      type = lib.types.str;
      example = "http://127.0.0.1:3000";
      description = "Domain name for the service";
    };

    listenAddr = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "The address the server will bind to";
    };

    database = {
      name = lib.mkOption {
        type = lib.types.str;
        default = defaultDbName;
        description = "Database name";
      };

      user = lib.mkOption {
        type = lib.types.str;
        default = defaultDbUser;
        description = "Database user";
      };

      createLocally = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Whether to create the database and user locally.
          If disabled, you must set databaseUrl.
        '';
      };
    };

    databaseUrl = lib.mkOption {
      type = lib.types.str;
      default = defaultDbUrl;
      description = "PostgreSQL URL for the messenger database (migrated on startup)";
    };

    jwtJwksUrl = lib.mkOption {
      type = lib.types.str;
      example = "https://example.supabase.co/auth/v1/.well-known/jwks.json";
      description = "JWKS endpoint used to verify client JWTs";
    };

    environmentFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      example = "/run/secrets/eko-messenger.env";
      description = ''
        File with secret environment variables, kept out of the Nix store.
        Must set SUPABASE_DB_URL, and may set VAPID_PRIVATE_KEY,
        VAPID_PUBLIC_KEY and the APNS_* variables.
      '';
    };

    apns = {
      teamId = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Apple developer team ID for push notifications";
      };

      keyId = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "APNs auth key ID (the key itself goes in environmentFile as APNS_PRIVATE_KEY)";
      };

      bundleId = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "App bundle ID used as the APNs topic";
      };

      sandbox = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Whether to use the APNs sandbox environment";
      };
    };

    logLevel = lib.mkOption {
      type = lib.types.str;
      default = "info";
      description = "Rust log level";
    };
  };

  config = lib.mkIf cfg.enable {
    # Automatic PostgreSQL setup
    services.postgresql = lib.mkIf cfg.database.createLocally {
      enable = true;
      ensureDatabases = [cfg.database.name];
      ensureUsers = [
        {
          name = cfg.database.user;
          ensureDBOwnership = true;
        }
      ];
    };

    systemd.services.eko-messenger = {
      description = "Eko Messenger Service";
      wantedBy = ["multi-user.target"];
      after =
        ["network-online.target"]
        ++ lib.optional cfg.database.createLocally "postgresql.service";
      wants = ["network-online.target"];
      requires = lib.optional cfg.database.createLocally "postgresql.service";

      serviceConfig = {
        Type = "simple";
        DynamicUser = true;
        # peer auth over the postgres socket needs the unix user to match the db user
        User = lib.mkIf cfg.database.createLocally cfg.database.user;
        Restart = "on-failure";
        RestartSec = "5s";
        StartLimitBurst = 3;
        ExecStart = lib.getExe cfg.package;
        EnvironmentFile = lib.mkIf (cfg.environmentFile != null) cfg.environmentFile;
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        StateDirectory = "eko-messenger";
        ReadWritePaths = [];
      };

      environment =
        {
          DATABASE_URL = cfg.databaseUrl;
          PORT = toString cfg.port;
          DOMAIN = cfg.domain;
          LISTEN_ADDR = cfg.listenAddr;
          JWT_JWKS_URL = cfg.jwtJwksUrl;
          RUST_LOG = cfg.logLevel;
          APNS_USE_SANDBOX = lib.boolToString cfg.apns.sandbox;
        }
        // lib.optionalAttrs (cfg.apns.teamId != null) {APNS_TEAM_ID = cfg.apns.teamId;}
        // lib.optionalAttrs (cfg.apns.keyId != null) {APNS_KEY_ID = cfg.apns.keyId;}
        // lib.optionalAttrs (cfg.apns.bundleId != null) {APNS_BUNDLE_ID = cfg.apns.bundleId;};
    };
  };
}
