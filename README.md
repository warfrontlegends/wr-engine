# `wr-engine`

> Canonical types and hashing primitives for the [Warfront Legends](https://warfrontlegends.com) battle simulator.

`wr-engine` is the foundation of the Warfront Legends off-chain battle simulator. Every value that crosses the simulator boundary — every input the chain commits, every output a verifier compares — is defined here.

The repository is a Cargo workspace. Today it contains one crate; more determinism-critical primitives may land here as the simulator grows.

| Crate | Purpose |
|---|---|
| [`wr-core`](crates/wr-core) | Hero, Roster, BattleInput, BattleOutput, BattleEvent, canonical hashing, fixed-point `Stat` type |

## Why this exists

The Warfront Legends whitepaper requires that battle outcomes be **re-derivable** by any third party from on-chain inputs. To honor that contract, every type that crosses the simulator boundary must serialize to the same bytes on every architecture, every release, forever.

`wr-engine` is where that promise is mechanically enforced. The crate is designed around four invariants:

1. **No floating-point.** Numeric stats use `fixed::types::I32F32` — a 64-bit two's-complement value with 32 fractional bits. Bit-identical across `x86_64`, `aarch64`, and `wasm32`.
2. **No ambient randomness or clocks.** Nothing in `wr-engine` reads a wall clock or asks the OS for entropy. All randomness is injected by callers (see [`wr-prng`](https://github.com/warfrontlegends/wr-prng)).
3. **Stable serialization.** Every public type derives `Serialize` / `Deserialize` with field order matching declaration order, so `bincode` produces a canonical byte stream that can be hashed and compared without ambiguity.
4. **Explicit endianness.** Hashing helpers always encode integers little-endian.

These invariants are enforced by lints (`unsafe_code = "forbid"`, `clippy::float_arithmetic = "deny"`), a compile-time assertion that `size_of::<Stat>() == 8`, golden byte tests, and property-based round-trip tests.

## Layout

```
wr-engine/
├── Cargo.toml                # workspace manifest
├── rust-toolchain.toml       # pinned toolchain (stable, 1.83+)
└── crates/
    └── wr-core/
        ├── src/
        │   ├── lib.rs        # module exports
        │   ├── battle.rs     # BattleInput, BattleOutput, BattleEvent, XpGrant
        │   ├── hero.rs       # Hero, HeroClass, Rarity
        │   ├── roster.rs     # Roster, Side, ROSTER_MIN/MAX_SIZE
        │   ├── stat.rs       # Stat = I32F32 + STAT_ZERO/ONE/HALF/TWO
        │   ├── status.rs     # StatusEffect (8 variants)
        │   ├── terrain.rs    # TerrainModifier
        │   ├── ids.rs        # HeroId, AbilityId, ItemId, Seed, WalletAddress, ...
        │   └── hash.rs       # canonical_hash, aggregate_round_root, domain tags
        └── tests/
            ├── golden.rs              # 11 pinned byte/hash snapshot tests
            └── proptest_roundtrip.rs  # 7 property-based round-trip tests
```

## Determinism guarantees (mechanically enforced)

The following are not aspirations. They are tests that fail loudly the moment they are violated.

| Invariant | Where it is enforced |
|---|---|
| `size_of::<Stat>() == 8` | Compile-time `const _: () = assert!(...)` in `crates/wr-core/src/stat.rs` |
| No `unsafe` code anywhere | `[lints.rust] unsafe_code = "forbid"` |
| No floating-point arithmetic | `[lints.clippy] float_arithmetic = "deny"`, `float_cmp = "deny"` |
| `Stat` bincode-serializes to exactly 8 bytes | `golden_stat_from_num_42` in `tests/golden.rs` |
| `Side`, `StatusEffect`, `HeroClass`, `Rarity` have stable u8 discriminants | `#[repr(u8)]` + `golden_*_encoding` tests |
| `Hero` bincode-serializes to a 112-byte canonical layout | `golden_hero_serialization` in `tests/golden.rs` |
| All 9 `BattleEvent` variants have position-stable discriminants | `golden_battle_event_variant_indices` |
| `canonical_hash` of a fixed event log produces a specific 32-byte Blake3 digest | `golden_canonical_hash_of_minimal_log` |
| `aggregate_round_root` is independent of input order | `round_root_is_independent_of_input_order` + golden |
| `BattleInput`, `BattleOutput`, `BattleEvent` round-trip cleanly through bincode | `tests/proptest_roundtrip.rs` × 256 cases each |
| Hash function is exact-pinned (`blake3 = "=1.8.5"`) | Workspace `Cargo.toml` |
| Fixed-point library is exact-pinned (`fixed = "=1.28.0"`) | Workspace `Cargo.toml` |

If a test in `tests/golden.rs` fails after a refactor, that refactor has changed the on-the-wire format of a type that crosses the simulator boundary. Independent verifiers running the old format will compute different hashes than the operator publishing under the new format — silent consensus break. Treat such a failure as a hard release blocker.

## Build & test

```bash
cargo build --workspace --release
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

All 46 tests should pass; clippy should report no warnings.

## Domain tags (immutable consensus bytes)

The following byte strings appear in every published hash and are part of the protocol's consensus surface. **They cannot change without a hard fork.**

| Constant | Value | Used for |
|---|---|---|
| `DOMAIN_BATTLE_OUTPUT` | `b"WFL-BATTLE-OUTPUT-V1"` | Hashing a battle's event log into `BattleOutput::output_hash` |
| `DOMAIN_ROUND_ROOT`    | `b"WFL-ROUND-ROOT-V1"`    | Aggregating per-battle hashes into a per-round root |
| `DOMAIN_ROSTER_COMMITMENT` | `b"WFL-ROSTER-COMMIT-V1"` | Hashing a roster's canonical encoding into its on-chain commitment |

Adding a new tag is fine. Reusing or renaming an existing tag is a breaking change to every verifier in the wild.

## Versioning policy

Field order, enum variant order, discriminant values, and the byte width of every public type are part of the wire contract. Reordering, inserting, or removing any of them is a major-version bump and must be coordinated with a ruleset version bump on the chain side.

The golden tests in `tests/golden.rs` are deliberately strict — they exist to make accidental wire-format changes impossible to land silently.

## Related repositories

`wr-engine` is one of six crates that make up the Warfront Legends backend. The full stack:

| Repository | Role |
|---|---|
| **`wr-engine`** (this) | Shared types and canonical hashing |
| [`wr-prng`](https://github.com/warfrontlegends/wr-prng) | Deterministic per-battle PRNG (ChaCha20 + Blake3) |
| [`wr-ruleset`](https://github.com/warfrontlegends/wr-ruleset) | Versioned ruleset spec (damage curves, class matchups, XP) |
| [`wr-sim`](https://github.com/warfrontlegends/wr-sim) | Canonical battle simulator (Bevy ECS, headless) |
| [`wr-server`](https://github.com/warfrontlegends/wr-server) | Round orchestrator + Renet UDP broadcaster |
| [`wr-verifier`](https://github.com/warfrontlegends/wr-verifier) | Independent re-derivation CLI |

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([`LICENSE-MIT`](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Links

| Resource | URL |
|---|---|
| Website | <https://warfrontlegends.com> |
| App | <https://play.warfrontlegends.com> |
| Documentation | <https://docs.warfrontlegends.com> |
| Whitepaper | <https://warfrontlegends.com/whitepaper.pdf> |
| X / Twitter | <https://x.com/warfrontlegends> |
| Telegram | <https://t.me/warfrontlegends> |
