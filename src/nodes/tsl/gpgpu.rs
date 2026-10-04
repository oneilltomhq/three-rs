//! `SubgroupFunctionNode`'s TSL functions (sweep 6,
//! `src/nodes/gpgpu/SubgroupFunctionNode.js`): the subgroup reductions,
//! scans, votes, broadcasts and shuffles, and the quad swaps. Each one is a
//! [`Node::Subgroup`] whose WGSL is pinned against three's own dump in
//! `tests/nodes_tsl_batch.rs`.
//!
//! Three exports them through `nodeProxyIntent`, which wraps the node in
//! `.toVarIntent()`; as everywhere in the port the plain node is three's
//! output, and [`to_var_intent`](super::to_var_intent) is the caller's when
//! the result is assigned to.
//!
//! Building one enables `subgroups` in its stage, as `SubgroupFunctionNode.
//! setup()` does: the module gets `enable subgroups;` (a compute entry point
//! also gets `@builtin( subgroup_size )`), and it needs a device with
//! `wgpu::Features::SUBGROUP`, which the renderer requests when the adapter
//! has it. Like three, the port refuses them in the vertex stage. Without the
//! feature, three logs `The 'subgroups' feature is not supported by the
//! current device.` and the browser rejects the module; the port's renderer
//! logs the same and skips the kernel (`docs/nodes.md` §84).

use crate::nodes::node::{Node, NodeRef};

fn subgroup(method: &'static str, a: Option<NodeRef>, b: Option<NodeRef>) -> NodeRef {
    NodeRef::new(Node::Subgroup { method, a, b })
}

/// `subgroupElect()` — `true` in exactly one active invocation of the
/// subgroup, the lowest. A `bool`.
///
/// **Native gap** (`docs/nodes.md` §84): the WGSL matches three's, but naga
/// 30's WGSL front end reserves `subgroupElect` as a keyword and has no
/// lowering for it, so a native kernel that calls it fails to parse. It
/// builds; it runs only where the WGSL goes to the browser.
pub fn subgroup_elect() -> NodeRef {
    subgroup("subgroupElect", None, None)
}

/// `subgroupBallot( pred )` — a `uvec4` bitmask of the invocations whose
/// `pred` is `true`.
pub fn subgroup_ballot(pred: impl Into<NodeRef>) -> NodeRef {
    subgroup("subgroupBallot", Some(pred.into()), None)
}

