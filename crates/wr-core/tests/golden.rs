//! Cross-architecture byte-stability golden tests.
//!
//! These tests pin the exact wire format of every type that crosses the
//! simulator boundary. A change to `bincode`, `serde`, `fixed`, or the
//! field order of any locked struct will cause one of these tests to
//! fail — alerting the author that they have made a change which would
//! desync independent verifiers from the operator's published hashes.
//!
//! Run on x86_64, aarch64, and wasm32 as part of CI to confirm the
//! "byte-identical across architectures" contract.

use wr_core::{
    aggregate_round_root, canonical_hash, BattleEvent, BattleInput, Hero, HeroClass, Rarity,
    Roster, Side, Stat, StatusEffect, XpGrant,
};

fn hx(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn fixed_hero(id: u64) -> Hero {
    Hero {
        id,
        class: HeroClass::Infantry,
        rarity: Rarity::Common,
        hp: Stat::from_num(100),
        atk: Stat::from_num(20),
        def: Stat::from_num(10),
        speed: Stat::from_num(8),
        crit_rate: Stat::from_num(0),
        crit_damage: Stat::from_num(1),
        abilities: vec![1, 2, 3],
        veterancy: 0,
        xp: 0,
        equipped_items: vec![100, 200],
    }
}

fn fixed_roster(owner_marker: u8, ids: &[u64]) -> Roster {
    Roster {
        owner: [owner_marker; 20],
        heroes: ids.iter().copied().map(fixed_hero).collect(),
        commitment_hash: [owner_marker; 32],
    }
}

// ---------------------------------------------------------------------
//   Stat / Side / StatusEffect — primitives
// ---------------------------------------------------------------------

#[test]
fn golden_stat_from_num_42() {
    let v = Stat::from_num(42);
    let bytes = bincode::serialize(&v).unwrap();
    // I32F32 = 64-bit two's complement, 32 fractional bits.
    // 42 << 32 = 0x0000_002a_0000_0000, little-endian.
    assert_eq!(hx(&bytes), "000000002a000000");
}

#[test]
fn golden_side_a_encoding() {
    let bytes = bincode::serialize(&Side::A).unwrap();
    assert_eq!(hx(&bytes), "00000000");
}

#[test]
fn golden_side_b_encoding() {
    let bytes = bincode::serialize(&Side::B).unwrap();
    assert_eq!(hx(&bytes), "01000000");
}

#[test]
fn golden_status_effect_encodings() {
    for (ix, eff) in StatusEffect::ALL.iter().enumerate() {
        let bytes = bincode::serialize(eff).unwrap();
        let want = format!("{:02x}000000", ix);
        assert_eq!(hx(&bytes), want, "{:?}", eff);
    }
}

// ---------------------------------------------------------------------
//   Hero / Roster — composite structs
// ---------------------------------------------------------------------

#[test]
fn golden_hero_serialization() {
    let h = fixed_hero(7);
    let bytes = bincode::serialize(&h).unwrap();
    // Locks: id (8) + class (4) + rarity (4) + six Stat (48) + abilities
    // [len(8) + 3*u32(12)] + veterancy (4) + xp (8) + equipped_items
    // [len(8) + 2*u32(8)] = 116 bytes.
    assert_eq!(
        hx(&bytes),
        "0700000000000000\
         00000000\
         00000000\
         0000000064000000\
         0000000014000000\
         000000000a000000\
         0000000008000000\
         0000000000000000\
         0000000001000000\
         0300000000000000\
         010000000200000003000000\
         00000000\
         0000000000000000\
         0200000000000000\
         64000000c8000000"
    );
}

#[test]
fn golden_roster_serialization_length_and_hash() {
    let r = fixed_roster(0xAB, &[1, 2]);
    let bytes = bincode::serialize(&r).unwrap();
    // owner (20) + heroes_len (8) + 2 * hero (112) + commitment_hash (32)
    //   = 284 bytes.
    assert_eq!(bytes.len(), 284);
    // Lock the full byte stream via Blake3 — any field reorder, size
    // change, or endianness flip alters this digest.
    let h: [u8; 32] = blake3::hash(&bytes).into();
    assert_eq!(
        hx(&h),
        "087b00ce05d72271461e5cb373c30feccd315b68a2198f13142033aadd13aa5d"
    );
}

// ---------------------------------------------------------------------
//   BattleEvent variant indices — all 9
// ---------------------------------------------------------------------

#[test]
fn golden_battle_event_variant_indices() {
    let cases: [(BattleEvent, u32); 9] = [
        (BattleEvent::BattleStart { roster_a_size: 0, roster_b_size: 0 }, 0),
        (BattleEvent::TurnStart { tick: 0 }, 1),
        (
            BattleEvent::Attack { tick: 0, src: 0, dst: 0, damage: Stat::from_num(0), is_crit: false },
            2,
        ),
        (BattleEvent::Heal { tick: 0, src: 0, dst: 0, amount: Stat::from_num(0) }, 3),
        (BattleEvent::Ability { tick: 0, src: 0, ability: 0, targets: vec![] }, 4),
        (
            BattleEvent::StatusApplied {
                tick: 0,
                hero: 0,
                status: StatusEffect::Burning,
                duration: 0,
            },
            5,
        ),
        (
            BattleEvent::StatusExpired { tick: 0, hero: 0, status: StatusEffect::Burning },
            6,
        ),
        (BattleEvent::Death { tick: 0, hero: 0, killer: None }, 7),
        (BattleEvent::BattleEnd { tick: 0, winner: Side::A }, 8),
    ];
    for (ev, want_ix) in cases {
        let bytes = bincode::serialize(&ev).unwrap();
        let got_ix = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        assert_eq!(got_ix, want_ix, "{ev:?}");
    }
}

// ---------------------------------------------------------------------
//   canonical_hash & aggregate_round_root — the hashes verifiers compare
// ---------------------------------------------------------------------

#[test]
fn golden_canonical_hash_of_minimal_log() {
    let log = vec![
        BattleEvent::BattleStart { roster_a_size: 1, roster_b_size: 1 },
        BattleEvent::TurnStart { tick: 0 },
        BattleEvent::Attack {
            tick: 0,
            src: 1,
            dst: 2,
            damage: Stat::from_num(10),
            is_crit: false,
        },
        BattleEvent::Death { tick: 0, hero: 2, killer: Some(1) },
        BattleEvent::BattleEnd { tick: 1, winner: Side::A },
    ];
    let h = canonical_hash(&log);
    assert_eq!(
        hx(&h),
        "9fc8848ad3f326ba7ae10a645a67cf770790f65ae6101d3c631ffbc47a80af0f"
    );
}

#[test]
fn golden_aggregate_round_root_of_three_hashes() {
    let h = aggregate_round_root(&[[1u8; 32], [2u8; 32], [3u8; 32]]);
    assert_eq!(
        hx(&h),
        "61f589e9ff209a9d3bb2222fe7068a77be7708a20d9ac2ad4a7e6fe2a9706c08"
    );
}

#[test]
fn golden_xp_grant_serialization() {
    let g = XpGrant { hero: 0xDEADBEEF, amount: 1000, veterancy_delta: 1 };
    let bytes = bincode::serialize(&g).unwrap();
    assert_eq!(hx(&bytes), "efbeadde00000000e80300000000000001000000");
}

// ---------------------------------------------------------------------
//   BattleInput — the full simulator-input struct
// ---------------------------------------------------------------------

#[test]
fn golden_battle_input_hash() {
    // Hash the bincode encoding of a fixed BattleInput. We don't lock
    // the full byte stream (it's long), but locking the digest is
    // sufficient to detect any change.
    let input = BattleInput {
        round: 42,
        roster_a: fixed_roster(0xAA, &[1, 2]),
        roster_b: fixed_roster(0xBB, &[3, 4]),
        seed: [0x55; 32],
        ruleset_version: 1,
        terrain: None,
    };
    let bytes = bincode::serialize(&input).unwrap();
    let h: [u8; 32] = blake3::hash(&bytes).into();
    assert_eq!(
        hx(&h),
        "77ef866a24c00f56f2b3548da410a3e045d311d829c34014c5c0c939c87fe3b6"
    );
}
