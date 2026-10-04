//! `KeyboardEvent.code`, narrowed to the keys the ported controls read.

/// A DOM `KeyboardEvent.code`, as a value: the physical keys
/// [`FirstPersonControls`](super::FirstPersonControls) and
/// [`FlyControls`](super::FlyControls) bind. Both JS classes switch on
/// `event.code`, which names a key by its position on a US layout, so `KeyW`
/// is the key left of `KeyE` on an AZERTY keyboard too; a host that maps its
/// own key events onto these should map physical keys (winit's `PhysicalKey`),
/// not the characters they type.
///
/// Every other code is [`KeyCode::Other`], which both classes ignore, as their
/// `switch` statements do.
///
/// Non-exhaustive: a later port that binds another key adds a variant.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum KeyCode {
    /// `"ArrowUp"`.
    ArrowUp,
    /// `"ArrowDown"`.
    ArrowDown,
    /// `"ArrowLeft"`.
    ArrowLeft,
    /// `"ArrowRight"`.
    ArrowRight,
    /// `"KeyW"`.
    KeyW,
    /// `"KeyA"`.
    KeyA,
    /// `"KeyS"`.
    KeyS,
    /// `"KeyD"`.
    KeyD,
    /// `"KeyR"`.
    KeyR,
    /// `"KeyF"`.
    KeyF,
    /// `"KeyQ"`.
    KeyQ,
    /// `"KeyE"`.
    KeyE,
    /// `"ShiftLeft"`.
    ShiftLeft,
    /// `"ShiftRight"`.
    ShiftRight,
    /// Any other code.
    Other,
}
