# SQP QR Generator — Native Desktop App Design

**Status:** draft for review
**Date:** 2026-09-24
**Owner:** marcozanella
**Repo:** https://github.com/marcozanella/sqp-qr-generator

## Purpose

A native, offline desktop application that reproduces the SQP ink-canister
code generator currently implemented in the `platform01` Rails app
(`lib/sqp/*.rb`). The user enters the human-meaningful fields of a canister
(ink type, color, volume, expiry, count), and the app emits, per requested
code:

- the code as text (8 groups of 4 chars, dash-separated), and
- a QR code carrying the same 32 characters undashed, that can be saved and
  printed.

No Rails, no network, no database — a single self-contained binary per OS
(macOS first, then Windows, then Linux).

## Source of truth

The algorithm is a direct port of the Ruby generator in `platform01`:

| Ruby file | Responsibility |
|---|---|
| `lib/sqp/rfe32.rb` | 32-symbol base-32 codec (`ABCDEFGHJKLMNPQRSTUVWXYZ23456789`), 32 chars ↔ 20 bytes, big-endian |
| `lib/sqp/rc6.rb` | RC6 block cipher (20 rounds, 128-bit key/block), key schedule, encrypt/decrypt |
| `lib/sqp/byte_pool.rb` + `bytepool.bin` | 4096-byte pool used for key derivation |
| `lib/sqp/product_code.rb` | preview + plaintext bit-packing, `encode`/`decode`, `encode_with_retries`, `auto_batch` |
| `lib/sqp/color_map.rb` | color→id, volume→id, tara-by-volume, ink_type_id derivation |

The port must be **byte-identical** to the Ruby output. This is verifiable:
RFE32 and RC6 are exact integer algorithms with no floating point in the code
path, so identical inputs must yield identical strings.

## Architecture

A Cargo **workspace** with two crates:

### `sqp-core` — pure logic library (no UI)

- `rfe32.rs` — port of `Rfe32`. Uses a 160-bit big-endian integer
  (via `u128` pair or byte-array shifting) to convert 32 chars ↔ 20 bytes.
- `rc6.rs` — port of `Rc6`. All arithmetic on `u32` with wrapping ops
  (`wrapping_add`, `wrapping_mul`, `rotate_left`/`rotate_right`) to match
  Ruby's `& 0xFFFFFFFF` masking exactly.
- `bytepool.rs` — the 4096-byte pool embedded with `include_bytes!` so there
  is no external file to ship. (The `.bin` is copied into this repo as build
  input.)
- `codec.rs` — `parse_preview`/`pack_preview`, `parse_plaintext`/
  `pack_plaintext`, `derive_key`, `encode`, `decode`. Bit layouts copied
  exactly from the Ruby, including the `year` constants (`+2007`, `0x2CC`,
  `0x7D7`) and the little/big-endian word packing.
- `maps.rs` — `COLOR_NAME_TO_ID`, `VOLUME_TO_ID`, `TARA_RAW_BY_VOLUME`,
  ink-type-id derivation, `SUPPORTED_COLOR_NAMES`, and the `SQSG3 + Clear`
  rejection.
- `generate.rs` — public API mirroring `encode_with_retries` + `auto_batch`:
  - randomizes `number` (0..=0xFFFF) and `legacy_tara_raw` (from the
    per-volume pool) using the OS RNG,
  - self-checks each code by decoding and confirming the cross-checks pass
    (same guard as the Ruby `encoder regression` check),
  - dedups against codes already produced **in the current session**.
  - `auto_batch(date)` → `"UB%y%jF1"`.

Public surface (illustrative):

```rust
pub enum InkType { Kx2, Sqsg3 }
pub struct GenerateRequest {
    pub ink_type: InkType,
    pub color: String,     // one of SUPPORTED_COLOR_NAMES
    pub volume_l: u8,      // 1 or 5
    pub expires_year: i32, // 4-digit
    pub expires_month: u8, // 1..=12
    pub batch: String,     // [A-Z0-9]{9}
    pub count: u8,         // 1..=20
}
pub struct GeneratedCode {
    pub code: String,          // dashed
    pub code_undashed: String, // for the QR
    pub number: u16,
    pub legacy_tara_raw: u8,
}
pub fn generate(req: &GenerateRequest, already: &HashSet<String>)
    -> Result<Vec<GeneratedCode>, SqpError>;
```

### `sqp-gui` — desktop app (egui / eframe)

