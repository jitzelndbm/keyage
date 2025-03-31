{ pkgs, ... }:
{
  packages =
    let
      inherit (pkgs)
        rustPlatform
        rust-analyzer
        clippy
        rustfmt
        bacon
        cargo
        rustc
        pkg-config
        openssl
        ;
      inherit (rustPlatform) rustLibSrc;
    in
    [
      rustLibSrc
      rust-analyzer
      clippy
      rustfmt
      bacon
      cargo
      rustc
      pkg-config
      openssl
    ];
}
