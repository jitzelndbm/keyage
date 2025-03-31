{
  rustPlatform,
  pkg-config,
  openssl,
}:
let
  inherit (rustPlatform) buildRustPackage;
in
buildRustPackage {
  pname = "keyage";
  version = "0.1.0";
  src = ../.;
  cargoLock.lockFile = ../Cargo.lock;
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl ];
}
