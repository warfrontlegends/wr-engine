//! Roster: an ordered set of heroes a player commits to a battle.

use serde::{Deserialize, Serialize};

use crate::hero::Hero;
use crate::ids::WalletAddress;

/// Maximum number of heroes a roster may contain.  Enforced both
/// off-chain (here, at construction) and on-chain (commitment contract).
pub const ROSTER_MAX_SIZE: usize = 8;

/// Minimum number of heroes a roster must contain to be considered
/// well-formed.
pub const ROSTER_MIN_SIZE: usize = 1;

/// A player-owned ordered list of heroes deployed for a battle.
///
/// Order is meaningful: it determines deployment slot, which the
/// ruleset consults for terrain assignment and adjacency-based ability
/// effects.  The chain stores only [`Self::commitment_hash`]; the full
/// roster is revealed at battle resolution time and validated against
/// that hash by the simulator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Roster {
    /// Wallet that owns every hero in [`Self::heroes`].
    pub owner: WalletAddress,
    /// Heroes in deployment order.
    pub heroes: Vec<Hero>,
    /// Hash committed on-chain at roster submission.  See
    /// `wr-prng::roster_commitment` for the canonical encoding.
    pub commitment_hash: [u8; 32],
}

/// Validation error for a malformed [`Roster`].
#[derive(Debug, thiserror::Error)]
pub enum RosterError {
    /// The roster contains fewer than [`ROSTER_MIN_SIZE`] heroes.
    #[error("roster too small: {found} heroes, minimum is {min}")]
    TooSmall {
        /// Number of heroes the roster actually contained.
        found: usize,
        /// Required minimum size — equal to [`ROSTER_MIN_SIZE`].
        min: usize,
    },

    /// The roster contains more than [`ROSTER_MAX_SIZE`] heroes.
    #[error("roster too large: {found} heroes, maximum is {max}")]
    TooLarge {
        /// Number of heroes the roster actually contained.
        found: usize,
        /// Allowed maximum size — equal to [`ROSTER_MAX_SIZE`].
        max: usize,
    },

    /// Two heroes in the roster share an id.  Forbidden because
    /// initiative tie-breaking assumes ids are unique within a battle.
    #[error("duplicate hero id {0} in roster")]
    DuplicateHero(u64),
}

impl Roster {
    /// Validate the roster's structural invariants.
    ///
    /// Cheap to call: O(n log n) due to a sort-and-scan over hero ids.
    /// Callers should run this once at intake; the simulator assumes
    /// validity thereafter.
    pub fn validate(&self) -> Result<(), RosterError> {
        let n = self.heroes.len();
        if n < ROSTER_MIN_SIZE {
            return Err(RosterError::TooSmall { found: n, min: ROSTER_MIN_SIZE });
        }
        if n > ROSTER_MAX_SIZE {
            return Err(RosterError::TooLarge { found: n, max: ROSTER_MAX_SIZE });
        }
        let mut ids: Vec<_> = self.heroes.iter().map(|h| h.id).collect();
        ids.sort_unstable();
        for w in ids.windows(2) {
            if w[0] == w[1] {
                return Err(RosterError::DuplicateHero(w[0]));
            }
        }
        Ok(())
    }
}

/// Which side of a battle a unit fights for.
///
/// `Side` is preserved on the wire as a `u8` discriminant so the
/// representation matches the on-chain combat log.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Side {
    /// Roster A — the challenger in territorial combat.
    A = 0,
    /// Roster B — the defender in territorial combat.
    B = 1,
}

impl Side {
    /// The opposing side.
    #[inline]
    #[must_use]
    pub const fn opposite(self) -> Side {
        match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hero::{HeroClass, Rarity};
    use crate::stat::Stat;

    fn dummy_hero(id: u64) -> Hero {
        Hero {
            id,
            class: HeroClass::Infantry,
            rarity: Rarity::Common,
            hp: Stat::from_num(100),
            atk: Stat::from_num(10),
            def: Stat::from_num(10),
            speed: Stat::from_num(10),
            crit_rate: Stat::from_num(0),
            crit_damage: Stat::from_num(1),
            abilities: vec![],
            veterancy: 0,
            xp: 0,
            equipped_items: vec![],
        }
    }

    fn roster_of(ids: &[u64]) -> Roster {
        Roster {
            owner: [0u8; 20],
            heroes: ids.iter().copied().map(dummy_hero).collect(),
            commitment_hash: [0u8; 32],
        }
    }

    #[test]
    fn opposite_is_an_involution() {
        assert_eq!(Side::A.opposite(), Side::B);
        assert_eq!(Side::B.opposite(), Side::A);
        assert_eq!(Side::A.opposite().opposite(), Side::A);
    }

    #[test]
    fn validate_accepts_well_formed_roster() {
        let r = roster_of(&[1, 2, 3, 4]);
        r.validate().unwrap();
    }

    #[test]
    fn validate_rejects_empty_roster() {
        let r = roster_of(&[]);
        let err = r.validate().unwrap_err();
        assert!(matches!(err, RosterError::TooSmall { .. }));
    }

    #[test]
    fn validate_rejects_oversized_roster() {
        let r = roster_of(&(0..(ROSTER_MAX_SIZE as u64 + 1)).collect::<Vec<_>>());
        let err = r.validate().unwrap_err();
        assert!(matches!(err, RosterError::TooLarge { .. }));
    }

    #[test]
    fn validate_rejects_duplicate_hero_id() {
        let r = roster_of(&[1, 2, 3, 2]);
        let err = r.validate().unwrap_err();
        assert!(matches!(err, RosterError::DuplicateHero(2)));
    }

    #[test]
    fn side_variants_serialize_distinctly_and_equal_width() {
        // Side's exact wire format (4-byte LE u32) is locked in
        // `tests/golden.rs::golden_side_a_encoding` /
        // `golden_side_b_encoding`.  This test just guards the
        // invariants that motivated the type — distinct variants
        // produce distinct bytes of equal length.
        let bytes_a = bincode::serialize(&Side::A).unwrap();
        let bytes_b = bincode::serialize(&Side::B).unwrap();
        assert_eq!(bytes_a.len(), bytes_b.len());
        assert_ne!(bytes_a, bytes_b);
    }
}
