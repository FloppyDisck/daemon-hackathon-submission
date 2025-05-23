module daemon::versioning;

const EOutdated: u64 = 0;

public struct Version has store, copy, drop {
    number: u16
}

public fun version(number: u16): Version {
    Version { number }
}

public fun increment(ver: &mut Version) {
    ver.number = ver.number + 1;
}

/// Returns true if the object is updated to the latest version.
public fun is_updated(ver: &Version, current: u16): bool {
    ver.number == current
}

/// Aborts if the object is not in the latest version.
public fun assert_updated(ver: &Version, current: u16) {
    assert!(ver.is_updated(current), EOutdated)
}

////// Tests //////
#[test_only]
use sui::test_utils::assert_eq;

#[test]
fun test_creation() {
    let ver = version(1);
    assert_eq(ver.number, 1);
}

#[test]
fun test_increment() {
    let mut ver = version(1);
    ver.increment();
    assert_eq(ver.number, 2);
}

#[test]
fun test_is_updated() {
    let mut ver = version(1);
    ver.increment();
    assert!(ver.is_updated(2) && !ver.is_updated(3), 1);
}

#[test]
#[expected_failure(abort_code = EOutdated)]
fun test_assert_updated() {
    let mut ver = version(1);
    ver.increment();
    ver.assert_updated(3);
}
