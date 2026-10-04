//! Port of `three.js/src/constants.js`' blending constants plus
//! `WebGPUPipelineUtils`' `_getBlending()` / `_getBlendFactor()` /
//! `_getBlendOperation()` — the table that turns a material's blending fields
//! into a WebGPU blend state.
//!
//! Three spells the constants as integers on `Material`; here they are three
//! enums, so a row of the table cannot be reached with a value the match does
//! not name. The mapping itself is literal: see `docs/nodes.md` §9.

/// `NoBlending` … `CustomBlending`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Blending {
    /// `NoBlending` — the pipeline gets no blend state, whatever `transparent` is.
    No,
    /// `NormalBlending` — three's default.
    #[default]
    Normal,
    /// `AdditiveBlending`.
    Additive,
    /// `SubtractiveBlending`. Ignored unless `premultipliedAlpha` is set;
    /// see `WebGPUPipelineUtils._getBlending()`.
    Subtractive,
    /// `MultiplyBlending`. Ignored unless `premultipliedAlpha` is set;
    /// see `WebGPUPipelineUtils._getBlending()`.
    Multiply,
    /// `CustomBlending` — `blendSrc`/`blendDst`/`blendEquation` (and the three
    /// `*Alpha` overrides) are used verbatim.
    Custom,
}

/// `ZeroFactor` … `OneMinusConstantAlphaFactor`, used only by `CustomBlending`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    /// `ZeroFactor`.
    Zero,
    /// `OneFactor`.
    One,
    /// `SrcColorFactor`.
    SrcColor,
    /// `OneMinusSrcColorFactor`.
    OneMinusSrcColor,
    /// `SrcAlphaFactor`.
    SrcAlpha,
    /// `OneMinusSrcAlphaFactor`.
    OneMinusSrcAlpha,
    /// `DstColorFactor`.
    DstColor,
    /// `OneMinusDstColorFactor`.
    OneMinusDstColor,
    /// `DstAlphaFactor`.
    DstAlpha,
    /// `OneMinusDstAlphaFactor`.
    OneMinusDstAlpha,
    /// `SrcAlphaSaturateFactor`.
    SrcAlphaSaturate,
    /// `ConstantColorFactor` — WebGPU has no separate constant-colour factor,
    /// so this maps to the same `wgpu::BlendFactor::Constant` as
    /// [`ConstantAlpha`](Self::ConstantAlpha).
    ConstantColor,
    /// `OneMinusConstantColorFactor` — collapses with
    /// [`OneMinusConstantAlpha`](Self::OneMinusConstantAlpha), for the same
    /// reason as [`ConstantColor`](Self::ConstantColor).
    OneMinusConstantColor,
    /// `ConstantAlphaFactor`.
    ConstantAlpha,
    /// `OneMinusConstantAlphaFactor`.
    OneMinusConstantAlpha,
}

/// `AddEquation` … `MaxEquation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendEquation {
    /// `AddEquation`.
    Add,
    /// `SubtractEquation`.
    Subtract,
    /// `ReverseSubtractEquation`.
    ReverseSubtract,
    /// `MinEquation`.
    Min,
    /// `MaxEquation`.
    Max,
}

/// `WebGPUPipelineUtils._getBlendFactor( blend )`. Note the two collapses Three
/// documents: WebGPU has no dedicated constant-*alpha* factors, so
/// `ConstantAlphaFactor` and `ConstantColorFactor` both become `Constant`.
pub(crate) fn blend_factor(blend: BlendFactor) -> wgpu::BlendFactor {
    match blend {
        BlendFactor::Zero => wgpu::BlendFactor::Zero,
        BlendFactor::One => wgpu::BlendFactor::One,
        BlendFactor::SrcColor => wgpu::BlendFactor::Src,
        BlendFactor::OneMinusSrcColor => wgpu::BlendFactor::OneMinusSrc,
        BlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
        BlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        BlendFactor::DstColor => wgpu::BlendFactor::Dst,
        BlendFactor::OneMinusDstColor => wgpu::BlendFactor::OneMinusDst,
        BlendFactor::DstAlpha => wgpu::BlendFactor::DstAlpha,
        BlendFactor::OneMinusDstAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
        BlendFactor::SrcAlphaSaturate => wgpu::BlendFactor::SrcAlphaSaturated,
        BlendFactor::ConstantColor | BlendFactor::ConstantAlpha => wgpu::BlendFactor::Constant,
        BlendFactor::OneMinusConstantColor | BlendFactor::OneMinusConstantAlpha => {
            wgpu::BlendFactor::OneMinusConstant
        }
    }
}

