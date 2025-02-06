{
  description = "cyberkrill";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        f = with fenix.packages.${system}; combine [
          stable.toolchain
        ];
      in
      {
        devShells.default = with pkgs; mkShell {
          packages = with pkgs; [

          ];
        };

        defaultPackage = pkgs.rustPlatform.buildRustPackage {
          pname = "cyberkrill";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock; 
          nativeBuildInputs = [  ];
          buildInputs = [ ] ;
        };
      }
    );
}
