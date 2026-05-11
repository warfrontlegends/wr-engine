//! Hero data model.
//!
//! A [`Hero`] is the unit of agency in combat.  Heroes are minted as
//! ERC-721 tokens; the on-chain attributes are mirrored here in a form
//! the simulator can consume directly.

use core::cmp::Reverse;

use serde::{Deserialize, Serialize};

use crate::ids::{AbilityId, HeroId, ItemId};
use crate::stat::Stat;

/// Tactical role of a hero.  Drives the class-matchup table in the
/// active ruleset.
///
/// The discriminants are stable: the on-chain hero contract emits these
/// same values, and downstream consumers index into a 5x5 matchup
/// matrix using `class as usize`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HeroClass {
    /// Frontline melee.  Strong against Artillery, weak to Armor.
    Infantry = 0,
    /// Heavy armor.  Strong against Infantry, weak to Support.
    Armor = 1,
    /// Long-range damage.  Strong against Armor, weak to Infantry.
    Artillery = 2,
    /// Buffer / debuffer.  Strong against Armor, weak to Command.
    Support = 3,
    /// Force multiplier.  Strong against Support, weak to Artillery.
    Command = 4,
}

impl HeroClass {
    /// All five hero classes, in stable discriminant order.
    pub const ALL: [HeroClass; 5] = [
        HeroClass::Infantry,
        HeroClass::Armor,
        HeroClass::Artillery,
        HeroClass::Support,
        HeroClass::Command,
    ];

    /// The number of distinct hero classes.  Equal to `Self::ALL.len()`.
    pub const COUNT: usize = 5;
}

/// Drop tier of a hero.  Determines base-stat budget at mint.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Rarity {
    /// Most common drop.
    Common = 0,
    /// Above common; modest stat boost.
    Uncommon = 1,
    /// Mid-tier drop.
    Rare = 2,
    /// Difficult to obtain.
    Epic = 3,
    /// Top-tier general drop.
    Legendary = 4,
    /// Capped supply across the entire game.
    Mythic = 5,
}

impl Rarity {
    /// All six rarities, ordered from `Common` to `Mythic`.
    pub const ALL: [Rarity; 6] = [
        Rarity::Common,
        Rarity::Uncommon,
        Rarity::Rare,
        Rarity::Epic,
        Rarity::Legendary,
        Rarity::Mythic,
    ];
}

/// A combat unit.
///
/// Fields fall into two categories:
///
/// * **Intrinsic** — set at mint, immutable: [`Self::id`], [`Self::class`],
///   [`Self::rarity`], the base statistics, and [`Self::abilities`].
/// * **Earned** — accumulated through play: [`Self::veterancy`],
///   [`Self::xp`], and [`Self::equipped_items`].
///
/// The simulator never mutates a `Hero`; it copies the relevant fields
/// into ECS components at battle start.  Earned attributes are updated
/// via separate transactions in the indexer pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hero {
    /// Unique on-chain identifier.  Doubles as the deterministic
    /// tie-breaker for any equal-priority decision in combat.
    pub id: HeroId,

    /// Tactical class.  See [`HeroClass`] for matchup semantics.
    pub class: HeroClass,

    /// Drop tier.
    pub rarity: Rarity,

    /// Maximum and current HP at battle start.
    pub hp: Stat,

    /// Base attack power.
    pub atk: Stat,

    /// Base defense rating.
    pub def: Stat,

    /// Initiative score.  Higher = acts earlier.
    pub speed: Stat,

    /// Probability of a critical hit, expressed as a unit-interval
    /// fixed-point value.  Should normally lie in `[0, 1]`.
    pub crit_rate: Stat,

    /// Damage multiplier applied on a crit.  Typical values: 1.5–3.0.
    pub crit_damage: Stat,

    /// Ability slot identifiers, in activation priority order.
    pub abilities: Vec<AbilityId>,

    /// Battle-tier veterancy.  Distinct from [`Self::xp`]; advanced via
    /// reaching ruleset-defined XP thresholds.
    pub veterancy: u32,

    /// Cumulative experience.  Veterancy is a function of XP; both are
    /// stored explicitly to avoid recomputation across versions.
    pub xp: u64,

    /// Items equipped at the moment of roster commitment.
    pub equipped_items: Vec<ItemId>,
}

impl Hero {
    /// Canonical ordering key used to break ties in initiative
    /// resolution.
    ///
    /// The key sorts heroes in *act-first* order under
    /// `slice::sort_by_key`: higher speed acts first, lower hero id
    /// wins equal speeds.
    ///
    /// Uses the raw `Stat` (full I32F32 precision) rather than the
    /// integer truncation, so two heroes with speeds `10.4` and `10.6`
    /// sort distinctly. The previous implementation collapsed sub-
    /// integer differences and is forbidden by the determinism
    /// checklist.
    #[inline]
    pub fn initiative_key(&self) -> (Reverse<Stat>, HeroId) {
        (Reverse(self.speed), self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hero(id: HeroId, speed: i64) -> Hero {
        Hero {
            id,
            class: HeroClass::Infantry,
            rarity: Rarity::Common,
            hp: Stat::from_num(100),
            atk: Stat::from_num(20),
            def: Stat::from_num(10),
            speed: Stat::from_num(speed),
            crit_rate: Stat::from_num(0),
            crit_damage: Stat::from_num(1),
            abilities: vec![],
            veterancy: 0,
            xp: 0,
            equipped_items: vec![],
        }
    }

    #[test]
    fn initiative_key_orders_higher_speed_first() {
        let mut heroes = [hero(1, 5), hero(2, 10), hero(3, 7)];
        heroes.sort_by_key(|h| h.initiative_key());
        let ids: Vec<_> = heroes.iter().map(|h| h.id).collect();
        assert_eq!(ids, vec![2, 3, 1]);
    }

    #[test]
    fn initiative_key_breaks_ties_by_lower_id() {
        let mut heroes = [hero(99, 5), hero(7, 5), hero(42, 5)];
        heroes.sort_by_key(|h| h.initiative_key());
        let ids: Vec<_> = heroes.iter().map(|h| h.id).collect();
        assert_eq!(ids, vec![7, 42, 99]);
    }

    #[test]
    fn class_count_matches_all_array() {
        assert_eq!(HeroClass::COUNT, HeroClass::ALL.len());
    }

    #[test]
    fn rarity_all_is_sorted_ascending() {
        let mut sorted = Rarity::ALL;
        sorted.sort();
        assert_eq!(sorted, Rarity::ALL);
    }

    #[test]
    fn hero_round_trips_through_bincode() {
        let h = hero(1234, 42);
        let bytes = bincode::serialize(&h).unwrap();
        let back: Hero = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.id, h.id);
        assert_eq!(back.speed, h.speed);
    }
}