/// `WebGPUPipelineUtils._getBlendOperation( blendEquation )`.
pub(crate) fn blend_operation(equation: BlendEquation) -> wgpu::BlendOperation {
    match equation {
        BlendEquation::Add => wgpu::BlendOperation::Add,
        BlendEquation::Subtract => wgpu::BlendOperation::Subtract,
        BlendEquation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
        BlendEquation::Min => wgpu::BlendOperation::Min,
        BlendEquation::Max => wgpu::BlendOperation::Max,
    }
}

/// `BlendMode` (`three.js/src/renderers/common/BlendMode.js`) — the blending
/// fields `_getBlending()` reads, as one value.
///
/// Three has two readers of the same seven fields: a `Material`, and a
/// stand-alone `BlendMode` an MRT gives one of its attachments
/// (`mrtNode.setBlendMode( name, blendMode )`). Here both are this struct: a
/// material builds one from its own fields
/// (`MeshBasicNodeMaterial::blend_mode`), and
/// [`MrtNode::set_blend_mode`](crate::nodes::MrtNode::set_blend_mode) stores
/// one per output name. `OITPassNode` is what needs the custom factors — its
/// `accum` attachment blends `One`/`One` and its `revealage` one
/// `Zero`/`OneMinusSrcColor`.
///
/// `Default` is `new BlendMode()`: `NormalBlending`, `SrcAlpha` /
/// `OneMinusSrcAlpha` / `Add`, the three alpha overrides `null`.
///
/// Three's `BlendMode` spells the premultiply flag `premultiplyAlpha`, which
/// `_getBlending()` (reading `premultipliedAlpha`) never sees, so on an MRT
/// attachment it is always off. The field here is `premultiplied_alpha` —
/// the material's spelling, the one the table reads — and it is `false` in
/// every `BlendMode` the port builds for an MRT, which is the same result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlendMode {
    /// `blendMode.blending`.
    pub blending: Blending,
    /// `material.premultipliedAlpha` — see the type's doc for why an MRT's is
    /// always `false`.
    pub premultiplied_alpha: bool,
    /// `blendSrc`, used only by `CustomBlending`.
    pub blend_src: BlendFactor,
    /// `blendDst`, used only by `CustomBlending`.
    pub blend_dst: BlendFactor,
    /// `blendEquation`, used only by `CustomBlending`.
    pub blend_equation: BlendEquation,
    /// `blendSrcAlpha` — `null` in Three, meaning "use the colour one".
    pub blend_src_alpha: Option<BlendFactor>,
    /// `blendDstAlpha` — `null` means [`blend_dst`](Self::blend_dst).
    pub blend_dst_alpha: Option<BlendFactor>,
    /// `blendEquationAlpha` — `null` means
    /// [`blend_equation`](Self::blend_equation).
    pub blend_equation_alpha: Option<BlendEquation>,
}

impl BlendMode {
    /// `new BlendMode( blending )` — every other field at its default.
    pub fn new(blending: Blending) -> Self {
        Self {
            blending,
            ..Self::default()
        }
    }
}

/// `new BlendMode( blending )`, so a preset can be passed wherever a
/// `BlendMode` is taken.
impl From<Blending> for BlendMode {
    fn from(blending: Blending) -> Self {
        Self::new(blending)
    }
}

impl Default for BlendMode {
    fn default() -> Self {
        Self {
            blending: Blending::Normal,
            premultiplied_alpha: false,
            blend_src: BlendFactor::SrcAlpha,
            blend_dst: BlendFactor::OneMinusSrcAlpha,
            blend_equation: BlendEquation::Add,
            blend_src_alpha: None,
            blend_dst_alpha: None,
            blend_equation_alpha: None,
        }
    }
}

/// `WebGPUPipelineUtils.createRenderPipeline()`'s gate:
///
/// ```js
/// if ( material.blending !== NoBlending && ( material.blending !== NormalBlending || material.transparent !== false ) )
/// ```
///
/// This is why rungs 1–5 and 9, all opaque `NormalBlending` materials, emit no
/// blend state at all.
pub(crate) fn needs_blend_state(mode: &BlendMode, transparent: bool) -> bool {
    mode.blending != Blending::No && (mode.blending != Blending::Normal || transparent)
}

