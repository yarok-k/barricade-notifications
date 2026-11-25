# Barricade Notifications [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[English](README.md) | [Русский](README_RU.md)

Desktop notification daemon and popup (notifications + calendar) for Wayland compositors built with Rust, GTK 3, `gtk-layer-shell`, and `zbus`.

## Features

- Implements `org.freedesktop.Notifications` over the D-Bus session bus
- Overlay notification popups across connected monitors with hotplug support and smooth animations
- Toggleable popup combining a calendar widget and active notifications list with close buttons
- Automatic collapsing of on-screen notification tiles when opening the popup
- Live reloading of configuration and CSS stylesheet

## Requirements

- Rust (edition 2024)
- GTK 3.x
- `gtk-layer-shell` 0.7+
- D-Bus session bus (make sure no conflicting notification daemon like `mako` or `dunst` is occupying `org.freedesktop.Notifications`)

## Build & Run

```bash
cargo build --release
cargo run
