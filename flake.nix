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
              "cktap-direct-0.1.0" = "sha256-ddQhghrmtwXKr750bTzjolSDLwyNZFUskNhJrR2vyBo=";
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
          
          # Build with smartcards feature by default
          buildFeatures = [ "smartcards" ];
          
          # Force cargo to use the musl target from .cargo/config.toml
          CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          CC_x86_64_unknown_linux_musl = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static -C link-arg=-static";
          
          # Override cargo target dir to use musl subdirectory
          preBuild = ''
            export CARGO_TARGET_DIR="target"
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
              "cktap-direct-0.1.0" = "sha256-ddQhghrmtwXKr750bTzjolSDLwyNZFUskNhJrR2vyBo=";
            };
          };
          
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
          ];
          
          buildInputs = with pkgs; [
            libusb1
          ];
          
          # Build with smartcards feature by default
          buildFeatures = [ "smartcards" ];
          
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
          ];

          CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          CC_x86_64_unknown_linux_musl = "${pkgs.pkgsStatic.stdenv.cc}/bin/${pkgs.pkgsStatic.stdenv.cc.targetPrefix}cc";
          
          # For static linking with musl
          LIBUSB_STATIC = "1";
          PKG_CONFIG_PATH = "${pkgs.pkgsStatic.libusb1}/lib/pkgconfig";
          
          # Add static libusb to the shell
          shellHook = ''
            export RUSTFLAGS="-L ${pkgs.pkgsStatic.libusb1}/lib"
          '';
        };
      }
    );
}
