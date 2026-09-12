//! Port of `three.js/test/unit/src/core/Layers.tests.js`.

use three_rs::core::Layers;

#[test]
fn instancing() {
    let object = Layers::new();
    assert_eq!(object.mask, 1, "Can instantiate a Layers.");
}

#[test]
fn set() {
    let mut a = Layers::new();

    for i in 0..31u32 {
        a.set(i);
        assert_eq!(
            a.mask,
            2u32.pow(i),
            "Mask has the expected value for channel: {i}"
        );
    }
}

#[test]
fn enable() {
    let mut a = Layers::new();

    a.set(0);
    a.enable(0);
    assert_eq!(a.mask, 1, "Enable channel 0 with mask 0");

    a.set(0);
    a.enable(1);
    assert_eq!(a.mask, 3, "Enable channel 1 with mask 0");

    a.set(1);
    a.enable(0);
    assert_eq!(a.mask, 3, "Enable channel 0 with mask 1");

    a.set(1);
    a.enable(1);
    assert_eq!(a.mask, 2, "Enable channel 1 with mask 1");
}

#[test]
fn toggle() {
    let mut a = Layers::new();

    a.set(0);
    a.toggle(0);
    assert_eq!(a.mask, 0, "Toggle channel 0 with mask 0");

    a.set(0);
    a.toggle(1);
    assert_eq!(a.mask, 3, "Toggle channel 1 with mask 0");

    a.set(1);
    a.toggle(0);
    assert_eq!(a.mask, 3, "Toggle channel 0 with mask 1");

    a.set(1);
    a.toggle(1);
    assert_eq!(a.mask, 0, "Toggle channel 1 with mask 1");
}

#[test]
fn disable() {
    let mut a = Layers::new();

    a.set(0);
    a.disable(0);
    assert_eq!(a.mask, 0, "Disable channel 0 with mask 0");

    a.set(0);
    a.disable(1);
    assert_eq!(a.mask, 1, "Disable channel 1 with mask 0");

    a.set(1);
    a.disable(0);
    assert_eq!(a.mask, 2, "Disable channel 0 with mask 1");

    a.set(1);
    a.disable(1);
    assert_eq!(a.mask, 0, "Disable channel 1 with mask 1");
}

#[test]
fn test() {
    let mut a = Layers::new();
    let mut b = Layers::new();

    assert!(a.test(&b), "Start out true");

    a.set(1);
    assert!(!a.test(&b), "Set channel 1 in a and fail the test");

    b.toggle(1);
    assert!(a.test(&b), "Toggle channel 1 in b and pass again");
}

#[test]
fn is_enabled() {
    let mut a = Layers::new();

    a.enable(1);
    assert!(a.is_enabled(1), "Enable channel 1 and pass the test");

    a.enable(2);
    assert!(a.is_enabled(2), "Enable channel 2 and pass the test");

    a.toggle(1);
    assert!(!a.is_enabled(1), "Toggle channel 1 and fail the test");
    assert!(a.is_enabled(2), "Channel 2 still enabled and pass the test");
}

/// `enableAll` / `disableAll` have no QUnit test upstream.
#[test]
fn enable_all_disable_all() {
    let mut a = Layers::new();

    a.enable_all();
    assert_eq!(a.mask, 0xffffffff);

    a.disable_all();
    assert_eq!(a.mask, 0);
}
