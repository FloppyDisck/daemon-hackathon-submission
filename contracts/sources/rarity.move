module daemon::rarity;

public enum Rarity has drop, copy, store {
    Common,
    Uncommon,
    Rare,
    Legendary,
    Epic,
    Unique
}

/// Returns the total number of rarities.
public fun rarities(): u8 {
    6
}

/// Returns the u8 representation of the rarity.
public fun to_u8(t: Rarity): u8 {
    match (t) {
        Rarity::Common => 0,
        Rarity::Uncommon => 1,
        Rarity::Rare => 2,
        Rarity::Legendary => 3,
        Rarity::Epic => 4,
        Rarity::Unique => 5,
    }
}


/// Returns the rarity from the u8 representation.
public fun from_u8(t: u8): Rarity {
    if (t == 1) {
        Rarity::Uncommon
    } else if (t == 2) {
        Rarity::Rare
    } else if (t == 3) {
        Rarity::Legendary
    } else if (t == 4) {
        Rarity::Epic
    } else if (t == 5) {
        Rarity::Unique
    } else {
        Rarity::Common
    }
}

#[test_only]
use sui::test_utils::assert_eq;

#[test]
fun test_to_u8() {
    assert_eq(Rarity::Common.to_u8(), 0);
    assert_eq(Rarity::Uncommon.to_u8(), 1);
    assert_eq(Rarity::Rare.to_u8(), 2);
    assert_eq(Rarity::Legendary.to_u8(), 3);
    assert_eq(Rarity::Epic.to_u8(), 4);
    assert_eq(Rarity::Unique.to_u8(), 5);
}

#[test]
fun test_from_u8() {
    let mut i = 0;
    while (i < rarities()) {
        assert_eq(from_u8(i).to_u8(), i);
        i = i + 1;
    };
}
