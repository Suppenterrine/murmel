# Home-manager module for Murmel speech-to-text
#
# Provides a systemd user service for autostart.
# Usage: imports = [ murmel.homeManagerModules.default ];
#        services.murmel.enable = true;
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.murmel;
in
{
  options.services.murmel = {
    enable = lib.mkEnableOption "Murmel speech-to-text user service";

    package = lib.mkOption {
      type = lib.types.package;
      defaultText = lib.literalExpression "murmel.packages.\${system}.murmel";
      description = "The Murmel package to use.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.murmel = {
      Unit = {
        Description = "Murmel speech-to-text";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/murmel";
        Restart = "on-failure";
        RestartSec = 5;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