- Single window. Left: input form. Right/below: results list.
- Inputs mirror the Rails form, with the two auto-filled fields **editable**:
  - **Ink type** — dropdown (KX2 / SQSG3).
  - **Color** — dropdown of the 9 supported names; `SQSG3 + Clear` disabled/
    rejected with an inline message.
  - **Volume** — radio 1 L / 5 L.
  - **Expiry** — month + year, pre-filled to +12 months from today, editable.
  - **Batch** — pre-filled `auto_batch(today)`, editable, validated
    `^[A-Z0-9]{9}$`.
  - **Count** — 1..=20.
- **Generate** button → calls `sqp_core::generate`, appends results.
- Per result: the dashed code (monospace, selectable/copyable) and the QR
  rendered on screen. Actions:
  - **Copy code** to clipboard.
  - **Save QR (PNG)** — file dialog (`rfd` crate).
  - **Export all** — a folder of PNGs plus a `codes.csv`
    (columns: code, ink_type, color, volume, expires, batch, number).
- QR generation via the `qrcode` crate → `image` buffer for both the on-screen
  texture and the saved PNG. Error-correction level **L** and no border, to
  match the Rails labels (`RQRCode ... level: :l`, `border_modules: 0`).

## Data flow

```
form fields ──▶ GenerateRequest ──▶ sqp_core::generate
                                        │  (encode → self-check decode)
                                        ▼
                                 Vec<GeneratedCode>
                                        │
                     ┌──────────────────┼───────────────────┐
                     ▼                  ▼                    ▼
              on-screen text      QR texture (egui)     Save PNG / CSV
```

## Correctness strategy (TDD)

1. **Generate fixtures from Ruby.** Run the `platform01` encoder over a fixed
   matrix of inputs (each ink/color/volume combo, a couple of expiries and
   batches, with `number` and `legacy_tara_raw` pinned to constants rather than
   random) and capture the exact output strings into a fixture file committed
   to this repo. This is a read-only use of `platform01` (run its code, copy
   the strings out; nothing written there).
2. **Unit-test each layer** in `sqp-core` bottom-up: RFE32 round-trip, RC6
   encrypt/decrypt round-trip, preview pack/parse, plaintext pack/parse,
   key derivation, then full `encode`/`decode`.
3. **Vector test**: `encode(fixed fields)` must equal the captured Ruby string;
   `decode` of it must reproduce the fields with all cross-checks true.
4. Only after `sqp-core` is green do we build the GUI.

## Error handling

- `sqp-core` returns a `SqpError` enum: `UnknownColor`, `UnknownVolume`,
  `UnknownInkType`, `InvalidCombo` (SQSG3+Clear), `BadBatch`, `EncoderRegression`,
  `CollisionExhausted`.
- The GUI validates before calling and shows inline messages; it never panics
  on bad input.
- `MAX_DUP_RETRIES = 20`, same as Ruby.

## Uniqueness caveat (explicit)

The Rails app dedups against the whole database. This desktop app has no
database, so uniqueness is enforced only against codes generated **within the
current run**. Collision probability is negligible (16-bit number × tara ×
crypto), but the app is not a system of record. This will be stated in the
README.

## Build & distribution

- **macOS (first):** `cargo run -p sqp-gui` for development; a distributable
  `.app` via `cargo-bundle` (or `cargo-packager`). Document both.
- **Windows (next):** native `cargo build --release` on Windows, or
  cross-build from macOS via the `x86_64-pc-windows-gnu` target; ship the
  `.exe`. egui needs no extra runtime.
- **Linux (eventually):** `cargo build --release` produces a single binary.
- README covers: install Rust (`rustup`), `cargo run`, release builds, and
  per-OS packaging.

## Repository layout

```
SQP-QR-generator/
├── Cargo.toml            # workspace
├── crates/
│   ├── sqp-core/
│   │   ├── src/{lib,rfe32,rc6,bytepool,codec,maps,generate}.rs
│   │   ├── assets/bytepool.bin
│   │   └── tests/vectors.rs
│   └── sqp-gui/
│       └── src/main.rs
├── docs/superpowers/specs/2026-09-24-sqp-qr-generator-design.md
└── README.md
```

## Out of scope (YAGNI)

- Decoding/scanning existing codes in the GUI (decode exists in `sqp-core`
  only for the self-check).
- The Rails PDF label layouts (grid/single/round). We ship plain QR PNGs; a
  print-layout feature can be a later slice if wanted.
- Persistence, accounts, import, print jobs — all Rails-only concerns.
