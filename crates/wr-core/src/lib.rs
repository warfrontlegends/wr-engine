//! # `wr-core`
//!
//! Canonical types for the Warfront Legends battle simulator.
//!
//! Every value that crosses the simulator boundary — every input the
//! chain commits, every output the simulator produces — is defined in
//! this crate.  Higher-level crates (`wr-prng`, `wr-ruleset`, `wr-sim`,
//! `wr-server`, `wr-verifier`) depend on `wr-core` and on nothing else
//! from the workspace.
//!
//! ## Determinism contract
//!
//! `wr-core` enforces, by construction:
//!
//! * **No floating-point.**  Numeric stats use [`Stat`] (= `fixed::I32F32`),
//!   a 64-bit two's-complement value with 32 fractional bits.  Bitwise
//!   identical across `x86_64`, `aarch64`, and `wasm32`.
//! * **No ambient randomness.**  Nothing in `wr-core` reads a clock or
//!   asks the OS for entropy.  All randomness is injected by callers.
//! * **Stable serialization.**  Every public type derives `Serialize` /
//!   `Deserialize` with field order matching declaration order, so
//!   `bincode` produces a canonical byte stream.
//! * **Explicit endianness.**  Hashing helpers always encode integers
//!   little-endian.
//!
//! These invariants are audited at the workspace level; see the
//! `Determinism Audit Checklist` in `github/tech_solutioning.md`.
//!
//! ## Module layout
//!
//! | Module      | Responsibility                                    |
//! |-------------|---------------------------------------------------|
//! | [`stat`]    | The `Stat` fixed-point alias + zero/one constants |
//! | [`hero`]    | `Hero`, `HeroClass`, `Rarity`                     |
//! | [`roster`]  | `Roster`, `Side`                                  |
//! | [`battle`]  | `BattleInput`, `BattleOutput`, `BattleEvent`, …   |
//! | [`status`]  | `StatusEffect`                                    |
//! | [`terrain`] | `TerrainModifier`                                 |
//! | [`hash`]    | Canonical hashing helpers                         |
//! | [`ids`]     | Type aliases for primitive identifiers            |

#![deny(missing_debug_implementations)]
#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod battle;
pub mod hash;
pub mod hero;
pub mod ids;
pub mod roster;
pub mod stat;
pub mod status;
pub mod terrain;

pub use battle::{
    BattleEvent, BattleInput, BattleOutput, BattleTarget, XpGrant,
};
pub use hash::{
    aggregate_round_root, canonical_hash, canonical_hash_with_tag, DOMAIN_BATTLE_OUTPUT,
    DOMAIN_ROUND_ROOT, DOMAIN_ROSTER_COMMITMENT,
};
pub use hero::{Hero, HeroClass, Rarity};
pub use ids::{
    AbilityId, HeroId, ItemId, RoundNumber, RulesetVersion, Seed, TerritoryId,
    WalletAddress, SEED_LEN, WALLET_LEN,
};
pub use roster::{Roster, Side};
pub use stat::{Stat, STAT_ONE, STAT_ZERO};
pub use status::StatusEffect;
pub use terrain::TerrainModifier;
