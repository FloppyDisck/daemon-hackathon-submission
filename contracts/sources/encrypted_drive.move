module daemon::encrypted_drive;

use sui::{
    balance::{Balance, zero},
    coin::{Coin, from_balance, put},
    package,
    display,
    event,
};
use daemon::{
    admin_cap::{PartialCap, EncryptedDrivePerm as DrivePerm, EncryptedDriveMinterPerm as DriveMinterPerm},
    versioning::{Version, version},
};

const VERSION: u16 = 1;

////// ERRORS ///////

const EIncorrectPaymentAmount: u64 = 0;
const EMintingDisabled: u64 = 1;

////// EVENTS ////////
public struct EncryptedDriveMinted has copy, drop {
    id: ID,
}

public struct EncryptedDriveBurned has copy, drop {
    id: ID
}

public struct MinterCreated has copy, drop {
    id: ID,
    price: u64
}

public struct MinterSetPrice has copy, drop {
    price: u64,
}

public struct MinterEnable has copy, drop {
    enable: bool
}

public struct MinterWithdrawn has copy, drop {
    amount: u64
}

/// OTW
public struct ENCRYPTED_DRIVE has drop {}

fun init(otw: ENCRYPTED_DRIVE, ctx: &mut TxContext) {
    let keys = vector[
        b"id".to_string(),
        b"description".to_string(),
        b"project_url".to_string(),
    ];

    let values = vector[
        b"{id}".to_string(),
        b"A drive that contains encrypted daemon information. Decrypt it to see what this monster can do.".to_string(),
        b"daemon.computer".to_string()
    ];

    let publisher = package::claim(otw, ctx);

    let mut display = display::new_with_fields<EncryptedDrive>(
        &publisher, keys, values, ctx
    );

    display.update_version();

    transfer::public_transfer(publisher, ctx.sender());
    transfer::public_transfer(display, ctx.sender());
}

//////// Encrypted Drive /////////

public struct EncryptedDrive has key, store {
    id: UID,
    version: Version,
}

/// Creates a new encrypted drive with the given protocol type.
public fun encrypted_drive(_: &PartialCap<DrivePerm>, ctx: &mut TxContext): EncryptedDrive {
    let drive = EncryptedDrive {
        id: object::new(ctx),
        version: version(VERSION),
    };

    event::emit(EncryptedDriveMinted {
        id: object::id(&drive),
    });

    drive
}

///Burns the encrypted drive and returns its protocol
public fun burn(self: EncryptedDrive) {
    self.version.assert_updated(VERSION);

    event::emit(EncryptedDriveBurned { id: object::id(&self) });

    let EncryptedDrive { id, version: _ } = self;
    object::delete(id);
}

public fun assert_version(self: &EncryptedDrive) {
    self.version.assert_updated(VERSION);
}

#[test_only]
public fun testing_encrypted_drive(ctx: &mut TxContext): EncryptedDrive {
    testing_encrypted_drive_with_version(VERSION, ctx)
}

#[test_only]
public fun testing_encrypted_drive_with_version(version: u16, ctx: &mut TxContext): EncryptedDrive {
    EncryptedDrive {
        id: object::new(ctx),
        version: version(version)
    }
}

////// Encrypted Drive Minter ///////

public struct EncryptedDriveMinter<phantom T> has key, store {
    id: UID,
    cap: PartialCap<DrivePerm>,
    version: Version,

    /// Base price for the random minting.
    price: u64,
    /// Enabled status of the minter, indicating whether it can mint new drives.
    enabled: bool,
    /// Balance of the minter.
    balance: Balance<T>,
}

/// Creates a new encrypted drive minter with the given damage type.
public fun encrypted_drive_minter<T>(_: PartialCap<DriveMinterPerm>, cap: PartialCap<DrivePerm>, price: u64, ctx: &mut TxContext) {
    let minter = EncryptedDriveMinter<T> {
        id: object::new(ctx),
        cap,
        price,
        version: version(VERSION),
        enabled: false,
        balance: zero(),
    };

    event::emit(MinterCreated { id: object::id(&minter), price });

    transfer::share_object(minter);
}

/// Creates a new random encrypted drive minter with the given damage type.
entry fun mint<T>(minter: &mut EncryptedDriveMinter<T>, payment: Coin<T>, ctx: &mut TxContext) {
    let base_price = minter.price;
    minter.mint_assertions(base_price, payment);

    let drive = encrypted_drive(&minter.cap, ctx);
    transfer::public_transfer(drive, ctx.sender());
}

/// Drains the current minter balance
public fun withdraw<T>(self: &mut EncryptedDriveMinter<T>,  _: &PartialCap<DriveMinterPerm>, ctx: &mut TxContext): Coin<T> {
    self.version.assert_updated(VERSION);
    let withdrawn_balance = self.balance.withdraw_all();

    event::emit(MinterWithdrawn { amount: withdrawn_balance.value() });

    from_balance(withdrawn_balance, ctx)
}

////// Admin Encrypted Drive Minter Functions //////

/// Sets the enabled status of the encrypted drive minter
public fun set_enabled<T>(self: &mut EncryptedDriveMinter<T>, _: &PartialCap<DriveMinterPerm>, enabled: bool) {
    event::emit(MinterEnable { enable: enabled });

    self.enabled = enabled;
}

