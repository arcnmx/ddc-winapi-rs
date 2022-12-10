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
      , nativeBuildInputs ? [ ]
      , generate
      }: mkShell {
        inherit rustTools;
        nativeBuildInputs = nativeBuildInputs ++ [
          generate
        ] ++ nixlib.optional enableRust cargo;
        ${if !hostPlatform.isWindows then "CARGO_BUILD_TARGET" else null} = defaultTarget;
      };
      stable = { rust'stable, outputs'devShells'plain }: outputs'devShells'plain.override {
        inherit (rust'stable) mkShell;
        enableRust = false;
      };
      dev = { arc'rustPlatforms'nightly, rust'distChannel, rust-w64-overlay, outputs'devShells'plain, rust-w64 }: let
        channel = rust'distChannel {
          inherit (arc'rustPlatforms'nightly) channel date manifestPath;
          channelOverlays = [ rust-w64-overlay ];
        };
      in outputs'devShells'plain.override {
        inherit (channel) mkShell;
        enableRust = false;
        rustTools = [ "rust-analyzer" ];
        nativeBuildInputs = [ rust-w64.pkgs.stdenv.cc.bintools ];
      };
      default = { outputs'devShells }: outputs'devShells.plain;
    };
    packages = {
      examples = { rust-w64, outputs'checks'test, source }: rust-w64.latest.rustPlatform.buildRustPackage {
        pname = self.lib.crate.package.name;
        inherit (self.lib.crate) version cargoLock;
        src = source;
        cargoBuildFlags = [ "--examples" ];
        buildType = "debug";
        postInstall = ''
          install -Dt $out/bin $releaseDir/examples/enum${rust-w64.pkgs.hostPlatform.extensions.executable}
        '';
        doCheck = false;
        meta = {
          mainProgram = "enum";
          name = "cargo build --examples";
        };
      };
    };
    legacyPackages = { callPackageSet }: callPackageSet {
      source = { rust'builders }: rust'builders.wrapSource self.lib.crate.src;

      rust-w32 = { pkgsCross'mingw32 }: import inputs.rust { inherit (pkgsCross'mingw32) pkgs; };
      rust-w64 = { pkgsCross'mingwW64 }: import inputs.rust { inherit (pkgsCross'mingwW64) pkgs; };
      rust-w64-overlay = { rust-w64, rust-w32 }: let
        target64 = rust-w64.lib.rustTargetEnvironment {
          inherit (rust-w64) pkgs;
          rustcFlags = [ "-L native=${rust-w64.pkgs.windows.pthreads}/lib" ];
        };
        target32 = rust-w32.lib.rustTargetEnvironment {
          inherit (rust-w32) pkgs;
          rustcFlags = [ "-L native=${rust-w32.pkgs.windows.pthreads}/lib" ];
        };
      in cself: csuper: {
        sysroot-std = csuper.sysroot-std ++ [
          cself.manifest.targets.${target64.triple}.rust-std
          cself.manifest.targets.${target32.triple}.rust-std
        ];
        cargo-cc = csuper.cargo-cc // cself.context.rlib.cargoEnv {
          target = target32;
        } // cself.context.rlib.cargoEnv {
          target = target64;
        };
        rustc-cc = csuper.rustc-cc // cself.context.rlib.rustcCcEnv {
          target = target32;
        } // cself.context.rlib.rustcCcEnv {
          target = target64;
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
      rustfmt = { rust'builders, source }: rust'builders.check-rustfmt-unstable {
        src = source;
        config = ./.rustfmt.toml;
      };
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
        inherit (self.lib.crate) version cargoLock;
        src = source;
        cargoBuildFlags = [ ];
        cargoTestFlags = [ "--all-targets" ];
        buildType = "debug";
        meta.name = "cargo test";
      };
      test32 = { outputs'checks'test, rust-w32 }: outputs'checks'test.override {
        rust-w64 = rust-w32;
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
