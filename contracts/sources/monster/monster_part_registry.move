module daemon::monster_part_registry;

use daemon::{
    admin_cap::{PartialCap, MonsterPartRegistryPerm},
    monster_part::MonsterPartTemplate,
    rarity::Rarity,
    distribution_table::DistributionTable,
};

use sui::{
    table::{Self, Table},
    vec_set::{Self, VecSet},
    random::RandomGenerator,
    event
};

use std::string::String;

//// EVENTS /////
public struct AddedTemplate has copy, drop {
    template: MonsterPartTemplate
}

public struct DisabledTemplate has copy, drop {
    name: String
}

public struct EnabledTemplate has copy, drop {
    name: String
}

public struct MonsterPartRegistryKey has store, copy, drop {
    part_type: u16,
    rarity: Rarity
}

public fun key_from_template(template: &MonsterPartTemplate): MonsterPartRegistryKey {
    MonsterPartRegistryKey {
        part_type: template.part_type(),
        rarity: template.rarity()
    }
}

// TODO: make a OTW
public struct MonsterPartRegistry has key {
    id: UID,
    total_part_types: u16,
    part_types: Table<u16, String>,
    registry: Table<String, MonsterPartTemplate>,
    cache: Table<MonsterPartRegistryKey, VecSet<String>>
}

fun init(ctx: &mut TxContext) {
    let registry = monster_part_registry(ctx);

    // Transfer the forge object to the module/package publisher
    transfer::share_object(registry);
}

fun monster_part_registry(ctx: &mut TxContext): MonsterPartRegistry {
    MonsterPartRegistry {
        id: object::new(ctx),
        total_part_types: 0,
        part_types: table::new(ctx),
        registry: table::new(ctx),
        cache: table::new(ctx)
    }
}

public use fun monster_part_registry_register as MonsterPartRegistry.register;
public fun monster_part_registry_register(
    self: &mut MonsterPartRegistry,
    _: &PartialCap<MonsterPartRegistryPerm>,
    template: MonsterPartTemplate
) {
    event::emit(AddedTemplate { template });

    let key = key_from_template(&template);

    // If cache doesnt exist, then create it
    if (!self.cache.contains(key)) {
        self.cache.add(key, vec_set::empty());
    };

    let cache = self.cache.borrow_mut(key);
    cache.insert(template.name());

    self.registry.add(template.name(), template);
}

public use fun register_part_type as MonsterPartRegistry.register_part;
public fun register_part_type(self: &mut MonsterPartRegistry, _: PartialCap<MonsterPartRegistryPerm>, name: String): u16 {
    let id = self.total_part_types;
    self.total_part_types = id + 1;

    self.part_types.add(id, name);
    id
}

/// Remove the template from the active set
public fun unregister(self: &mut MonsterPartRegistry, _: PartialCap<MonsterPartRegistryPerm>, name: String) {
    event::emit(DisabledTemplate { name });

    let key = key_from_template(self.registry.borrow(name));
    let cache = self.cache.borrow_mut(key);
    cache.remove(&name);
}

/// Add a previously unregistered template
public fun reregister(self: &mut MonsterPartRegistry, _: PartialCap<MonsterPartRegistryPerm>, name: String) {
    event::emit(EnabledTemplate { name });

    let key = key_from_template(self.registry.borrow(name));
    let cache = self.cache.borrow_mut(key);
    cache.insert(name);
}

public fun borrow(self: &MonsterPartRegistry, name: String): &MonsterPartTemplate {
    self.registry.borrow(name)
}

public fun borrow_mut(self: &mut MonsterPartRegistry, name: String, _: PartialCap<MonsterPartRegistryPerm>): &MonsterPartTemplate {
    self.registry.borrow_mut(name)
}

public fun borrow_cache(self: &MonsterPartRegistry, key: MonsterPartRegistryKey): &VecSet<String> {
    self.cache.borrow(key)
}

public(package) fun borrow_random(
    self: &MonsterPartRegistry,
    part_type: u16,
    distribution_table: &DistributionTable,
    generator: &mut RandomGenerator
): &MonsterPartTemplate {
    let key = MonsterPartRegistryKey {
        part_type,
        rarity: distribution_table.pick_rarity(generator)
    };

    let templates = self.borrow_cache(key);
    let idx = generator.generate_u8_in_range(0, (templates.size() - 1) as u8);
    let name = templates.keys().borrow(idx as u64);

    self.borrow(*name)
}

// TODO: test unregister
// TODO: test reregister

#[test_only]
use sui::test_scenario;
#[test_only]
use std::string::utf8;
#[test_only]
use daemon::{
    monster_part::test_template,
    admin_cap::{Self, testing_partial_cap}
};

#[test_only]
public fun setup(admin: address, scenario: &mut test_scenario::Scenario) {
    // Chain setup
    scenario.next_tx(admin);
    {
        let registry = monster_part_registry(scenario.ctx());
        transfer::share_object(registry);
    };
}

#[test]
fun add_item() {
    let admin = @0x01;

    // Chain setup
    let mut scenario = test_scenario::begin(admin);
    admin_cap::setup(admin, &mut scenario);
    setup(admin, &mut scenario);

    scenario.next_tx(admin);
    {
        let mut registry = test_scenario::take_shared<MonsterPartRegistry>(&scenario);
        let cap = testing_partial_cap<MonsterPartRegistryPerm>();
        let template = test_template(utf8(b"test"), 0, 0, vector::empty(), vector::empty());
        registry.register(&cap, template);

        test_scenario::return_shared(registry);
    };

    scenario.end();
}

#[test]
fun get_from_key() {
    let admin = @0x01;

    // Chain setup
    let mut scenario = test_scenario::begin(admin);
    admin_cap::setup(admin, &mut scenario);
    setup(admin, &mut scenario);

    scenario.next_tx(admin);
    {
        let mut registry = test_scenario::take_shared<MonsterPartRegistry>(&scenario);
        let cap = testing_partial_cap<MonsterPartRegistryPerm>();
        let template = test_template(utf8(b"test"), 0, 0, vector::empty(), vector::empty());
        let key = key_from_template(&template);

        registry.register(&cap, template);

        registry.borrow_cache(key);

        test_scenario::return_shared(registry);
    };

    scenario.end();
}
