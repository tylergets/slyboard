{
  description = "slyboard daemon";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
  }:
    (flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
        };
        craneLib = crane.mkLib pkgs;
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (pkgs.lib.hasSuffix ".svg" (toString path));
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          nativeBuildInputs = with pkgs; [
            makeWrapper
            pkg-config
          ];
          buildInputs = with pkgs; [
            gtk3
            libappindicator-gtk3
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        slyboard = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "slyboard";
            version = "0.2.5";
            meta = {
              mainProgram = "slyboard";
            };
            postFixup = ''
              wrapProgram "$out/bin/slyboard" \
                --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [
                  pkgs.libappindicator-gtk3
                  pkgs.gtk3
                ]}"
            '';
          }
        );
      in {
        packages.default = slyboard;

        apps.default = flake-utils.lib.mkApp {
          drv = slyboard;
        };

        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            rust-analyzer
            clippy
            rustfmt
            cargo-watch
            cargo-nextest
            cargo-edit
            cargo-audit
            cargo-deny
            cargo-expand
            libappindicator-gtk3
            gtk3
            pkg-config
          ];
        };
      }
    ))
    // {
      homeManagerModules.default = import ./nix/home-manager.nix {
        inherit self;
      };
    };
}
