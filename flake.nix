{
  description = "cyberkrill";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
          targets = [ "x86_64-unknown-linux-musl" ];
        };
      in
      {
        # Default package: static musl build with smartcards
        packages.default = let
          rustPlatformMusl = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
        in rustPlatformMusl.buildRustPackage {
          pname = "cyberkrill";
          version = "0.1.0";
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "cktap-direct-0.1.0" = "sha256-yX9sFHWsisEXYBjYo2TrpC/vj58JO/b+Bb+phubkJNA=";
              "bip39-2.2.0" = "sha256-HVA5vJ24gU3ZFswk5MAMZ1mfdQOGOoBywwUlG9u72dU=";
              "frozenkrill-core-0.0.0" = "sha256-xCE26AxN4HgB3Yrfv3jgl0WYIl+xSmCKtWgJMjAql8w=";
              "coldcard-0.12.2" = "sha256-S+MARrWsdGCsfe4A3cUqaKSijo81MfH6KLIeuBpMckc=";
              "hidapi-compat-0.1.0" = "sha256-SYtLivsas+kkmhdsZSWQXF+AkCe4bMDryzJBfsZQhXE=";
              "trezor-client-0.1.5" = "sha256-MYBGRBGG818U65r0LmDOZCW09rCwa2R52Sh3WomKXek=";
            };
          };
          
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
            pkgsStatic.stdenv.cc
          ];
          
          buildInputs = with pkgs.pkgsStatic; [
            libusb1
          ];
          
          # Environment variables for static libusb
          LIBUSB_STATIC = "1";
          PKG_CONFIG_PATH = "${pkgs.pkgsStatic.libusb1}/lib/pkgconfig";
          
          # Don't specify features - use the defaults from Cargo.toml
          # which includes all hardware wallets
          
          # Force cargo to use the musl target from .cargo/config.toml
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          CC_x86_64_unknown_linux_musl = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static -C link-arg=-static";
          
          # Override buildPhase to use the correct target
          buildPhase = ''
            runHook preBuild
            
            echo "Building with musl target and default features (all hardware wallets)..."
            cargo build \
              --release \
              --target x86_64-unknown-linux-musl \
              --offline \
              -j $NIX_BUILD_CORES
            
            runHook postBuild
          '';
          
          installPhase = ''
            runHook preInstall
            
            mkdir -p $out/bin
            cp target/x86_64-unknown-linux-musl/release/cyberkrill $out/bin/
            
            runHook postInstall
          '';
          
          # Ensure static linking
          doCheck = false; # Tests don't work well with static linking
          
          # Verify the binary is statically linked
          postInstall = ''
            echo "Checking if binary is statically linked..."
            file $out/bin/cyberkrill
            # Strip the binary to reduce size
            ${pkgs.binutils}/bin/strip $out/bin/cyberkrill
          '';
          
          meta = with pkgs.lib; {
            description = "CLI utility for Bitcoin and Lightning Network operations";
            homepage = "https://github.com/douglaz/cyberkrill";
            license = licenses.mit;
            maintainers = [ ];
          };
        };
        
        # Alternative dynamic build (non-static)
        packages.cyberkrill-dynamic = pkgs.rustPlatform.buildRustPackage {
          pname = "cyberkrill-dynamic";
          version = "0.1.0";
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "cktap-direct-0.1.0" = "sha256-yX9sFHWsisEXYBjYo2TrpC/vj58JO/b+Bb+phubkJNA=";
              "bip39-2.2.0" = "sha256-HVA5vJ24gU3ZFswk5MAMZ1mfdQOGOoBywwUlG9u72dU=";
              "frozenkrill-core-0.0.0" = "sha256-xCE26AxN4HgB3Yrfv3jgl0WYIl+xSmCKtWgJMjAql8w=";
              "coldcard-0.12.2" = "sha256-S+MARrWsdGCsfe4A3cUqaKSijo81MfH6KLIeuBpMckc=";
              "hidapi-compat-0.1.0" = "sha256-SYtLivsas+kkmhdsZSWQXF+AkCe4bMDryzJBfsZQhXE=";
              "trezor-client-0.1.5" = "sha256-MYBGRBGG818U65r0LmDOZCW09rCwa2R52Sh3WomKXek=";
            };
          };
          
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
          ];
          
          buildInputs = with pkgs; [
            libusb1
          ];
          
          # Use default features from Cargo.toml (all hardware wallets)
          # No need to specify buildFeatures
          
          meta = with pkgs.lib; {
            description = "CLI utility for Bitcoin and Lightning Network operations (dynamic build)";
            homepage = "https://github.com/douglaz/cyberkrill";
            license = licenses.mit;
            maintainers = [ ];
          };
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            bashInteractive
            rustToolchain
            pkg-config
            pkgsStatic.stdenv.cc
            libusb1
            pkgsStatic.libusb1
            gh
            cargo-edit
            cargo-outdated
          ];

          CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          CC_x86_64_unknown_linux_musl = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          
          # For static linking with musl
          LIBUSB_STATIC = "1";
          PKG_CONFIG_PATH = "${pkgs.pkgsStatic.libusb1}/lib/pkgconfig";
          
          # Add static libusb to the shell
          shellHook = ''
            export RUSTFLAGS="-L ${pkgs.pkgsStatic.libusb1}/lib"
            
            # Automatically configure Git hooks for code quality
            if [ -d .git ] && [ -d .githooks ]; then
              current_hooks_path=$(git config core.hooksPath || echo "")
              if [ "$current_hooks_path" != ".githooks" ]; then
                echo "📎 Setting up Git hooks for code quality checks..."
                git config core.hooksPath .githooks
                echo "✅ Git hooks configured automatically!"
                echo "   • pre-commit: Checks code formatting"
                echo "   • pre-push: Runs formatting and clippy checks"
                echo ""
                echo "To disable: git config --unset core.hooksPath"
              fi
            fi
          '';
        };
      }
    );
}
