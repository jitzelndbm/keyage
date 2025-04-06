{
  description = "<DESCRIPTION>";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      treefmt-nix,
      pre-commit-hooks,
      ...
    }:
    let
      fa = nixpkgs.lib.genAttrs [ "x86_64-linux" ];
      treefmtEval = fa (
        system: treefmt-nix.lib.evalModule nixpkgs.legacyPackages.${system} ./nix/formatters.nix
      );
    in
    {
      homeManagerModules.keyage =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        with lib;
        let
          cfg = config.programs.keyage;
          tomlFormat = pkgs.formats.toml { };
        in
        {
          options.programs.keyage = {
            enable = mkEnableOption "keyage";
            package = mkOption {
              type = types.package;
              default = buildKeyage pkgs;
            };
            storePath = mkOption {
              type = types.path;
              default = "${config.xdg.dataHome}/keyage-store";
              example = literalExpression ''"${config.home.homeDirectory}/.keyage-store"'';
              description = "This is where the password store will be initialized.";
            };
            settings = mkOption {
              type = tomlFormat.type;
              default = { };
              example = literalExpression ''
                {
                  identifier = "${config.xdg.configHome}/sops/age/keys.txt"
                  recipient = "age1somethingsomethingyougetthepoint"
                }
              '';
              description = ''
                Settings for the configuration file which will be embedded into the store.
              '';
            };
          };

          config = {
            home.packages = [ cfg.package ];

            home.sessionVariables = {
              KEYAGE_STORE = toString cfg.storePath;
            };

            home.file."${cfg.storePath}/config.toml" = {
              source = tomlFormat.generate "config.toml" cfg.settings;
            };
          };
        };

      packages = fa (system: {
        keyage = self.packages.${system}.default;
        default = (import nixpkgs { inherit system; }).callPackage ./nix/keyage.nix { };
      });

      devShells = fa (system: {
        default =
          let
            pkgs = import nixpkgs { inherit system; };
            inherit (pkgs) mkShell nil;
            inherit (pkgs.lib) concatLines;
            inherit (self.checks.${pkgs.system}.pre-commit-check) shellHook enabledPackages;

            treefmt = treefmtEval.${pkgs.system}.config.build.wrapper;
            shell = import ./nix/shell.nix { inherit pkgs; };
          in
          mkShell (
            shell
            // {
              packages = (shell.packages or [ ]) ++ [
                treefmt
                nil
                enabledPackages
              ];

              shellHook = concatLines [
                (shell.shellHook or "")
                shellHook
              ];
            }
          );
      });

      formatter = fa (system: treefmtEval.${system}.config.build.wrapper);

      checks = fa (system: {
        pre-commit-check = pre-commit-hooks.lib.${system}.run {
          src = ./.;
          hooks = (import ./nix/pre-commit-hooks.nix) // {
            treefmt = {
              enable = true;
              package = treefmtEval.${system}.config.build.wrapper;
            };
          };
        };
      });
    };
}
