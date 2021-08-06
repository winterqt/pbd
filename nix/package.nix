{ lib, stdenv, rustPlatform, Security }:

rustPlatform.buildRustPackage {
  pname = "pbd";
  version = (lib.importTOML ../Cargo.toml).package.version;

  src = ../.;
  cargoLock.lockFile = ../Cargo.lock;

  doCheck = false;

  buildInputs = lib.optionals stdenv.isDarwin [ Security ];
}
