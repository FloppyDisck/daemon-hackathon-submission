module daemon::admin_cap;

/// Master key used for admin operations.
public struct AdminCap has key, store {
    id: UID,
}

fun init(ctx: &mut TxContext) {
    let admin = AdminCap {
        id: object::new(ctx),
    };

    // Transfer the forge object to the module/package publisher
    transfer::transfer(admin, ctx.sender());
}

public fun wrapped_permit<T>(self: &AdminCap, ctx: &mut TxContext): WrappedPartialCap<T> {
    let cap = self.permit<T>();
    WrappedPartialCap {
        id: object::new(ctx),
        cap
    }
}

public fun permit<T>(_: &AdminCap): PartialCap<T> {
    PartialCap {}
}

#[test_only]
public fun testing_admin_cap(ctx: &mut TxContext): AdminCap {
    AdminCap {
        id: object::new(ctx),
    }
}


/// A permanent wrapper for partial caps, can only be provided by the admin key
public struct WrappedPartialCap<phantom T> has key, store {
    id: UID,
    cap: PartialCap<T>
}

public use fun borrow_wrapped_partial_cap as WrappedPartialCap.borrow;
public fun borrow_wrapped_partial_cap<T>(self: &WrappedPartialCap<T>): &PartialCap<T> {
    &self.cap
}

public use fun unwrap_wrapped_partial_cap as WrappedPartialCap.unwrap;
public fun unwrap_wrapped_partial_cap<T>(self: WrappedPartialCap<T>): PartialCap<T> {
    let WrappedPartialCap { id, cap } = self;
    object::delete(id);
    cap
}

/// Keys used for permissioned operations, this is limited to have admin power over one of the modules
public struct PartialCap<phantom T> has store, drop {}

#[test_only]
public fun testing_partial_cap<T>(): PartialCap<T> {
    PartialCap {}
}

// Encrypted Drive
public struct EncryptedDriveMinterPerm {}
public struct EncryptedDrivePerm {}

// Registry
public struct MonsterPartRegistryPerm {}

// Monster
public struct MonsterMinterPerm {}
public struct MonsterPerm {}

// Distribution Table
public struct DistributionTablePerm {}

#[test_only]
use sui::{test_scenario, random};

#[test_only]
public fun setup(admin: address, scenario: &mut test_scenario::Scenario) {
    let chain = @0x0;

    // Chain setup
    scenario.next_tx(chain);
    {
        random::create_for_testing(scenario.ctx());
    };

    // Package setup
    scenario.next_tx(admin);
    {
        // Create the admin cap
        let admin_cap = daemon::admin_cap::testing_admin_cap(scenario.ctx());
        transfer::public_transfer(admin_cap, admin);
    };
}
