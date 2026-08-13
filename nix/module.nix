# NixOS module for Murmel speech-to-text
#
# Handles system-level configuration that the package wrapper cannot:
#   - udev rule for /dev/uinput (rdev grab() needs it for virtual input)
#
# Note: users must add themselves to the "input" group for evdev hotkey access.
#
# Usage in your flake:
#
#   inputs.murmel.url = "github:Suppenterrine/murmel";
#
#   nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
#     modules = [
#       murmel.nixosModules.default
#       { programs.murmel.enable = true; }
#     ];
#   };
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.murmel;
in
{
  options.programs.murmel = {
    enable = lib.mkEnableOption "Murmel offline speech-to-text";

    package = lib.mkOption {
      type = lib.types.package;
      defaultText = lib.literalExpression "murmel.packages.\${system}.murmel";
      description = "The Murmel package to use.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    # rdev grab() creates virtual input devices via /dev/uinput.
    # Default permissions are crw------- root root — open it to the input group.
    services.udev.extraRules = ''
      KERNEL=="uinput", GROUP="input", MODE="0660"
    '';
  };
}
