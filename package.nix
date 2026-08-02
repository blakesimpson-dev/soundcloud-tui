{
  lib,
  rustPlatform,
  pkg-config,
  openssl,
  dbus,
  mpv,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "soundcloud-tui";
  version = "beta";

  src = ./.;

  cargoHash = "sha256-JB7Kj2X6nGN3qLf69KP94NN1uLXUXFWI9Zm+UAMMe2I=";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
    dbus
    mpv
  ];

  meta = {
    description = "A TUI for soundcloud";
    homepage = "https://github.com/Mvkweb/soundcloud-tui";
    license = lib.licenses.unlicense;
  };
})
