# SQP QR Generator

Native desktop app that generates SQP ink-canister codes and printable QR
codes. Offline; a single binary per OS (macOS, Windows, Linux).

> **Status:** implemented. `sqp-core` is a byte-for-byte port of the Ruby
> generator, verified against committed reference vectors; `sqp-gui` is the
> egui/eframe desktop front-end. Built per
> `docs/superpowers/plans/2026-09-24-sqp-qr-generator.md`.

## Self-contained repository

This repo is **fully self-contained** — it needs no sibling folders or
external services to build, test, or run. The SQP algorithm was ported from a
separate Rails app, but everything required is copied in here:

- the 4096-byte key pool is embedded in the binary
  (`crates/sqp-core/assets/bytepool.bin` via `include_bytes!`), and
- the reference test vectors are committed in `crates/sqp-core/tests/`.

That means a cloud Claude Code session (or any fresh clone) can build and
verify the project from this repository alone.

## Download a prebuilt macOS app (no Rust needed)

Every push builds a **universal** macOS app (Apple Silicon + Intel) via GitHub
Actions. To get it:

1. Open the repo's **Actions** tab → the latest "CI + macOS build" run.
2. Under **Artifacts**, download **`SQP-QR-Generator-macos-app`**.
3. Unzip it, then unzip the inner `SQP-QR-Generator-macos-universal.zip` to get
   `SQP QR Generator.app`.
4. The app is unsigned, so on first launch **right-click → Open** (once) to get
   past Gatekeeper. If macOS still blocks it:
   `xattr -dr com.apple.quarantine "SQP QR Generator.app"`.

## Prerequisites (to build locally)

- Rust (stable) via https://rustup.rs

## Run (development)

    cargo run -p sqp-gui

## Test the core algorithm

    cargo test -p sqp-core

The core tests assert **byte-identical output** to the original generator via
committed reference vectors — run these first after any change to `sqp-core`.

## Release build (current OS)

    cargo build --release -p sqp-gui

Binary: `target/release/sqp-gui` (`sqp-gui.exe` on Windows).

## macOS .app bundle

    cargo install cargo-bundle
    cargo bundle --release -p sqp-gui

Bundle: `target/release/bundle/osx/SQP QR Generator.app`

## Windows

- Native: on Windows, `cargo build --release -p sqp-gui` → `target\release\sqp-gui.exe`
- Cross-compile from macOS:

      rustup target add x86_64-pc-windows-gnu
      brew install mingw-w64
      cargo build --release -p sqp-gui --target x86_64-pc-windows-gnu

## Linux

    cargo build --release -p sqp-gui

Binary: `target/release/sqp-gui`

Running the GUI on Linux needs the usual desktop libraries at runtime (X11 or
Wayland, plus `libxkbcommon`); on Debian/Ubuntu:

    sudo apt-get install libx11-dev libxkbcommon-dev libwayland-dev

## Notes

- Uniqueness is enforced only within a single run of the app (no database).
- QR error-correction level L, no border, matching the production labels.
