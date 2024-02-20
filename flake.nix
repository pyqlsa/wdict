# with help from https://hoverbear.org/blog/a-flake-for-your-crate/
{
  description = "Create dictionaries by scraping webpages.";

  inputs = {
    nixpkgs = {
      url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    };
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
  };

  outputs =
    { self
    , nixpkgs
    , naersk
    , flake-utils
    , ...
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ self.overlays.${system}.default ];
      };
    in
    {
      overlays = {
        default = final: prev: {
          "${cargoToml.package.name}" = final.callPackage ./. { inherit naersk; };
        };
      };

      packages = {
        default = pkgs."${cargoToml.package.name}";
      };

      apps = {
        default = {
          type = "app";
          program = "${pkgs."${cargoToml.package.name}"}/bin/${cargoToml.package.name}";
        };
      };

      devShells = {
        default = pkgs.mkShell {
          inputsFrom = [ pkgs."${cargoToml.package.name}" ];
          buildInputs = with pkgs; [ rustfmt nixpkgs-fmt ];
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        };
      };

      checks = {
        format = pkgs.runCommand "check-format"
          {
            buildInputs = with pkgs; [ rustfmt cargo ];
          } ''
          ${pkgs.rustfmt}/bin/cargo-fmt fmt --manifest-path ${./.}/Cargo.toml -- --check
          ${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt --check ${./.}
          touch $out # it worked!
        '';
        "${cargoToml.package.name}" = pkgs."${cargoToml.package.name}";
      };
    });
}
