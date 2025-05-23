module daemon::monster;

use daemon::{
    admin_cap::{PartialCap, MonsterPerm, MonsterMinterPerm},
    encrypted_drive::EncryptedDrive,
    versioning::{Version, version},
    palette::{MiniPalette, generate_palette},
    monster_part::{MonsterPart, MonsterPartTemplate},
    monster_part_registry::MonsterPartRegistry,
    distribution_table::DistributionTable
};
use sui::{random::Random, package, display, event};

const VERSION: u16 = 1;

const EMintingDisabled: u64 = 0;

///// EVENTS /////
public struct MintedMonster has copy, drop {
    id: ID,
    version: Version,
    generated_on: Version,
    palette: MiniPalette,
    parts: vector<MonsterPart>,
}

public struct MinterEnabled has copy, drop {
    enabled: bool
}

/// OTW
public struct MONSTER has drop {}

fun init(otw: MONSTER, ctx: &mut TxContext) {
    let minter = monster_minter(ctx);

    // Transfer the forge object to the module/package publisher
    transfer::share_object(minter);

    let keys = vector[
        b"id".to_string(),
        b"version".to_string(),
        b"generated_on".to_string(),
        b"parts".to_string(),
        b"palette".to_string(),
        b"description".to_string(),
        b"project_url".to_string(),
    ];

    let values = vector[
        b"{id}".to_string(),
        b"{version}".to_string(),
        b"{generated_on}".to_string(),
        b"{parts}".to_string(),
        b"{palette}".to_string(),
        b"A daemon monster.".to_string(),
        b"daemon.computer".to_string()
    ];

    let publisher = package::claim(otw, ctx);

    let mut display = display::new_with_fields<Monster>(
        &publisher, keys, values, ctx
    );

    display.update_version();

    transfer::public_transfer(publisher, ctx.sender());
    transfer::public_transfer(display, ctx.sender());
}

////// Monster ////////

public struct Monster has key, store {
    id: UID,
    version: Version,
    generated_on: Version,
    palette: MiniPalette,
    parts: vector<MonsterPart>,
}

/// Just in case we want to custom generate a monster
public fun monster(
    _: &PartialCap<MonsterPerm>,
    palette: MiniPalette,
    parts: vector<MonsterPart>,
    ctx: &mut TxContext
): Monster {
    mint_monster(palette, parts, ctx)
}

public fun assert_version(self: &Monster) {
    self.version.assert_updated(VERSION);
}

fun mint_monster(
    palette: MiniPalette,
    parts: vector<MonsterPart>,
    ctx: &mut TxContext
): Monster {
    let ver = version(VERSION);

    let monster = Monster {
        id: object::new(ctx),
        version: ver,
        generated_on: ver,
        palette,
        parts
    };

    event::emit(MintedMonster {
        id: object::id(&monster),
        version: ver,
        generated_on: ver,
        palette,
        parts,
    });

    monster
}

fun monster_random_generation(
    registry: &MonsterPartRegistry,
    distribution_table: &DistributionTable,
    random: &Random,
    ctx: &mut TxContext
): Monster {
    let mut generator = random.new_generator(ctx);
    let palette = generate_palette(&mut generator);

    let mut generator = random.new_generator(ctx);
    let body = registry.borrow_random(0, distribution_table, &mut generator);
    let mut parts = vector::empty();

    generate_part(body, registry, &mut parts, distribution_table, random, ctx);

    mint_monster(palette, parts, ctx)
}

fun generate_part(
    part: &MonsterPartTemplate,
    registry: &MonsterPartRegistry,
    parts: &mut vector<MonsterPart>,
    distribution_table: &DistributionTable,
    random: &Random,
    ctx: &mut TxContext
) {
    let mut generator = random.new_generator(ctx);
    let generated_part = part.generate(&mut generator);
    parts.push_back(generated_part);

    let required_parts = part.parts();
    let mut i = 0;
    while (i < required_parts.length()) {
        let part_type = required_parts.borrow(i);
        let required_part = registry.borrow_random(*part_type, distribution_table, &mut generator);

        generate_part(required_part, registry, parts, distribution_table, random, ctx);
        i = i + 1;
    };
}

// ////// Monster Minter //////

public struct MonsterMinter has key {
    id: UID,
    enabled: bool,
}

fun monster_minter(ctx: &mut TxContext): MonsterMinter {
    MonsterMinter {
        id: object::new(ctx),
        enabled: false,
    }
}

public fun is_enabled(self: &MonsterMinter): bool {
    self.enabled
}

public fun set_enabled(self: &mut MonsterMinter, _: &PartialCap<MonsterMinterPerm>, enabled: bool) {
    event::emit(MinterEnabled { enabled });

    self.enabled = enabled;
}

entry fun generate(
    self: &MonsterMinter,
    drive: EncryptedDrive,
    registry: &MonsterPartRegistry,
    distribution_table: &DistributionTable,
    random: &Random,
    ctx: &mut TxContext
) {
    assert!(self.is_enabled(), EMintingDisabled);

    // Only accept updated drives
    drive.assert_version();
    drive.burn();

    let monster = monster_random_generation(registry, distribution_table, random, ctx);

    transfer::public_transfer(monster, ctx.sender());
}

