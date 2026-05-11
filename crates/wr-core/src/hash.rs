//! Canonical hashing helpers.
//!
//! Every hash that the simulator stores or compares passes through one
//! of the functions in this module.  The behavior is fixed by three
//! choices:
//!
//! 1. **Hash function**: Blake3.  Fast, well-audited, no extension
//!    attacks, deterministic across architectures.
//! 2. **Encoding**: `bincode::serialize`, which on a `Serialize` value
//!    with derived implementation yields a fixed-int little-endian
//!    encoding with no length-delimiter ambiguity.
//! 3. **Domain separation**: every hashed payload is prefixed with a
//!    fixed-length tag so that hashes computed for distinct purposes
//!    cannot collide even if the payloads happen to coincide.
//!
//! Callers should always use the constants in this module as tags
//! (e.g. [`DOMAIN_BATTLE_OUTPUT`]) rather than ad-hoc strings, so the
//! set of in-use tags is grep-able from one place.

use blake3::Hasher;
use serde::Serialize;

/// Domain tag for hashing a finalized [`crate::BattleOutput::events`]
/// log into [`crate::BattleOutput::output_hash`].
pub const DOMAIN_BATTLE_OUTPUT: &[u8] = b"WFL-BATTLE-OUTPUT-V1";

/// Domain tag for hashing the per-round aggregate of every
/// [`crate::BattleOutput::output_hash`] published in that round.
pub const DOMAIN_ROUND_ROOT: &[u8] = b"WFL-ROUND-ROOT-V1";

/// Domain tag for hashing a roster's canonical encoding into the
/// commitment value submitted on-chain.
pub const DOMAIN_ROSTER_COMMITMENT: &[u8] = b"WFL-ROSTER-COMMIT-V1";

/// Hash any `Serialize` value with the given domain tag.
///
/// Layout: `Blake3(tag || bincode::serialize(value))`.
///
/// # Errors
///
/// Returns the underlying `bincode` error if `value` cannot be
/// serialized.  In practice this never fails for derived `Serialize`
/// implementations on `wr-core` types; a failure here indicates a
/// caller passing in something unusual.
pub fn canonical_hash_with_tag<T: Serialize + ?Sized>(
    tag: &[u8],
    value: &T,
) -> Result<[u8; 32], bincode::Error> {
    let bytes = bincode::serialize(value)?;
    let mut h = Hasher::new();
    h.update(tag);
    h.update(&bytes);
    Ok(h.finalize().into())
}

/// Hash a battle's event log using [`DOMAIN_BATTLE_OUTPUT`].
///
/// This is the value the simulator writes into
/// [`crate::BattleOutput::output_hash`] and that verifiers compare.
///
/// # Panics
///
/// Panics if `bincode` fails to serialize the slice.  The derived
/// `Serialize` implementations on [`crate::BattleEvent`] cannot fail,
/// so this is unreachable in practice.
pub fn canonical_hash(events: &[crate::BattleEvent]) -> [u8; 32] {
    canonical_hash_with_tag(DOMAIN_BATTLE_OUTPUT, events)
        .expect("BattleEvent serialization is infallible")
}

/// Aggregate per-battle hashes into a single round root.
///
/// The aggregator sorts inputs ascending so the result is independent
/// of the order in which battles were resolved.  This matches the
/// merkle-equivalent behavior on the chain side: re-derivers can submit
/// hashes in any order.
pub fn aggregate_round_root(per_battle_hashes: &[[u8; 32]]) -> [u8; 32] {
    let mut sorted: Vec<[u8; 32]> = per_battle_hashes.to_vec();
    sorted.sort();
    let mut h = Hasher::new();
    h.update(DOMAIN_ROUND_ROOT);
    h.update(&(sorted.len() as u64).to_le_bytes());
    for hash in &sorted {
        h.update(hash);
    }
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::BattleEvent;
    use crate::roster::Side;
    use crate::stat::Stat;

    #[test]
    fn canonical_hash_is_deterministic() {
        let log = vec![
            BattleEvent::BattleStart { roster_a_size: 2, roster_b_size: 2 },
            BattleEvent::TurnStart { tick: 0 },
            BattleEvent::Attack {
                tick: 0,
                src: 1,
                dst: 2,
                damage: Stat::from_num(10),
                is_crit: false,
            },
            BattleEvent::BattleEnd { tick: 1, winner: Side::A },
        ];
        let h1 = canonical_hash(&log);
        let h2 = canonical_hash(&log);
        assert_eq!(h1, h2);
    }

    #[test]
    fn canonical_hash_differs_under_event_reorder() {
        let a = vec![
            BattleEvent::TurnStart { tick: 0 },
            BattleEvent::TurnStart { tick: 1 },
        ];
        let b = vec![
            BattleEvent::TurnStart { tick: 1 },
            BattleEvent::TurnStart { tick: 0 },
        ];
        assert_ne!(canonical_hash(&a), canonical_hash(&b));
    }

    #[test]
    fn domain_separation_changes_the_hash() {
        let payload = b"same bytes either way";
        let h1 = canonical_hash_with_tag(b"DOMAIN-A", payload).unwrap();
        let h2 = canonical_hash_with_tag(b"DOMAIN-B", payload).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn round_root_is_independent_of_input_order() {
        let mut hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let r1 = aggregate_round_root(&hashes);
        hashes.reverse();
        let r2 = aggregate_round_root(&hashes);
        assert_eq!(r1, r2);
    }

    #[test]
    fn round_root_includes_count_in_preimage() {
        // Two distinct hash multisets that happen to share the same XOR
        // would otherwise collide; the count-prefix prevents that for
        // length-distinct inputs.
        let one = aggregate_round_root(&[[7u8; 32]]);
        let two = aggregate_round_root(&[[7u8; 32], [7u8; 32]]);
        assert_ne!(one, two);
    }
}