/// `WebGPUPipelineUtils._getBlending( object )`.
///
/// `None` is Three's `undefined` return: `SubtractiveBlending` and
/// `MultiplyBlending` without `premultipliedAlpha` log
/// `"requires material.premultipliedAlpha = true"` and fall through the
/// `color !== undefined && alpha !== undefined` check, so the pipeline ends up
/// with no blend state. `NoBlending` never reaches here (see
/// [`needs_blend_state`]), and is `None` for the same reason.
pub(crate) fn blending(mode: &BlendMode) -> Option<wgpu::BlendState> {
    // The non-custom rows: `setBlend( srcRGB, dstRGB, srcAlpha, dstAlpha )`,
    // both operations always `Add`.
    let set_blend = |src_rgb, dst_rgb, src_alpha, dst_alpha| {
        Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: blend_factor(src_rgb),
                dst_factor: blend_factor(dst_rgb),
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: blend_factor(src_alpha),
                dst_factor: blend_factor(dst_alpha),
                operation: wgpu::BlendOperation::Add,
            },
        })
    };

    use BlendFactor::*;

    match mode.blending {
        Blending::Custom => {
            let src_alpha = mode.blend_src_alpha.unwrap_or(mode.blend_src);
            let dst_alpha = mode.blend_dst_alpha.unwrap_or(mode.blend_dst);
            let equation_alpha = mode.blend_equation_alpha.unwrap_or(mode.blend_equation);

            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: blend_factor(mode.blend_src),
                    dst_factor: blend_factor(mode.blend_dst),
                    operation: blend_operation(mode.blend_equation),
                },
                alpha: wgpu::BlendComponent {
                    src_factor: blend_factor(src_alpha),
                    dst_factor: blend_factor(dst_alpha),
                    operation: blend_operation(equation_alpha),
                },
            })
        }
        Blending::No => None,
        _ if mode.premultiplied_alpha => match mode.blending {
            Blending::Normal => set_blend(One, OneMinusSrcAlpha, One, OneMinusSrcAlpha),
            Blending::Additive => set_blend(One, One, One, One),
            Blending::Subtractive => set_blend(Zero, OneMinusSrcColor, Zero, One),
            Blending::Multiply => set_blend(DstColor, OneMinusSrcAlpha, Zero, One),
            Blending::No | Blending::Custom => {
                unreachable!("three-rs: NoBlending and CustomBlending are handled above")
            }
        },
        _ => match mode.blending {
            Blending::Normal => set_blend(SrcAlpha, OneMinusSrcAlpha, One, OneMinusSrcAlpha),
            Blending::Additive => set_blend(SrcAlpha, One, One, One),
            // `error( 'WebGPURenderer: "SubtractiveBlending" requires
            // "material.premultipliedAlpha = true".' )` — no blend state.
            Blending::Subtractive | Blending::Multiply => None,
            Blending::No | Blending::Custom => {
                unreachable!("three-rs: NoBlending and CustomBlending are handled above")
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::BlendFactor as F;
    use wgpu::BlendOperation as Op;

    /// `( srcFactor, dstFactor, operation )` of one blend component.
    type BlendRow = (F, F, Op);

    /// `( srcFactor, dstFactor, operation )` of both components, in the shape
    /// the dumped `GPURenderPipelineDescriptor.blend` prints.
    fn rows(mode: &BlendMode) -> Option<(BlendRow, BlendRow)> {
        blending(mode).map(|b| {
            (
                (b.color.src_factor, b.color.dst_factor, b.color.operation),
                (b.alpha.src_factor, b.alpha.dst_factor, b.alpha.operation),
            )
        })
    }

    fn mode(blending: Blending, premultiplied_alpha: bool) -> BlendMode {
        BlendMode {
            blending,
            premultiplied_alpha,
            ..Default::default()
        }
    }

    #[test]
    fn no_blending_has_no_state() {
        assert_eq!(rows(&mode(Blending::No, false)), None);
        assert_eq!(rows(&mode(Blending::No, true)), None);
    }

    #[test]
    fn non_premultiplied_rows() {
        // `setBlend( SrcAlpha, OneMinusSrcAlpha, One, OneMinusSrcAlpha )`
        assert_eq!(
            rows(&mode(Blending::Normal, false)),
            Some((
                (F::SrcAlpha, F::OneMinusSrcAlpha, Op::Add),
                (F::One, F::OneMinusSrcAlpha, Op::Add)
            ))
        );
        // `setBlend( SrcAlpha, One, One, One )` — the rung-13 row, matching
        // `renderPipeline_SpriteNodeMaterial_17` in the rung-13 dump.
        assert_eq!(
            rows(&mode(Blending::Additive, false)),
            Some(((F::SrcAlpha, F::One, Op::Add), (F::One, F::One, Op::Add)))
        );
        // Both log an error and leave the blend state undefined.
        assert_eq!(rows(&mode(Blending::Subtractive, false)), None);
        assert_eq!(rows(&mode(Blending::Multiply, false)), None);
    }

    #[test]
    fn premultiplied_rows() {
        assert_eq!(
            rows(&mode(Blending::Normal, true)),
            Some((
                (F::One, F::OneMinusSrcAlpha, Op::Add),
                (F::One, F::OneMinusSrcAlpha, Op::Add)
            ))
        );
        assert_eq!(
            rows(&mode(Blending::Additive, true)),
            Some(((F::One, F::One, Op::Add), (F::One, F::One, Op::Add)))
        );
        assert_eq!(
            rows(&mode(Blending::Subtractive, true)),
            Some((
                (F::Zero, F::OneMinusSrc, Op::Add),
                (F::Zero, F::One, Op::Add)
            ))
        );
        assert_eq!(
            rows(&mode(Blending::Multiply, true)),
            Some((
                (F::Dst, F::OneMinusSrcAlpha, Op::Add),
                (F::Zero, F::One, Op::Add)
            ))
        );
    }

    #[test]
    fn custom_blending_uses_the_material_factors() {
        // `Material`'s own defaults for the custom fields, which reproduce
        // `NormalBlending` on the colour component and apply the same pair to
        // alpha, since the three `*Alpha` fields are null.
        let mut mode = mode(Blending::Custom, false);
        assert_eq!(
            rows(&mode),
            Some((
                (F::SrcAlpha, F::OneMinusSrcAlpha, Op::Add),
                (F::SrcAlpha, F::OneMinusSrcAlpha, Op::Add)
            ))
        );

        // The `*Alpha` overrides are taken one by one, as
        // `blendSrcAlpha !== null ? blendSrcAlpha : blendSrc` does.
        mode.blend_src_alpha = Some(BlendFactor::One);
        mode.blend_dst_alpha = Some(BlendFactor::Zero);
        mode.blend_equation_alpha = Some(BlendEquation::Max);
        mode.blend_equation = BlendEquation::ReverseSubtract;
        assert_eq!(
            rows(&mode),
            Some((
                (F::SrcAlpha, F::OneMinusSrcAlpha, Op::ReverseSubtract),
                (F::One, F::Zero, Op::Max)
            ))
        );
    }

    #[test]
    fn every_blend_factor_maps_as_three_does() {
        let table = [
            (BlendFactor::Zero, F::Zero),
            (BlendFactor::One, F::One),
            (BlendFactor::SrcColor, F::Src),
            (BlendFactor::OneMinusSrcColor, F::OneMinusSrc),
            (BlendFactor::SrcAlpha, F::SrcAlpha),
            (BlendFactor::OneMinusSrcAlpha, F::OneMinusSrcAlpha),
            (BlendFactor::DstColor, F::Dst),
            (BlendFactor::OneMinusDstColor, F::OneMinusDst),
            (BlendFactor::DstAlpha, F::DstAlpha),
            (BlendFactor::OneMinusDstAlpha, F::OneMinusDstAlpha),
            (BlendFactor::SrcAlphaSaturate, F::SrcAlphaSaturated),
            (BlendFactor::ConstantColor, F::Constant),
            (BlendFactor::ConstantAlpha, F::Constant),
            (BlendFactor::OneMinusConstantColor, F::OneMinusConstant),
            (BlendFactor::OneMinusConstantAlpha, F::OneMinusConstant),
        ];
        for (three, gpu) in table {
            assert_eq!(blend_factor(three), gpu, "{three:?}");
        }
    }

    #[test]
    fn every_blend_equation_maps_as_three_does() {
        assert_eq!(blend_operation(BlendEquation::Add), Op::Add);
        assert_eq!(blend_operation(BlendEquation::Subtract), Op::Subtract);
        assert_eq!(
            blend_operation(BlendEquation::ReverseSubtract),
            Op::ReverseSubtract
        );
        assert_eq!(blend_operation(BlendEquation::Min), Op::Min);
        assert_eq!(blend_operation(BlendEquation::Max), Op::Max);
    }

    #[test]
    fn the_gate_matches_webgpu_pipeline_utils() {
        // The rungs 1–9 case: opaque, NormalBlending — no blend state.
        assert!(!needs_blend_state(&mode(Blending::Normal, false), false));
        // `transparent: true` alone turns blending on.
        assert!(needs_blend_state(&mode(Blending::Normal, false), true));
        // So does any non-normal blending, transparent or not.
        assert!(needs_blend_state(&mode(Blending::Additive, false), false));
        assert!(needs_blend_state(&mode(Blending::Additive, false), true));
        assert!(needs_blend_state(&mode(Blending::Custom, false), false));
        // `NoBlending` wins over everything.
        assert!(!needs_blend_state(&mode(Blending::No, false), true));
    }
}
