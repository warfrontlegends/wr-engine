//! Terrain modifiers attached to a battle.
//!
//! A territory may have a terrain class (forest, ridge, urban, …); each
//! class translates into a [`TerrainModifier`] that adjusts the base
//! statistics of every combatant for the duration of the battle.

use serde::{Deserialize, Serialize};

use crate::stat::{Stat, STAT_ZERO};

/// Additive adjustments applied to combatants' base statistics.
///
/// All fields default to [`STAT_ZERO`], which is the no-op.  The
/// simulator adds these directly to a hero's effective stat after
/// reading components, so a positive value buffs and a negative value
/// debuffs.  The ruleset bounds each modifier to a sane range; values
/// outside that range cause the simulator to refuse to start.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerrainModifier {
    /// Added to every hero's [`crate::Hero::atk`].
    pub atk_mod: Stat,
    /// Added to every hero's [`crate::Hero::def`].
    pub def_mod: Stat,
    /// Added to every hero's [`crate::Hero::speed`].
    pub speed_mod: Stat,
}

impl TerrainModifier {
    /// The neutral modifier — applies no change to any stat.
    pub const NEUTRAL: TerrainModifier = TerrainModifier {
        atk_mod: STAT_ZERO,
        def_mod: STAT_ZERO,
        speed_mod: STAT_ZERO,
    };

    /// Whether this modifier is a no-op.  Cheap fast-path check the
    /// simulator uses to skip per-hero stat adjustment.
    #[inline]
    pub fn is_neutral(&self) -> bool {
        *self == Self::NEUTRAL
    }
}

impl Default for TerrainModifier {
    #[inline]
    fn default() -> Self {
        Self::NEUTRAL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_modifier_is_default() {
        assert_eq!(TerrainModifier::default(), TerrainModifier::NEUTRAL);
    }

    #[test]
    fn neutral_modifier_is_detected() {
        assert!(TerrainModifier::NEUTRAL.is_neutral());
        let buff = TerrainModifier {
            atk_mod: Stat::from_num(5),
            ..Default::default()
        };
        assert!(!buff.is_neutral());
    }

    #[test]
    fn modifier_round_trips_through_bincode() {
        let m = TerrainModifier {
            atk_mod: Stat::from_num(3),
            def_mod: Stat::from_num(-2),
            speed_mod: Stat::from_num(1),
        };
        let bytes = bincode::serialize(&m).unwrap();
        let back: TerrainModifier = bincode::deserialize(&bytes).unwrap();
        assert_eq!(m, back);
    }
}
