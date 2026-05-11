//! Primitive identifier aliases.
//!
//! These are deliberately *type aliases* rather than newtypes: every
//! identifier in `wr-core` ends up serialized to and from the chain,
//! and a transparent representation keeps the wire format trivially
//! inspectable without knowing the Rust type system.

/// Stable identifier for a hero (= ERC-721 token id, narrowed to 64 bits
/// for off-chain use).  The on-chain id is a `uint256`; the simulator
/// requires only that the supply fits in `u64`, which the issuance
/// schedule guarantees.
pub type HeroId = u64;

/// Identifier for an ability slot.  Definitions live in the on-chain
/// ruleset registry and are referenced from [`crate::Hero::abilities`].
pub type AbilityId = u32;

/// Identifier for an equippable item.  Items are ERC-1155 tokens; only
/// the type id is meaningful for combat resolution.
pub type ItemId = u32;

/// Identifier for a territory in the territory registry.
pub type TerritoryId = u32;

/// Monotonically-increasing version number of a published ruleset.  The
/// simulator binds a battle to the ruleset version active at roster
/// commitment time and never observes later upgrades.
pub type RulesetVersion = u32;

/// Round counter, starting at zero and incrementing once per beacon
/// publication.
pub type RoundNumber = u64;

/// Length, in bytes, of a beacon-published seed.
pub const SEED_LEN: usize = 32;

/// 256-bit seed value published by the beacon.  Always exactly
/// [`SEED_LEN`] bytes.
pub type Seed = [u8; SEED_LEN];

/// Length, in bytes, of an EVM wallet address.
pub const WALLET_LEN: usize = 20;

/// EVM wallet address (20 bytes).
pub type WalletAddress = [u8; WALLET_LEN];
