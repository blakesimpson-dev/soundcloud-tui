{
  lib,
  stdenv,
  rustPlatform,
  pkgs,
  ...
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "soundcloud-tui";
  version = "beta";

  src = ./.;

  cargoHash = "";

  nativeBuildInputs = [
    stdenv.cc
    pkgs.pkg-config
  ];

  meta = {
    description = "A TUI for soundcloud";
    homepage = "https://github.com/Mvkweb/soundcloud-tui";
    lincese = lib.licenses.unlicense;
    maintainers = [ lib.maintainers.Mvkweb ];
  };

})
