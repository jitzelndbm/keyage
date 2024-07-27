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
  }:
    utils.lib.eachDefaultSystem
    (
      system: let
        pkgs = import nixpkgs {inherit system;};
        inherit (nixpkgs) lib;
        craneLib = crane.mkLib pkgs;
        toolchain = pkgs.rustPlatform;
        markdownFilter = path: _type: builtins.match ".*md$" path != null;
        txtFilter = path: _type: builtins.match ".*txt$" path != null;
        sumFilter = path: type:
          (markdownFilter path type)
          || (txtFilter path type)
          || (craneLib.filterCargoSources path type);
      in rec
      {
        # Executed by `nix build`
        packages.default = craneLib.buildPackage {
          src = lib.cleanSourceWith {
            src = ./.;
            filter = sumFilter;
            name = "source";
          };

          preBuild = ''
            echo "Contents of src directory:"
            tree
          '';

          nativeBuildInputs = with pkgs; [
            tree
            pkg-config
            openssl
          ];

          buildInputs = with pkgs; [
            pkg-config
            tree
            openssl
          ];
        };
        #toolchain.buildRustPackage {
        #  pname = "keyage";
        #  version = "0.1.0";
        #  src = self;
        #  cargoLock.lockFile = ./Cargo.lock;

        #  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

        #  # For other makeRustPlatform features see:
        #  # https://github.com/NixOS/nixpkgs/blob/master/doc/languages-frameworks/rust.section.md#cargo-features-cargo-features
        #};

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
              (with pkgs;
                with toolchain; [
                  cargo
                  rustc
                  rustLibSrc
                ])

              (with pkgs; [
                rust-analyzer
                clippy
                rustfmt
                bacon

                pkg-config
                openssl
              ])

              nvim
            ];

            nativeBuildInputs = with pkgs; [pkg-config openssl];

            shellHook = "exec $SHELL";

            # Specify the rust-src path (many editors rely on this)
            RUST_SRC_PATH = "${toolchain.rustLibSrc}";
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };
      }
    );
}