#[test_only]
use sui::test_scenario;
#[test_only]
use std::string::{String, utf8};
#[test_only]
use daemon::{
    monster_part::{test_template, monster_part_param},
    encrypted_drive::testing_encrypted_drive,
    admin_cap::{Self, testing_partial_cap, MonsterPartRegistryPerm},
    monster_part_registry,
    rarity::rarities,
    distribution_table::quick_test_distribution_table
};

#[test_only]
public fun setup(admin: address, scenario: &mut test_scenario::Scenario) {
    scenario.next_tx(admin);
    {
        let minter = monster_minter(scenario.ctx());
        quick_test_distribution_table(scenario.ctx());
        transfer::share_object(minter);
    };
}

#[test_only]
fun init_templates(admin: address, part: u16, parts: vector<u16>, scenario: &mut test_scenario::Scenario) {
    scenario.next_tx(admin);
    {
        let mut registry = test_scenario::take_shared<MonsterPartRegistry>(scenario);

        let mut params = vector::empty();
        let mut i = 0;
        while (i < 5) {
            params.push_back(monster_part_param(0, 10));
            i = i + 1;
        };

        let cap = testing_partial_cap<MonsterPartRegistryPerm>();

        let mut i = 0;
        while (i < rarities()) {
            let mut buffer = vector::empty();
            buffer.push_back(part as u8);
            buffer.push_back(i);

            let name = utf8(buffer);
            let template = test_template(name, part, i, params, parts);

            registry.register(&cap, template);

            i = i + 1;
        };

        test_scenario::return_shared(registry);
    };
}

#[test_only]
fun init_part_type(admin: address, name: vector<u8>, scenario: &mut test_scenario::Scenario): u16 {
    let mut id = 0;

    scenario.next_tx(admin);
    {
        let mut registry = test_scenario::take_shared<MonsterPartRegistry>(scenario);

        let cap = testing_partial_cap<MonsterPartRegistryPerm>();

        id = registry.register_part_type(cap, utf8(name));

        test_scenario::return_shared(registry);
    };
    id
}

#[test]
#[expected_failure(abort_code = EMintingDisabled)]
fun minting_disabled() {
    let admin = @0x01;

    // Chain setup
    let mut scenario = test_scenario::begin(admin);
    admin_cap::setup(admin, &mut scenario);
    monster_part_registry::setup(admin, &mut scenario);
    setup(admin, &mut scenario);

    let user = @0x02;

    scenario.next_tx(user);
    {
        let registry = test_scenario::take_shared<MonsterPartRegistry>(&scenario);
        let distrib = scenario.take_shared<DistributionTable>();
        let minter = test_scenario::take_shared<MonsterMinter>(&scenario);
        let random = test_scenario::take_shared<Random>(&scenario);
        let drive = testing_encrypted_drive(scenario.ctx());

        minter.generate(drive, &registry, &distrib, &random, scenario.ctx());

        test_scenario::return_shared(random);
        test_scenario::return_shared(distrib);
        test_scenario::return_shared(minter);
        test_scenario::return_shared(registry);
    };

    scenario.end();
}

#[test]
fun test_mint() {
    let admin = @0x01;

    // Chain setup
    let mut scenario = test_scenario::begin(admin);
    admin_cap::setup(admin, &mut scenario);
    monster_part_registry::setup(admin, &mut scenario);
    setup(admin, &mut scenario);

    // Setup registry

    let torso = init_part_type(admin, b"torso", &mut scenario);
    let head = init_part_type(admin, b"head", &mut scenario);
    let tail = init_part_type(admin, b"tail", &mut scenario);
    let limb = init_part_type(admin, b"limb", &mut scenario);
    let eye = init_part_type(admin, b"eye", &mut scenario);

    // Torso
    let mut torso_parts = vector::empty();
    torso_parts.push_back(head);
    torso_parts.push_back(tail);
    torso_parts.push_back(limb);
    init_templates(admin, torso, torso_parts, &mut scenario);

    // Head
    let mut head_parts = vector::empty();
    head_parts.push_back(eye);
    init_templates(admin, head, head_parts, &mut scenario);

    // Tail
    init_templates(admin, tail, vector::empty(), &mut scenario);

    // Limbs
    init_templates(admin, limb, vector::empty(), &mut scenario);

    // Eyes
    init_templates(admin, eye, vector::empty(), &mut scenario);

    let user = @0x02;

    scenario.next_tx(user);
    {
        let registry = test_scenario::take_shared<MonsterPartRegistry>(&scenario);
        let distrib = scenario.take_shared<DistributionTable>();
        let mut minter = test_scenario::take_shared<MonsterMinter>(&scenario);
        let random = test_scenario::take_shared<Random>(&scenario);
        let drive = testing_encrypted_drive(scenario.ctx());

        let cap = testing_partial_cap<MonsterMinterPerm>();
        minter.set_enabled(&cap, true);
        minter.generate(drive, &registry, &distrib, &random, scenario.ctx());

        test_scenario::return_shared(random);
        test_scenario::return_shared(distrib);
        test_scenario::return_shared(minter);
        test_scenario::return_shared(registry);
    };

    scenario.next_tx(user);
    {
        let monster = test_scenario::take_from_sender<Monster>(&scenario);
        assert!(monster.parts.length() == 5, 0);
        test_scenario::return_to_sender(&scenario, monster);
    };

    scenario.end();
}
