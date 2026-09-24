# SQP QR Generator — Project Guide

Native, offline **desktop app in Rust** that reproduces the `platform01` Rails
SQP ink-canister code generator and renders each code as text plus a printable
QR. Targets macOS first, then Windows, then Linux — a single binary per OS.

- GitHub: https://github.com/marcozanella/sqp-qr-generator
- Design doc: `docs/superpowers/specs/2026-09-24-sqp-qr-generator-design.md`
- Implementation plan: `docs/superpowers/plans/2026-09-24-sqp-qr-generator.md`

## Reference folders are READ-ONLY

Only ever write inside this repo (`SQP-QR-generator`). These sibling folders
are information sources — never create, edit, or delete anything in them:

- `../platform01` — the Rails app. The SQP algorithm being ported lives in
  `platform01/lib/sqp/*.rb` (`rfe32.rb`, `rc6.rb`, `byte_pool.rb` +
  `bytepool.bin`, `product_code.rb`, `color_map.rb`). This is the source of
  truth for correctness.
- `../amber-re` — background analysis notes and sample codes.

## Architecture

Cargo workspace with two crates:

- `crates/sqp-core` — pure logic library, **no UI**. Byte-for-byte port of the
  Ruby algorithm: `rfe32.rs` (base-32 codec), `rc6.rs` (RC6 cipher),
  `bytepool.rs` (4096-byte pool embedded via `include_bytes!`), `codec.rs`
  (bit-packing + encode/decode), `maps.rs` (color/volume/tara/ink-type),
  `generate.rs` (public `generate()` + `auto_batch()`).
- `crates/sqp-gui` — egui/eframe desktop front-end (form → codes + QR, save
  PNG / export CSV).

## Non-negotiable: byte-identical output

`sqp-core::encode` MUST produce the exact same string as the Ruby
`Sqp::ProductCode.encode` for the same fields. The plan captures real
reference vectors from the Ruby encoder; `crates/sqp-core/tests/vectors.rs`
enforces them. All RC6 `u32` math uses `wrapping_*` / `rotate_*`; RC6 words
are little-endian, the RFE32 view is big-endian.

## Commands

```bash
cargo test -p sqp-core        # algorithm tests (run these first)
cargo run  -p sqp-gui         # launch the app
cargo build --release -p sqp-gui
```

## Workflow

This project follows the Superpowers flow: brainstorm → spec → plan →
test-first implementation. Implement the plan task-by-task; each task is a
red→green→commit cycle. Keep commits frequent and scoped to one task.
