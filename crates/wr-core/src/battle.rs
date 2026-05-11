//! Battle inputs, outputs, and the on-chain event log schema.
//!
//! `BattleInput` is everything the simulator needs to resolve a single
//! engagement; `BattleOutput` is everything it produces.  The output's
//! [`BattleOutput::output_hash`] field commits to the full event log
//! and is the single value an independent verifier compares against
//! the operator's published claim.

use serde::{Deserialize, Serialize};

use crate::ids::{AbilityId, HeroId, RoundNumber, RulesetVersion, Seed, TerritoryId, WalletAddress};
use crate::roster::{Roster, Side};
use crate::stat::Stat;
use crate::status::StatusEffect;
use crate::terrain::TerrainModifier;

// ---------------------------------------------------------------------
//   Inputs
// ---------------------------------------------------------------------

/// What a battle is fought against — another player's roster or a
/// territory's defense slot.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BattleTarget {
    /// Direct PvP: target the roster owned by `wallet`.
    Roster(WalletAddress),
    /// Territorial assault: contest the named territory's controller.
    Territory(TerritoryId),
}

/// Complete input to one invocation of `simulate(...)`.
///
/// Every field is reconstructible from on-chain data (or hashed onto
/// chain at commitment time).  This is what makes battles
/// re-derivable: anyone with chain access can rebuild this struct.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BattleInput {
    /// Round in which the battle was committed.  Used as the timeline
    /// anchor for XP grants and territory transfers.
    pub round: RoundNumber,

    /// Aggressor.
    pub roster_a: Roster,

    /// Defender.
    pub roster_b: Roster,

    /// Beacon-published seed for the round.  Combined with rosters by
    /// `wr-prng::battle_rng` to derive the per-battle PRNG seed.
    pub seed: Seed,

    /// Pinned ruleset version.  Subsequent ruleset upgrades have no
    /// effect on this battle.
    pub ruleset_version: RulesetVersion,

    /// Optional terrain modifier from the contested territory's class.
    /// `None` for player-vs-player skirmishes.
    pub terrain: Option<TerrainModifier>,
}

// ---------------------------------------------------------------------
//   Outputs
// ---------------------------------------------------------------------

/// XP awarded to a single hero as a result of the battle.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct XpGrant {
    /// Recipient.
    pub hero: HeroId,
    /// XP amount.
    pub amount: u64,
    /// Veterancy levels gained.  Zero if this XP grant did not cross a
    /// veterancy threshold.
    pub veterancy_delta: u32,
}

/// Complete output of `simulate(...)`.
///
/// `output_hash` is the byte-stable fingerprint of [`Self::events`] and
/// is what verifiers check.  The operator may publish only the hash on
/// chain; the events themselves are streamed to interested clients.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BattleOutput {
    /// Round in which this battle resolved.
    pub round: RoundNumber,

    /// Winning side.  In a draw — extremely rare under the standard
    /// ruleset's tie-break — defaults to [`Side::A`].
    pub winner: Side,

    /// Hero ids of A's roster that survived to battle end.  Sorted
    /// ascending for deterministic output.
    pub survivors_a: Vec<HeroId>,

    /// Hero ids of B's roster that survived to battle end.  Sorted
    /// ascending.
    pub survivors_b: Vec<HeroId>,

    /// Full event log.  Ordering is significant: each event's `tick`
    /// is monotonically non-decreasing within the log.
    pub events: Vec<BattleEvent>,

    /// XP / veterancy grants triggered by the battle.
    pub xp_grants: Vec<XpGrant>,

    /// Total ticks elapsed before the battle terminated.  Bounded by
    /// the active ruleset's `turn_cap`.
    pub total_ticks: u32,

    /// Blake3 hash of the canonical encoding of [`Self::events`].  See
    /// [`crate::canonical_hash`].
    pub output_hash: [u8; 32],
}

// ---------------------------------------------------------------------
//   Event log
// ---------------------------------------------------------------------