/// Sets the price of the encrypted drive minter
public fun set_price<T>(self: &mut EncryptedDriveMinter<T>, _: &PartialCap<DriveMinterPerm>, price: u64) {
    event::emit(MinterSetPrice { price });

    self.price = price;
}

///// Encrypted Drive Minter Helpers //////

fun mint_assertions<T>(minter: &mut EncryptedDriveMinter<T>, expected_amount: u64, payment: Coin<T>) {
    minter.version.assert_updated(VERSION);
    assert!(payment.value() == expected_amount, EIncorrectPaymentAmount);
    assert!(minter.enabled, EMintingDisabled);
    put(&mut minter.balance, payment);
}

#[test_only]
public fun testing_encrypted_drive_minter<T>(price: u64, ctx: &mut TxContext) {
    let cap = testing_partial_cap<DrivePerm>();
    encrypted_drive_minter<T>(testing_partial_cap<DriveMinterPerm>(), cap, price, ctx);
}

#[test_only]
fun change_version<T>(minter: &mut EncryptedDriveMinter<T>, version: u16) {
    minter.version = version(version);
}

////// Tests //////
#[test_only]
use sui::{test_scenario, sui::SUI, coin};
#[test_only]
use daemon::{
    versioning::EOutdated,
    admin_cap::{Self, AdminCap, testing_partial_cap},
    distribution_table::quick_test_distribution_table,
};

#[test_only]
fun setup(admin: address, scenario: &mut test_scenario::Scenario) {
    // Package setup
    scenario.next_tx(admin);
    {
        // Create the admin cap for the minter
        let cap = testing_partial_cap<DrivePerm>();
        // Init minter
        encrypted_drive_minter<SUI>(testing_partial_cap<DriveMinterPerm>(), cap, 10, scenario.ctx());
    };

    scenario.next_tx(admin);
    {
        quick_test_distribution_table(scenario.ctx());

        // Enable minter
        let mut minter = scenario.take_shared<EncryptedDriveMinter<SUI>>();
        let cap = testing_partial_cap<DriveMinterPerm>();
        minter.set_enabled(&cap, true);

        test_scenario::return_shared(minter);
    };
}

#[test]
fun test_mint() {
    let admin = @0x01;
    let user = @0x02;

    // Chain setup
    let mut scenario = test_scenario::begin(admin);
    admin_cap::setup(admin, &mut scenario);
    setup(admin, &mut scenario);

    // Purchase drive
    scenario.next_tx(user);
    {
        let mut minter = scenario.take_shared<EncryptedDriveMinter<SUI>>();
        let coin = coin::mint_for_testing(10, scenario.ctx());
        minter.mint(coin, scenario.ctx());
        test_scenario::return_shared(minter);
    };

    scenario.end();
}

#[test]
#[expected_failure(abort_code = EIncorrectPaymentAmount)]
fun incorrect_amount() {
    let admin = @0x01;
    let user = @0x02;

    // Chain setup
    let mut scenario = test_scenario::begin(admin);
    admin_cap::setup(admin, &mut scenario);
    setup(admin, &mut scenario);

    // Purchase drive
    scenario.next_tx(user);
    {
        let mut minter = scenario.take_shared<EncryptedDriveMinter<SUI>>();
        let coin = coin::mint_for_testing(9, scenario.ctx());
        minter.mint(coin, scenario.ctx());
        test_scenario::return_shared(minter);
    };

    scenario.end();
}

#[test]
#[expected_failure(abort_code = EOutdated)]
fun mint_incorrect_version() {
    let admin = @0x01;
    let user = @0x02;

    // Chain setup
    let mut scenario = test_scenario::begin(admin);
    admin_cap::setup(admin, &mut scenario);
    setup(admin, &mut scenario);

    // Set incorrect version
    scenario.next_tx(admin);
    {
        let mut minter = scenario.take_shared<EncryptedDriveMinter<SUI>>();
        minter.change_version(0);
        test_scenario::return_shared(minter);
    };

    // Purchase drive
    scenario.next_tx(user);
    {
        let mut minter = scenario.take_shared<EncryptedDriveMinter<SUI>>();
        let coin = coin::mint_for_testing(9, scenario.ctx());
        minter.mint(coin, scenario.ctx());
        test_scenario::return_shared(minter);
    };

    scenario.end();
}

#[test]
#[expected_failure(abort_code = EMintingDisabled)]
fun mint_disabled() {
    let admin = @0x01;
    let user = @0x02;

    // Chain setup
    let mut scenario = test_scenario::begin(admin);
    admin_cap::setup(admin, &mut scenario);
    setup(admin, &mut scenario);

    // Disable minter
    scenario.next_tx(admin);
    {
        let admin_cap = scenario.take_from_sender<AdminCap>();

        // Enable minter
        let mut minter = scenario.take_shared<EncryptedDriveMinter<SUI>>();
        let cap = admin_cap.permit<DriveMinterPerm>();
        minter.set_enabled(&cap, false);
        test_scenario::return_shared(minter);
        scenario.return_to_sender(admin_cap);
    };

    // Purchase drive
    scenario.next_tx(user);
    {
        let mut minter = scenario.take_shared<EncryptedDriveMinter<SUI>>();
        let coin = coin::mint_for_testing(10, scenario.ctx());
        minter.mint(coin, scenario.ctx());
        test_scenario::return_shared(minter);
    };

    scenario.end();
}
