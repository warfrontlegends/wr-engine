//! Status effects applied to heroes during combat.

use serde::{Deserialize, Serialize};

/// Transient combat condition tracked per hero.
///
/// Effect semantics live in the active `Ruleset` (crate `wr-ruleset`):
/// damage-over-time magnitudes, duration, stacking rules, and
/// resistance interactions are all ruleset-versioned, so changing the
/// numbers does not require touching this enum.
///
/// `wr-core` only owns the *identity* of each effect — the discriminant
/// is part of the on-chain log and must remain stable forever.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StatusEffect {
    /// Damage over time, fire-typed.
    Burning = 0,
    /// Damage over time, poison-typed; ignores armor.
    Poisoned = 1,
    /// Reduces effective speed.
    Frozen = 2,
    /// Skips the affected hero's next action.
    Stunned = 3,
    /// Absorbs incoming damage up to a ruleset-defined cap.
    Shielded = 4,
    /// Multiplies outgoing damage by a ruleset-defined factor.
    Enraged = 5,
    /// Reflects a fraction of incoming damage back to the attacker.
    Thorns = 6,
    /// Restores HP at end of turn.
    Regenerating = 7,
}

impl StatusEffect {
    /// Stable list of every status effect, in discriminant order.
    pub const ALL: [StatusEffect; 8] = [
        StatusEffect::Burning,
        StatusEffect::Poisoned,
        StatusEffect::Frozen,
        StatusEffect::Stunned,
        StatusEffect::Shielded,
        StatusEffect::Enraged,
        StatusEffect::Thorns,
        StatusEffect::Regenerating,
    ];

    /// Whether this effect deals damage over time.  Used by the
    /// simulator to short-circuit unnecessary work in DoT systems.
    #[inline]
    pub const fn is_damage_over_time(self) -> bool {
        matches!(self, StatusEffect::Burning | StatusEffect::Poisoned)
    }

    /// Whether this effect prevents the affected hero from acting.
    #[inline]
    pub const fn is_disabling(self) -> bool {
        matches!(self, StatusEffect::Stunned | StatusEffect::Frozen)
    }

    /// Whether this effect grants the hero a positive value.
    #[inline]
    pub const fn is_buff(self) -> bool {
        matches!(
            self,
            StatusEffect::Shielded | StatusEffect::Enraged
                | StatusEffect::Thorns   | StatusEffect::Regenerating
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_classification_is_consistent_with_buff_classification() {
        for eff in StatusEffect::ALL {
            // Nothing can be both a DoT and a buff.
            assert!(!(eff.is_damage_over_time() && eff.is_buff()));
        }
    }

    #[test]
    fn discriminants_are_stable() {
        // These literals appear in on-chain logs; changing them is a
        // breaking change for downstream consumers.  Lock them here.
        assert_eq!(StatusEffect::Burning as u8, 0);
        assert_eq!(StatusEffect::Poisoned as u8, 1);
        assert_eq!(StatusEffect::Frozen as u8, 2);
        assert_eq!(StatusEffect::Stunned as u8, 3);
        assert_eq!(StatusEffect::Shielded as u8, 4);
        assert_eq!(StatusEffect::Enraged as u8, 5);
        assert_eq!(StatusEffect::Thorns as u8, 6);
        assert_eq!(StatusEffect::Regenerating as u8, 7);
    }

    #[test]
    fn all_array_lists_each_effect_exactly_once() {
        let mut seen = std::collections::BTreeSet::new();
        for eff in StatusEffect::ALL {
            assert!(seen.insert(eff as u8), "duplicate in ALL: {eff:?}");
        }
        assert_eq!(seen.len(), StatusEffect::ALL.len());
    }
}
