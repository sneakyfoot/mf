{
  description = "Mana Farm - k8s farm cli";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      mfPackage = pkgs.rustPlatform.buildRustPackage {
        pname = "mf";
        version = "0.1.0";
        src = pkgs.lib.cleanSource ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
        };
      };
    in {
      packages.${system} = {
        default = mfPackage;
        mf = mfPackage;
      };

      apps.${system}.default = {
        type = "app";
        program = "${mfPackage}/bin/mf";
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          rustc
          cargo
          rust-analyzer
          rustfmt
          pkg-config
          kubectl
        ];
        inputsFrom = [ mfPackage ];
      };
    };
}
