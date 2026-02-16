{self}: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.slyboard;
  yaml = pkgs.formats.yaml {};
  generatedConfig = yaml.generate "slyboard-config.yaml" {};
in {
  options.programs.slyboard = {
    enable = lib.mkEnableOption "slyboard daemon";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.default;
      defaultText = lib.literalExpression "self.packages.\${pkgs.system}.default";
      description = "The slyboard package to run.";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [cfg.package];

    systemd.user.services.slyboard = {
      Unit = {
        Description = "slyboard daemon";
        After = ["graphical-session.target"];
        PartOf = ["graphical-session.target"];
      };

      Service = {
        ExecStart = "${lib.getExe cfg.package} --config ${generatedConfig} run";
        Restart = "on-failure";
        RestartSec = 2;
      };

      Install = {
        WantedBy = ["graphical-session.target"];
      };
    };
  };
}
