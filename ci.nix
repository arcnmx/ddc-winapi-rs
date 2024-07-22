{ config, channels, pkgs, lib, ... }: with pkgs; with lib; let
  inherit (import ./. { inherit pkgs; }) checks packages;
in {
  config = {
    name = "ddc-winapi";
    ci = {
      version = "v0.7";
      gh-actions.enable = true;
    };
    cache.cachix.arc.enable = true;
    channels = {
      nixpkgs = "24.05";
    };
    tasks = {
      build.inputs = [ checks.test checks.test32 packages.examples ];
      fmt.inputs = singleton checks.rustfmt;
    };
  };
}
