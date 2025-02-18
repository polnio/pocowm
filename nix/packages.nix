{
  # Inputs
  crane,

  # Tools
  lib,
  clippy,
  pkg-config,
  pkgs,

  # Dependencies
  cairo,
  dbus,
  egl-wayland,
  libGL,
  libdisplay-info,
  libinput,
  libxkbcommon,
  mesa,
  pango,
  seatd,
  wayland,
}:
let
  craneLib = crane.mkLib pkgs;
in
craneLib.buildPackage rec {
  src = craneLib.cleanCargoSource ../.;
  strictDeps = true;
  nativeBuildInputs = [
    clippy
    pkg-config
  ];
  buildInputs = [
    cairo
    dbus
    egl-wayland
    libGL
    libdisplay-info
    libinput
    libxkbcommon
    mesa
    pango
    seatd
    wayland
  ];
  LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
}
