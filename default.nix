{ lib
, darwin
, system
, naersk
, targetPlatform
, pkg-config
, libiconv
, rustfmt
, cargo
, rustc
, openssl
, # , llvmPackages # Optional
  # , protobuf     # Optional
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
  optionalItems = cond: items: if cond then items else [ ];
in
naersk.lib."${targetPlatform.system}".buildPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
  src = ./.;

  buildInputs = [
    cargo
    rustc
    rustfmt
    pkg-config
    libiconv
    openssl
  ] ++ (optionalItems (system == "aarch64-darwin") [
    darwin.apple_sdk.frameworks.SystemConfiguration
  ]);

  checkInputs = [
    cargo
    rustc
  ];

  doCheck = true;
  CARGO_BUILD_INCREMENTAL = "false";
  RUST_BACKTRACE = "full";
  copyLibs = true;

  # Optional things you might need:
  #
  # If you depend on `libclang`:
  # LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
  #
  # If you depend on protobuf:
  # PROTOC = "${protobuf}/bin/protoc";
  # PROTOC_INCLUDE = "${protobuf}/include";

  meta = with lib; {
    description = cargoToml.package.description;
    homepage = cargoToml.package.homepage;
    license = with licenses; [ mit asl20 ];
    maintainers = [ ];
  };
}
