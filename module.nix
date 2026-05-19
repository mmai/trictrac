{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.trictrac;
in
{

  options = {
    services.trictrac = {
      enable = mkEnableOption "trictrac";

      user = mkOption {
        type = types.str;
        default = "trictrac";
        description = "User under which trictrac is ran.";
      };

      group = mkOption {
        type = types.str;
        default = "trictrac";
        description = "Group under which trictrac is ran.";
      };

      protocol = mkOption {
        type = types.enum [ "http" "https" ];
        default = "https";
        description = "Web server protocol.";
      };

      hostname = mkOption {
        type = types.str;
        default = "trictrac.localhost";
        description = "Public domain name of the trictrac web app.";
      };

      apiPort = mkOption {
        type = types.port;
        default = 8080;
        description = "Port the relay server listens on.";
      };

      smtp = {
        host = mkOption {
          type = types.str;
          default = "127.0.0.1";
          description = "SMTP server hostname.";
        };
        port = mkOption {
          type = types.nullOr types.port;
          default = null;
          description = "SMTP server port. Defaults to 465 when tls = true, 1025 otherwise.";
        };
        tls = mkOption {
          type = types.bool;
          default = false;
          description = "Use TLS (port 465). Required for Resend and other cloud SMTP providers.";
        };
        from = mkOption {
          type = types.str;
          default = "noreply@trictrac.local";
          description = "Sender address for outgoing mail.";
        };
        user = mkOption {
          type = types.str;
          default = "";
          description = "SMTP username (leave empty to skip authentication). Use \"resend\" for Resend.";
        };
        passwordFile = mkOption {
          type = types.nullOr types.path;
          default = null;
          example = "/run/secrets/trictrac-smtp-password";
          description = ''
            Path to a file containing a single line: SMTP_PASSWORD=<secret>.
            Loaded as a systemd EnvironmentFile so the secret never appears in
            the Nix store or process environment of other units.
          '';
        };
      };

      createDatabaseLocally = mkOption {
        type = types.bool;
        default = true;
        example = false;
        description = "Create a local PostgreSQL database for trictrac.";
      };

    };
  };

  config = mkIf cfg.enable {
    users.users.trictrac = mkIf (cfg.user == "trictrac") {
      group = cfg.group;
      isSystemUser = true;
    };
    users.groups.trictrac = mkIf (cfg.group == "trictrac") { };

    services.nginx = {
      enable = true;
      # map needed for WebSocket Connection header upgrade
      appendHttpConfig = ''
        upstream trictrac-api {
          server 127.0.0.1:${toString cfg.apiPort};
        }
        map $http_upgrade $connection_upgrade {
          default upgrade;
          ""      close;
        }
      '';
      virtualHosts =
        let
          proxyConfig = ''
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_redirect off;
            # WebSocket support
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection $connection_upgrade;
            proxy_read_timeout 3600s;
          '';
          withSSL = cfg.protocol == "https";
        in
        {
          "${cfg.hostname}" = {
            enableACME = withSSL;
            forceSSL = withSSL;
            # Explicit listen so this vhost isn't shadowed by a default_server
            # created by other virtual hosts with forceSSL = true.
            listen = if withSSL then [
              { addr = "0.0.0.0"; port = 443; ssl = true; }
              { addr = "[::]";    port = 443; ssl = true; }
            ] else [
              { addr = "0.0.0.0"; port = 80; ssl = false; }
              { addr = "[::]";    port = 80; ssl = false; }
            ];
            locations."/" = {
              extraConfig = proxyConfig;
              proxyPass = "http://trictrac-api/";
            };
          };
        };
    };

    services.postgresql = mkIf cfg.createDatabaseLocally {
      enable = mkDefault true;
      ensureDatabases = [ "trictrac" ];
      ensureUsers = [
        {
          name = cfg.user;
          ensureDBOwnership = true;
        }
      ];
      # Allow the trictrac service user to connect via TCP without a password
      authentication = mkAfter ''
        host trictrac ${cfg.user} 127.0.0.1/32 trust
        host trictrac ${cfg.user} ::1/128 trust
      '';
    };

    systemd.services.trictrac-server =
      let
        setupScript = pkgs.writeShellScript "trictrac-setup" ''
          set -euo pipefail
          # Symlink frontend static files into the state directory so the
          # relay server can serve them from its working directory.
          for f in ${pkgs.trictrac-front}/*; do
            ln -sf "$f" "$STATE_DIRECTORY/$(basename "$f")"
          done
          # Seed a writable GameConfig.json on first run; admins may edit it later.
          if [ ! -f "$STATE_DIRECTORY/GameConfig.json" ]; then
            install -m 644 ${pkgs.trictrac}/GameConfig.json "$STATE_DIRECTORY/GameConfig.json"
          fi
        '';
        smtpEnvScript = if cfg.smtp.passwordFile != null then
          pkgs.writeShellScript "trictrac-smtp-env" ''
            set -euo pipefail
            printf 'SMTP_PASS=%s\n' "$(< ${cfg.smtp.passwordFile})" > /run/trictrac/smtp.env
            chmod 400 /run/trictrac/smtp.env
          ''
        else null;
      in
      {
        description = "trictrac relay server";
        after = [ "network.target" ] ++ optional cfg.createDatabaseLocally "postgresql.service";
        requires = optional cfg.createDatabaseLocally "postgresql.service";
        wantedBy = [ "multi-user.target" ];

        environment = {
          DATABASE_URL = "postgresql://${cfg.user}@127.0.0.1/${cfg.user}";
          APP_URL = "${cfg.protocol}://${cfg.hostname}";
          SMTP_HOST = cfg.smtp.host;
          SMTP_PORT = toString (if cfg.smtp.port != null then cfg.smtp.port
                                else if cfg.smtp.tls then 465 else 1025);
          SMTP_FROM = cfg.smtp.from;
        } // optionalAttrs cfg.smtp.tls {
          SMTP_TLS = "true";
        } // optionalAttrs (cfg.smtp.user != "") {
          SMTP_USER = cfg.smtp.user;
        };

        serviceConfig = {
          User = cfg.user;
          Group = cfg.group;
          # systemd creates /var/lib/trictrac and sets STATE_DIRECTORY accordingly
          StateDirectory = "trictrac";
          StateDirectoryMode = "0755";
          # systemd creates /run/trictrac for the smtp.env file
          RuntimeDirectory = "trictrac";
          WorkingDirectory = "/var/lib/trictrac";
          ExecStartPre = [ "${setupScript}" ] ++ optional (smtpEnvScript != null) "+${smtpEnvScript}";
          ExecStart = "${pkgs.trictrac}/bin/relay-server";
          EnvironmentFile = mkIf (cfg.smtp.passwordFile != null) "/run/trictrac/smtp.env";
          Restart = "on-failure";
          RestartSec = "5s";
        };
      };

  };

  meta = {
    maintainers = with lib.maintainers; [ mmai ];
  };
}
