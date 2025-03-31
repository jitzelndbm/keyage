{ pkgs, ... }:
{
  projectRootFile = "flake.nix";
  programs.nixfmt.enable = true;
  programs.mdformat.enable = true;
  programs.stylua.enable = true;
  programs.rustfmt.enable = true;
}