/// One entry in a battle's event log.
///
/// Variants are tagged with `#[repr(u8)]`-style discriminant numbers
/// implicitly by `bincode` — what we contractually guarantee is the
/// **declaration order** here, which `bincode` translates into stable
/// integer tags.  Reordering, inserting, or removing variants is a
/// breaking change for the on-chain log format.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BattleEvent {
    /// Emitted exactly once at the start of every battle.
    BattleStart {
        /// Number of heroes in roster A at battle start.
        roster_a_size: u8,
        /// Number of heroes in roster B at battle start.
        roster_b_size: u8,
    },

    /// Emitted at the start of each tick.
    TurnStart {
        /// Tick number, starting at 0.
        tick: u32,
    },

    /// A hero attacks another hero with a basic attack.
    Attack {
        /// Tick at which this attack resolves.
        tick: u32,
        /// Attacker.
        src: HeroId,
        /// Target.
        dst: HeroId,
        /// Final damage dealt after all modifiers, post-clamp.
        damage: Stat,
        /// Whether this attack was a critical hit.
        is_crit: bool,
    },

    /// A hero heals another hero (including self-heal: `src == dst`).
    Heal {
        /// Tick.
        tick: u32,
        /// Healer.
        src: HeroId,
        /// Target.
        dst: HeroId,
        /// HP restored.
        amount: Stat,
    },

    /// A hero activates an ability.
    Ability {
        /// Tick.
        tick: u32,
        /// Caster.
        src: HeroId,
        /// Ability identifier.
        ability: AbilityId,
        /// Heroes targeted by the ability, in resolution order.
        targets: Vec<HeroId>,
    },

    /// A status effect is applied to a hero.
    StatusApplied {
        /// Tick.
        tick: u32,
        /// Recipient.
        hero: HeroId,
        /// Effect kind.
        status: StatusEffect,
        /// Number of ticks the effect will persist for, including this
        /// one.
        duration: u8,
    },

    /// A status effect ends — either by duration expiry or by removal.
    StatusExpired {
        /// Tick.
        tick: u32,
        /// Hero from whom the effect departed.
        hero: HeroId,
        /// Effect kind.
        status: StatusEffect,
    },

    /// A hero is reduced to zero HP.
    Death {
        /// Tick.
        tick: u32,
        /// Hero who died.
        hero: HeroId,
        /// Killer, if attributable.  `None` for DoT and environmental
        /// kills.
        killer: Option<HeroId>,
    },

    /// Emitted exactly once at battle end, after all other events.
    BattleEnd {
        /// Tick at which termination was detected.
        tick: u32,
        /// Winning side.
        winner: Side,
    },
}

impl BattleEvent {
    /// The tick at which this event happened.
    #[inline]
    pub fn tick(&self) -> u32 {
        match self {
            Self::BattleStart { .. } => 0,
            Self::TurnStart { tick }
            | Self::Attack { tick, .. }
            | Self::Heal { tick, .. }
            | Self::Ability { tick, .. }
            | Self::StatusApplied { tick, .. }
            | Self::StatusExpired { tick, .. }
            | Self::Death { tick, .. }
            | Self::BattleEnd { tick, .. } => *tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_event_tick_accessor_returns_event_tick() {
        let e = BattleEvent::Attack {
            tick: 7,
            src: 1,
            dst: 2,
            damage: Stat::from_num(10),
            is_crit: false,
        };
        assert_eq!(e.tick(), 7);
    }

    #[test]
    fn battle_target_round_trips_through_bincode() {
        let t = BattleTarget::Territory(42);
        let bytes = bincode::serialize(&t).unwrap();
        let back: BattleTarget = bincode::deserialize(&bytes).unwrap();
        assert_eq!(t, back);
    }

    // Note: `battle_event_discriminants_are_position_stable` previously
    // lived here and only locked variants 0 and 8.  All nine variants
    // are now locked by the `golden_battle_event_variant_indices`
    // test in `tests/golden.rs`, which is the canonical source of
    // truth for wire-format stability.

    #[test]
    fn xp_grant_is_compact() {
        // hero (u64) + amount (u64) + veterancy_delta (u32) = 20 bytes
        let g = XpGrant { hero: 1, amount: 1000, veterancy_delta: 0 };
        let bytes = bincode::serialize(&g).unwrap();
        assert_eq!(bytes.len(), 20);
    }
}
