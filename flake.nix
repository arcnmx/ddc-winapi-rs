{
  description = "DDC/CI monitor control on Windows";
  inputs = {
    flakelib.url = "github:flakelib/fl";
    nixpkgs = { };
    rust = {
      url = "github:arcnmx/nixexprs-rust";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    arc = {
      url = "github:arcnmx/nixexprs";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, flakelib, nixpkgs, rust, ... }@inputs: let
    nixlib = nixpkgs.lib;
    defaultTarget = "x86_64-pc-windows-gnu";
  in flakelib {
    inherit inputs;
    systems = [ "x86_64-linux" "aarch64-linux" ];
    devShells = {
      plain = {
        mkShell, hostPlatform
      , udev, libiconv
      , pkg-config
      , enableRust ? true, cargo
      , rustTools ? [ ]
      , generate
      }: mkShell {
        inherit rustTools;
        nativeBuildInputs = [
          generate
        ] ++ nixlib.optional enableRust cargo;
        ${if !hostPlatform.isWindows then "CARGO_BUILD_TARGET" else null} = defaultTarget;
      };
      stable = { rust'stable, outputs'devShells'plain }: outputs'devShells'plain.override {
        inherit (rust'stable) mkShell;
        enableRust = false;
      };
      dev = { arc'rustPlatforms'nightly, rust'distChannel, rust-w64-overlay, outputs'devShells'plain }: let
        channel = rust'distChannel {
          inherit (arc'rustPlatforms'nightly) channel date manifestPath;
          channelOverlays = [ rust-w64-overlay ];
        };
      in outputs'devShells'plain.override {
        inherit (channel) mkShell;
        enableRust = false;
        rustTools = [ "rust-analyzer" ];
      };
      default = { outputs'devShells }: outputs'devShells.plain;
    };
    packages = {
      example-enum = { rust-w64, outputs'checks'test, source }: rust-w64.latest.rustPlatform.buildRustPackage {
        pname = self.lib.crate.package.name;
        inherit (self.lib.crate) version;
        inherit (outputs'checks'test) cargoSha256;
        src = source;
        cargoBuildFlags = [ "--example" "enum" ];
        buildType = "debug";
        postInstall = ''
          install -Dt $out/bin target/cargo/*/debug/examples/enum.exe
        '';
        doCheck = false;
        meta.name = "cargo build --example enum";
      };
    };
    legacyPackages = { callPackageSet }: callPackageSet {
      source = { rust'builders }: rust'builders.wrapSource self.lib.crate.src;

      rust-w64 = { pkgsCross'mingwW64 }: import inputs.rust { inherit (pkgsCross'mingwW64) pkgs; };
      rust-w64-overlay = { rust-w64 }: let
        target = rust-w64.lib.rustTargetEnvironment {
          inherit (rust-w64) pkgs;
          rustcFlags = [ "-L native=${rust-w64.pkgs.windows.pthreads}/lib" ];
        };
      in cself: csuper: {
        sysroot-std = csuper.sysroot-std ++ [ cself.manifest.targets.${target.triple}.rust-std ];
        cargo-cc = csuper.cargo-cc // cself.context.rlib.cargoEnv {
          inherit target;
        };
        rustc-cc = csuper.rustc-cc // cself.context.rlib.rustcCcEnv {
          inherit target;
        };
      };

      generate = { rust'builders, outputHashes }: rust'builders.generateFiles {
        paths = {
          "lock.nix" = outputHashes;
        };
      };
      outputHashes = { rust'builders }: rust'builders.cargoOutputHashes {
        inherit (self.lib) crate;
      };
    } { };
    checks = {
      versions = { rust'builders, source }: rust'builders.check-contents {
        src = source;
        patterns = [
          { path = "src/lib.rs"; docs'rs = {
            inherit (self.lib.crate) name version;
          }; }
        ];
      };
      test = { rust-w64, source }: rust-w64.latest.rustPlatform.buildRustPackage {
        pname = self.lib.crate.package.name;
        inherit (self.lib.crate) version;
        cargoSha256 = "sha256-Y++iZL14Mt53VRWnPn+UbMkhSGO0o03YCj2Bdqrm9+c=";
        src = source;
        cargoBuildFlags = [ ];
        cargoTestFlags = [ "--all-targets" ];
        buildType = "debug";
        meta.name = "cargo test";
      };
    };
    lib = with nixlib; {
      crate = rust.lib.importCargo {
        inherit self;
        path = ./Cargo.toml;
        inherit (import ./lock.nix) outputHashes;
      };
      inherit (self.lib.crate.package) version;
      releaseTag = "v${self.lib.version}";
    };
    config = rec {
      name = "ddc-winapi-rs";
      packages.namespace = [ name ];
    };
  };
}
