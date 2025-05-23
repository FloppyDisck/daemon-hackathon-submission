module daemon::monster_part;

use daemon::rarity::Rarity;

#[test_only]
use daemon::rarity;

use sui::random::RandomGenerator;

use std::string::String;

public struct MonsterPart has store, copy, drop {
    name: String,
    params: vector<u32>
}

public fun monster_part(name: String, params: vector<u32>): MonsterPart {
    MonsterPart {
        name,
        params
    }
}

/////// Template to generate the monster part

public  struct MonsterPartTemplate has store, copy, drop {
    name: String,
    part_type: u16,
    rarity: Rarity,
    params: vector<MonsterPartParam>,
    parts: vector<u16>
}

public fun monster_part_template(
    name: String,
    part_type: u16,
    rarity: Rarity,
    params: vector<MonsterPartParam>,
    parts: vector<u16>
): MonsterPartTemplate {
    MonsterPartTemplate {
        name,
        part_type,
        rarity,
        params,
        parts,
    }
}

#[test_only]
public fun empty(name: String): MonsterPartTemplate {
    monster_part_template(
        name,
        0,
        rarity::from_u8(0),
        vector::empty(),
        vector::empty()
    )
}

#[test_only]
public fun test_template(
    name: String,
    part: u16,
    rarity: u8,
    params: vector<MonsterPartParam>,
    parts: vector<u16>
): MonsterPartTemplate {
    monster_part_template(
        name,
        part,
        rarity::from_u8(rarity),
        params,
        parts
    )
}

public fun name(self: &MonsterPartTemplate): String {
    self.name
}

public fun part_type(self: &MonsterPartTemplate): u16 {
    self.part_type
}

public fun rarity(self: &MonsterPartTemplate): Rarity {
    self.rarity
}

public fun params(self: &MonsterPartTemplate): &vector<MonsterPartParam> {
    &self.params
}

public fun parts(self: &MonsterPartTemplate): &vector<u16> {
    &self.parts
}

public fun borrow_mut_params(self: &mut MonsterPartTemplate): &mut vector<MonsterPartParam> {
    &mut self.params
}

public use fun generate_monster_part as MonsterPartTemplate.generate;
public(package) fun generate_monster_part(self: &MonsterPartTemplate, generator: &mut RandomGenerator): MonsterPart {
    let mut params = vector::empty();
    while (params.length() != self.params.length()) {
        let param = self.params.borrow(params.length());
        params.push_back(param.generate(generator));
    };
    monster_part(self.name, params)
}

/////// Parameters requires to generate the monster part ///////////////

public struct MonsterPartParam has store, copy, drop {
    min: u32,
    max: u32
}

public fun monster_part_param(min: u32, max: u32): MonsterPartParam {
    MonsterPartParam {
        min,
        max
    }
}

public use fun monster_part_param_generate as MonsterPartParam.generate;
public(package) fun monster_part_param_generate(self: &MonsterPartParam, generator: &mut RandomGenerator): u32 {
    generator.generate_u32_in_range(self.min, self.max)
}

#[test_only]
use sui::random::new_generator_for_testing;
#[test_only]
use std::string::utf8;

#[test]
fun generate_param() {
    let param = monster_part_param(10, 50);

    let mut i = 0;
    let mut generator = new_generator_for_testing();
    while (i < 10) {
        let gen = param.generate(&mut generator);
        assert!(gen >= 10 && gen <= 50, 1);

        i = i + 1
    }
}

#[test]
fun generate_from_template() {
    let mut params = vector::empty();

    while (params.length() < 10) {
        let len = params.length() as u32;
        params.push_back(monster_part_param(
            len * 10, (len + 1) * 10
        ));
    };

    let template = test_template(utf8(b"test"), 0, 0, params, vector::empty());

    let mut generator = new_generator_for_testing();
    let part = template.generate(&mut generator);

    let mut i = 0;
    while (i < 10) {
        let n = part.params.borrow(i);
        assert!(*n as u64 >= i * 10 && *n as u64 <= (i + 1) * 10, 0);
        i = i + 1
    };
}
