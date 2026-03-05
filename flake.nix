{
  description = "Flake for stella-typechecker (Rust + lalrpop dev shell)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in {
        devShell = pkgs.mkShell {
          buildInputs = [ pkgs.rustc pkgs.cargo ];
        };
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "stella-typechecker";
          version = "0.1.0";
          src = ./.;
        };
      }
    );
}
