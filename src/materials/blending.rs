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
    #[default]
    Normal,
    Additive,
    Subtractive,
    Multiply,
    /// `CustomBlending` — `blendSrc`/`blendDst`/`blendEquation` (and the three
    /// `*Alpha` overrides) are used verbatim.
    Custom,
}

/// `ZeroFactor` … `OneMinusConstantAlphaFactor`, used only by `CustomBlending`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstColor,
    OneMinusDstColor,
    DstAlpha,
    OneMinusDstAlpha,
    SrcAlphaSaturate,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
}

/// `AddEquation` … `MaxEquation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendEquation {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

/// `WebGPUPipelineUtils._getBlendFactor( blend )`. Note the two collapses Three
/// documents: WebGPU has no dedicated constant-*alpha* factors, so
/// `ConstantAlphaFactor` and `ConstantColorFactor` both become `Constant`.
pub fn blend_factor(blend: BlendFactor) -> wgpu::BlendFactor {
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
pub fn blend_operation(equation: BlendEquation) -> wgpu::BlendOperation {
    match equation {
        BlendEquation::Add => wgpu::BlendOperation::Add,
        BlendEquation::Subtract => wgpu::BlendOperation::Subtract,
        BlendEquation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
        BlendEquation::Min => wgpu::BlendOperation::Min,
        BlendEquation::Max => wgpu::BlendOperation::Max,
    }
}

/// The blending fields of `Material` that `_getBlending()` reads. Taken as a
/// struct rather than the whole material so the table can be unit-tested
/// without building one, and so a second material type plugs into the same
/// code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlendMode {
    pub blending: Blending,
    pub premultiplied_alpha: bool,
    /// `Material.blendSrc` / `.blendDst` / `.blendEquation`.
    pub blend_src: BlendFactor,
    pub blend_dst: BlendFactor,
    pub blend_equation: BlendEquation,
    /// `Material.blendSrcAlpha` / `.blendDstAlpha` / `.blendEquationAlpha` —
    /// `null` in Three, meaning "use the colour one".
    pub blend_src_alpha: Option<BlendFactor>,
    pub blend_dst_alpha: Option<BlendFactor>,
    pub blend_equation_alpha: Option<BlendEquation>,
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
pub fn needs_blend_state(mode: &BlendMode, transparent: bool) -> bool {
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
pub fn blending(mode: &BlendMode) -> Option<wgpu::BlendState> {
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
            Blending::No | Blending::Custom => unreachable!(),
        },
        _ => match mode.blending {
            Blending::Normal => set_blend(SrcAlpha, OneMinusSrcAlpha, One, OneMinusSrcAlpha),
            Blending::Additive => set_blend(SrcAlpha, One, One, One),
            // `error( 'WebGPURenderer: "SubtractiveBlending" requires
            // "material.premultipliedAlpha = true".' )` — no blend state.
            Blending::Subtractive | Blending::Multiply => None,
            Blending::No | Blending::Custom => unreachable!(),
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
