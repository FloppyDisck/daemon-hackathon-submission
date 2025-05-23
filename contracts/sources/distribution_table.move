module daemon::distribution_table;

use daemon::{
    rarity::{Self, Rarity},
    admin_cap::{PartialCap, DistributionTablePerm}
};
use sui::random::RandomGenerator;

const ERarityDistributionInvalid: u64 = 0;

fun init(ctx: &mut TxContext) {
    let table = DistributionTable {
        id: object::new(ctx),
        rarity: vector::empty(),
    };

    transfer::share_object(table);
}

public struct DistributionTable has key, store {
    id: UID,
    rarity: vector<u16>,
}

#[test_only]
public fun test_distribution_table(rarity: vector<u16>, ctx: &mut TxContext) {
    let distrib = DistributionTable {
        id: object::new(ctx),
        rarity,
    };

    transfer::share_object(distrib);
}

#[test_only]
public fun quick_test_distribution_table(ctx: &mut TxContext) {
    let mut rarity = vector::empty();
    rarity.push_back(10);

    let mut protocol = vector::empty();
    protocol.push_back(10);

    test_distribution_table(rarity, ctx)
}

public fun set_rarity_distribution(self: &mut DistributionTable, _: PartialCap<DistributionTablePerm>, rarity: vector<u16>) {
    assert!(rarity.length() <= rarity::rarities() as u64, ERarityDistributionInvalid);
    self.rarity = rarity;
}

public fun borrow_rarity_distribution(self: &DistributionTable): &vector<u16> {
    &self.rarity
}

fun rarity_count(self: &DistributionTable): u16 {
    let mut total = 0;
    let mut i = 0;
    while (i < vector::length(&self.rarity)) {
        total = total + self.rarity[i];
        i = i + 1;
    };
    total
}

public(package) fun pick_rarity(self: &DistributionTable, generator: &mut RandomGenerator): Rarity {
    let pick = generator.generate_u16_in_range(0, self.rarity_count());

    let mut total = 0;
    let mut i = 0;
    while (i < vector::length(&self.rarity)) {
        total = total + self.rarity[i];

        if (pick <= total) {
            return rarity::from_u8(i as u8)
        };

        i = i + 1;
    };
    rarity::from_u8(i as u8)
}
