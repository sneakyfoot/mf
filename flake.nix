{
  description = "Mana Farm - k8s farm cli";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      mkPkgs = system: import nixpkgs { inherit system; };
      mkMfPackage = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "mf";
        version = "0.1.0";
        src = pkgs.lib.cleanSource ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
        };
      };
    in {
      packages = forAllSystems (system:
        let
          pkgs = mkPkgs system;
          mfPackage = mkMfPackage pkgs;
        in {
          default = mfPackage;
          mf = mfPackage;
        });

      apps = forAllSystems (system:
        let
          pkgs = mkPkgs system;
          mfPackage = mkMfPackage pkgs;
        in {
          default = {
            type = "app";
            program = "${mfPackage}/bin/mf";
          };
        });

      devShells = forAllSystems (system:
        let
          pkgs = mkPkgs system;
          mfPackage = mkMfPackage pkgs;
        in {
          default = pkgs.mkShell {
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
        });
    };
}