macro_rules! subgroup_unary {
    ($($(#[$m:meta])* $name:ident => $method:literal;)*) => {
        $(
            $(#[$m])*
            pub fn $name(e: impl Into<NodeRef>) -> NodeRef {
                subgroup($method, Some(e.into()), None)
            }
        )*
    };
}

subgroup_unary! {
    /// `subgroupAdd( e )` — the sum of `e` over the subgroup's active
    /// invocations, the same in each.
    subgroup_add => "subgroupAdd";
    /// `subgroupInclusiveAdd( e )` — the sum over this invocation and every
    /// active one below it.
    subgroup_inclusive_add => "subgroupInclusiveAdd";
    /// `subgroupExclusiveAdd( e )` — the sum over the active invocations below
    /// this one.
    subgroup_exclusive_add => "subgroupExclusiveAdd";
    /// `subgroupMul( e )` — the product over the subgroup.
    subgroup_mul => "subgroupMul";
    /// `subgroupInclusiveMul( e )` — the inclusive prefix product.
    subgroup_inclusive_mul => "subgroupInclusiveMul";
    /// `subgroupExclusiveMul( e )` — the exclusive prefix product.
    subgroup_exclusive_mul => "subgroupExclusiveMul";
    /// `subgroupAnd( e )` — the bitwise and over the subgroup; integers only.
    subgroup_and => "subgroupAnd";
    /// `subgroupOr( e )` — the bitwise or over the subgroup.
    subgroup_or => "subgroupOr";
    /// `subgroupXor( e )` — the bitwise xor over the subgroup.
    subgroup_xor => "subgroupXor";
    /// `subgroupMin( e )` — the minimum over the subgroup.
    subgroup_min => "subgroupMin";
    /// `subgroupMax( e )` — the maximum over the subgroup.
    subgroup_max => "subgroupMax";
    /// `subgroupAll( e )` — whether `e` is `true` in every active invocation.
    subgroup_all => "subgroupAll";
    /// `subgroupAny( e )` — whether `e` is `true` in any active invocation.
    subgroup_any => "subgroupAny";
    /// `subgroupBroadcastFirst( e )` — `e` from the lowest active invocation.
    subgroup_broadcast_first => "subgroupBroadcastFirst";
    /// `quadSwapX( e )` — `e` from the invocation across the quad
    /// horizontally.
    quad_swap_x => "quadSwapX";
    /// `quadSwapY( e )` — `e` from the invocation across the quad vertically.
    quad_swap_y => "quadSwapY";
    /// `quadSwapDiagonal( e )` — `e` from the diagonally opposite invocation
    /// of the quad.
    quad_swap_diagonal => "quadSwapDiagonal";
}

macro_rules! subgroup_binary {
    ($($(#[$m:meta])* $name:ident($b:ident) => $method:literal;)*) => {
        $(
            $(#[$m])*
            pub fn $name(e: impl Into<NodeRef>, $b: impl Into<NodeRef>) -> NodeRef {
                subgroup($method, Some(e.into()), Some($b.into()))
            }
        )*
    };
}

subgroup_binary! {
    /// `subgroupBroadcast( e, id )` — `e` from invocation `id`. WGSL wants
    /// `id` to be a constant: three builds a `float` id as an `int`, so a
    /// plain number (`3.0`) becomes the literal `3`; any other type is built
    /// at the node's type, as three does.
    ///
    /// **Native gap** (`docs/nodes.md` §84): WGSL takes an `i32` or `u32` id,
    /// but naga 30 accepts only `u32`. Three's rule builds the id as an `int`
    /// or at the input type, where the id wins a tie, so natively any scalar
    /// `e` with a `u32` id validates, at the cost of converting `e` to `u32`;
    /// a vector `e` fails, as three's own `int` ids do.
    subgroup_broadcast(id) => "subgroupBroadcast";
    /// `subgroupShuffle( e, id )` — `e` from invocation `id`, which may vary
    /// per invocation. `id` is built like [`subgroup_broadcast`]'s, with the
    /// same native gap: only a `u32` id passes naga, which converts a scalar
    /// `e` to `u32`.
    subgroup_shuffle(id) => "subgroupShuffle";
    /// `subgroupShuffleXor( e, mask )` — `e` from invocation
    /// `subgroup_invocation_id ^ mask`; `mask` is built as a `uint`.
    subgroup_shuffle_xor(mask) => "subgroupShuffleXor";
    /// `subgroupShuffleUp( e, delta )` — `e` from invocation
    /// `subgroup_invocation_id - delta`; `delta` is built as a `uint`.
    subgroup_shuffle_up(delta) => "subgroupShuffleUp";
    /// `subgroupShuffleDown( e, delta )` — `e` from invocation
    /// `subgroup_invocation_id + delta`; `delta` is built as a `uint`.
    subgroup_shuffle_down(delta) => "subgroupShuffleDown";
    /// `quadBroadcast( e, id )` — `e` from quad invocation `id`, a constant
    /// built like [`subgroup_broadcast`]'s, with the same native gap.
    ///
    /// **Divergence, arity** (`docs/nodes.md` §84): three at 5f610f5 declares
    /// it `setParameterLength( 1 )`, so `quadBroadcast( e, id )` logs
    /// "parameter length exceeds limit" and drops `id`, and `generate()` then
    /// reads `getNodeType()` off the missing `bNode` and throws — three cannot
    /// build it at all. The port takes the `id` WGSL's `quadBroadcast` needs
    /// and generates what three's `QUAD_BROADCAST` arm was written to.
    quad_broadcast(id) => "quadBroadcast";
}
