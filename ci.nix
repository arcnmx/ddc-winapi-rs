{ config, channels, pkgs, lib, ... }: with pkgs; with lib; let
  inherit (import ./. { inherit pkgs; }) checks packages;
in {
  config = {
    name = "ddc-winapi";
    ci.gh-actions.enable = true;
    cache.cachix.arc.enable = true;
    channels = {
      nixpkgs = "22.11";
    };
    tasks = {
      build.inputs = [ checks.test checks.test32 packages.examples ];
      fmt.inputs = singleton checks.rustfmt;
    };
  };
}
