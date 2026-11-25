# Пакет barricade-bar.
#   nix-build                       # сборка без flake
#   pkgs.callPackage ./default.nix {}  # из своей конфигурации
{
  pkgs ? import <nixpkgs> { },
  lib ? pkgs.lib,
  rustPlatform ? pkgs.rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "barricade-notifications";
  version = (lib.importTOML ./Cargo.toml).package.version;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = with pkgs; [
    pkg-config
    wrapGAppsHook3
  ];

  buildInputs = with pkgs; [
    gtk3
    glib
    cairo
    pango
    gdk-pixbuf
    gtk-layer-shell
    dbus
    networkmanager
    libpulseaudio
    adwaita-icon-theme # символические иконки freedesktop
  ];

  # wpctl нужен виджету audio во время работы
  preFixup = ''
    gappsWrapperArgs+=(--prefix PATH : ${lib.makeBinPath [ pkgs.wireplumber ]})
  '';

  meta = {
    description = "календарь и менеджер уведомлений на gtk3";
    homepage = "https://github.com/yarok-k/barricade-notifications";
    license = lib.licenses.mit;
    mainProgram = "barricade-notifications";
    platforms = lib.platforms.linux;
  };
}
