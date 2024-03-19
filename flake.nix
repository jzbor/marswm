{
  description = "marswm window manager";
  inputs = {
    nixpkgs.url = "nixpkgs";
    cf.url = "github:jzbor/cornflakes";
    cf.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, cf, crane }:
  cf.lib.flakeForDefaultSystems (system:
  with builtins;
  let
    pkgs = nixpkgs.legacyPackages.${system};
    craneLib = crane.lib.${system};
    nativeBuildInputs = with pkgs; [
      clang
      pkg-config
    ];
    buildInputs = with pkgs; [
      xorg.libX11
      xorg.libXft
      xorg.libXinerama
      xorg.libXrandr
    ];
    devInputs = with pkgs; [
      xorg.xinit
    ];
  in {
    ### PACKAGES ###
    packages = {
      default = craneLib.buildPackage {
        pname = "marswm";

        src = ./.;

        # Add extra inputs here or any other derivation settings
        # doCheck = true;
        inherit nativeBuildInputs;
        inherit buildInputs;
      };

      docs = pkgs.stdenvNoCC.mkDerivation {
        name = "marswm-docs";
        src = ./.;
        buildPhase = "${pkgs.mdbook}/bin/mdbook build .";
        installPhase = "mkdir -p $out; cp -rf target/book/* $out/";
      };

      marswm-scripts = pkgs.symlinkJoin {
        name = "marswm-scripts";
        paths = [
          (pkgs.writeShellApplication {
            name = "mars-help";
            runtimeInputs = with pkgs; [ bat ];
            text = readFile examples/mars-help.sh;
          })
          (pkgs.writeShellApplication {
            name = "mars-status";
            runtimeInputs = with pkgs; [ gnugrep libcanberra-gtk3 libnotify power-profiles-daemon pulseaudio xkb-switch xmenu ];
            text = readFile examples/mars-status.sh;
          })
          (pkgs.writeShellApplication {
            name = "xdg-xmenu";
            runtimeInputs = with pkgs; [ imagemagick ];
            text = "${pkgs.python3}/bin/python3 ${./examples}/xdg-xmenu.py";
          })
        ];
      };
    };

    ### DEVELOPMENT SHELLS ###
    devShells.default = pkgs.mkShellNoCC {
      name = self.packages.${system}.default.name;
      nativeBuildInputs = nativeBuildInputs ++ devInputs;
      inherit buildInputs;
    };
  }) // {

    ### NIXOS MODULE ###
    nixosModules.default = { config, lib, pkgs, ... }: with lib; let
      cfg = config.services.xserver.windowManager.marswm;
    in {
      options.services.xserver.windowManager.marswm = {
        enable = mkEnableOption "marswm";
        package = mkOption {
          type        = types.package;
          default     = pkgs.marswm;
          description = lib.mdDoc ''
          marswm package to use.
          '';
        };
        installScripts = mkEnableOption "install marswm scripts";
      };
      config = mkIf cfg.enable {
        services.xserver.windowManager.session = singleton {
          name = "marswm";
          start = "${cfg.package}/bin/marswm";
        };

        environment.systemPackages = [ cfg.package ] ++ (if cfg.installScripts then [ pkgs.marswm-scripts ] else []);
      };
    };

    ### OVERLAY ###
    overlays.default = final: prev: {
      marswm = self.packages.${prev.system}.default;
      marswm-scripts = self.packages.${prev.system}.marswm-scripts;
    };
  };
}

