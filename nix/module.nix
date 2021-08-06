{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.pbd;
  pkg = pkgs.callPackage (import ./package.nix) { inherit (pkgs.darwin.apple_sdk.frameworks) Security; };
  configFile = pkgs.writeText "pbd-config.json" (builtins.toJSON {
    "api_key" = cfg.apiKey;
    "secret_api_key" = cfg.secretApiKey;
    inherit (cfg) domains;
  });
in

{
  options.services.pbd = {
    enable = mkEnableOption "Simple Porkbun dynamic DNS";

    apiKey = mkOption {
      type = types.nullOr types.str;
      description = "Your Porkbun API key.";
      default = null;
    };

    secretApiKey = mkOption {
      type = types.nullOr types.str;
      description = "Your Porkbun secret API key.";
      default = null;
    };

    domains = mkOption {
      type = types.attrsOf (types.listOf types.str);
      description = "Domains and records.";
      default = { };
    };

    interval = mkOption {
      type = types.str;
      description = "How often the DNS records are updated.";
      default = "daily";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.apiKey != null;
        message = "Porkbun API key must be set.";
      }
      {
        assertion = cfg.secretApiKey != null;
        message = "Porkbun secret API key must be set.";
      }
    ];

    systemd.services.pbd = {
      description = "Porkbun DDNS";
      requires = [ "network-online.target" ];
      serviceConfig = {
        Type = "oneshot";
      };
      script = ''
        ${pkg}/bin/pbd ${configFile}
      '';
    };

    systemd.timers.pbd = {
      description = "Porkbun DDNS";
      wantedBy = [ "timers.target" ];
      timerConfig.OnCalendar = cfg.interval;
    };
  };
}
