{ config, lib, pkgs, ... }: 
let
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
      default = pkgs.callPackage ./package.nix { };
      description = "The eko-messenger package to use";
    };
    
    port = lib.mkOption {
      type = lib.types.port;
      default = 3000;
      description = "Port to listen on";
    };
    
    domain = lib.mkOption {
      type = lib.types.str;
      example = "messenger.example.com";
      description = "Domain name for the service";
    };
    
    database = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Whether to automatically configure PostgreSQL database.
          If disabled, you must manually set databaseUrl.
        '';
      };
      
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
        description = "Whether to create the database and user locally";
      };
    };
    
    databaseUrl = lib.mkOption {
      type = lib.types.str;
      default = defaultDbUrl;
      description = "PostgreSQL database URL";
    };
    
    ipSource = lib.mkOption {
      type = lib.types.str;
      default = "ConnectInfo";
      description = "IP source configuration";
    };
    

    firebaseApiKey = lib.mkOption {
      type = lib.types.str;
      default = null;
      description = "";
    };
    jwtSecret = lib.mkOption {
      type = lib.types.str;
      default = null;
      description = "";
    };
    
    logLevel = lib.mkOption {
      type = lib.types.str;
      default = "info";
      description = "Rust log level)";
    };
  };
  
  config = lib.mkIf cfg.enable {
    # Automatic PostgreSQL setup
    services.postgresql = lib.mkIf cfg.database.createLocally {
      enable = true;
      ensureDatabases = [ cfg.database.name ];
      ensureUsers = [{
        name = cfg.database.user;
        ensureDBOwnership = true;
      }];
    };
    
    systemd.services.eko-messenger = {
      description = "Eko Messenger Service";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ] 
        ++ lib.optional cfg.database.createLocally "postgresql.service";
      requires = lib.optional cfg.database.createLocally "postgresql.service";
      
      serviceConfig = {
        Type = "simple";
        DynamicUser = true;
        User = lib.mkIf cfg.database.createLocally cfg.database.user;
        Restart = "on-failure";
        RestartSec = "5s";
        
        ExecStart = "${cfg.package}/bin/eko-messenger";
        
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ ];
      };
      
      environment = {
        DATABASE_URL = cfg.databaseUrl;
        DOMAIN = cfg.domain;
        IP_SOURCE = cfg.ipSource;
        RUST_LOG = cfg.logLevel;
        FIREBASE_API_KEY = cfg.firebaseApiKey;
        JWT_SECRET = cfg.jwtSecret;
      };
    } ;
  };
}