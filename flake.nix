{
  description = "Rust development template";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    nixvim = {
      url = "github:nix-community/nixvim";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    dnc.url = "https://linsoft.nl/git/jitze/default-nixvim-config/archive/master.tar.gz";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    dnc,
    nixvim,
    crane,
    ...
  }: let
    buildKeyage = pkgs: let
      craneLib = crane.mkLib pkgs;
      markdownFilter = path: _type: builtins.match ".*md$" path != null;
      txtFilter = path: _type: builtins.match ".*txt$" path != null;
      sumFilter = path: type:
        (markdownFilter path type)
        || (txtFilter path type)
        || (craneLib.filterCargoSources path type);
    in
      craneLib.buildPackage {
        src = nixpkgs.lib.cleanSourceWith {
          src = ./.;
          filter = sumFilter;
          name = "source";
        };

        nativeBuildInputs = with pkgs; [pkg-config];
        buildInputs = with pkgs; [openssl];
      };
  in
    utils.lib.eachDefaultSystem
    (
      system: let
        pkgs = import nixpkgs {inherit system;};
        toolchain = pkgs.rustPlatform;
      in rec
      {
        # Executed by `nix build`
        packages.default = buildKeyage pkgs;

        # Executed by `nix run`
        apps.default = utils.lib.mkApp {drv = packages.default;};

        # Used by `nix develop`
        devShells.default = let
          # Enable rust support for the nvim instance in the devshell
          default_config = nixvim.legacyPackages.${system}.makeNixvim dnc.config;

          # Then apply the configuration
          nvim = default_config.extend {
            plugins = {
              treesitter.settings.ensure_installed = ["rust"];
              lsp.servers.rust-analyzer = {
                enable = true;
                installCargo = false;
                installRustc = false;
              };
              conform-nvim = {
                formattersByFt = {
                  rust = ["rustfmt"];
                };
              };
            };
          };
        in
          pkgs.mkShell {
            buildInputs = [
              (with pkgs; with toolchain; [cargo rustc rustLibSrc])
              (with pkgs; [rust-analyzer clippy rustfmt bacon openssl])
              nvim
            ];
            nativeBuildInputs = with pkgs; [pkg-config];

            shellHook = "exec $SHELL";

            # Specify the rust-src path (many editors rely on this)
            RUST_SRC_PATH = "${toolchain.rustLibSrc}";
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };
      }
    )
    // {
      homeManagerModules.keyage = {
        config,
        lib,
        pkgs,
        ...
      }:
        with lib; let
          cfg = config.programs.keyage;
          tomlFormat = pkgs.formats.toml {};
        in {
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
              default = {};
              example = literalExpression ''
                {
                  identifier = "${config.xdg.configHome}/sops/age/keys.txt"
                }
              '';
              description = ''
                Settings for the configuration file which will be embedded into the store.
              '';
            };
          };

          config = {
            home.packages = [cfg.package];

            home.sessionVariables = {
              KEYAGE_STORE = toString cfg.storePath;
            };

            home.file."${cfg.storePath}/config.toml" = {
              source = tomlFormat.generate "config.toml" cfg.settings;
            };
          };
        };
    };
}
