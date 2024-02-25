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
  };

  outputs =
    { self
    , nixpkgs
    , naersk
    , ...
    }:
    let
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

      # https://ayats.org/blog/no-flake-utils/
      forSystems = func:
        nixpkgs.lib.genAttrs [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ]
          (system: func (import nixpkgs {
            inherit system;
            config.allowUnfree = true;
            overlays = [
              self.overlays.default
            ];
          }));

    in
    {
      overlays = {
        default = final: prev: {
          "${cargoToml.package.name}" = final.callPackage ./. { inherit naersk; };
        };
      };

      packages = forSystems (pkgs: {
        default = pkgs."${cargoToml.package.name}";
        "${cargoToml.package.name}" = pkgs."${cargoToml.package.name}";
      });

      apps = forSystems (pkgs: {
        default = {
          type = "app";
          program = "${pkgs."${cargoToml.package.name}"}/bin/${cargoToml.package.name}";
        };
      });

      devShells = forSystems (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ pkgs."${cargoToml.package.name}" ];
          buildInputs = with pkgs; [ rustfmt nixpkgs-fmt ];
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        };
      });

      checks = forSystems (pkgs: {
        format = pkgs.runCommand "check-format"
          {
            buildInputs = with pkgs; [ rustfmt cargo nixpkgs-fmt shellcheck ];
          }
          ''
            shopt -s globstar nullglob
            pushd "${./.}"

            for file in ./**/Cargo.toml; do
              cargo-fmt fmt --manifest-path "''${file}" -- --check
            done

            nixpkgs-fmt --check .

            for file in ./scripts/*.sh; do
              shellcheck --severity=info "''${file}"
            done

            popd

            touch $out # it worked!
          '';
        "${cargoToml.package.name}" = pkgs."${cargoToml.package.name}";
      });
    };
}
