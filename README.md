# SQP QR Generator

Native desktop app that generates SQP ink-canister codes and printable QR
codes. Offline; a single binary per OS (macOS, Windows, Linux).

> **Status:** design + implementation plan committed; code is built
> task-by-task per `docs/superpowers/plans/2026-09-24-sqp-qr-generator.md`.

## Self-contained repository

This repo is **fully self-contained** — it needs no sibling folders or
external services to build, test, or run. The SQP algorithm was ported from a
separate Rails app, but everything required is copied in here:

- the 4096-byte key pool is embedded in the binary
  (`crates/sqp-core/assets/bytepool.bin` via `include_bytes!`), and
- the reference test vectors are committed in `crates/sqp-core/tests/`.

That means a cloud Claude Code session (or any fresh clone) can build and
verify the project from this repository alone.

## Prerequisites

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

## Notes

- Uniqueness is enforced only within a single run of the app (no database).
- QR error-correction level L, no border, matching the production labels.
