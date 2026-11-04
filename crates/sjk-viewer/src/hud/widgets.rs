//! Data-bound HUD widget emission into the renderer-neutral draw list.

use super::{HudLayout, WidgetData, text_id};
use sjk_ui::{
    Color, DrawCommand, DrawList, FontWeight, Gradient, HudDataSource, HudWidget, HudWidgetKind,
    Rect, TextAlign, TextId, TextOverflow, TextureId, Theme,
};

pub(super) struct EmitContext<'a> {
    pub(super) data: &'a WidgetData<'a>,
    /// Live presentation switches.
    pub(super) family: super::family::Policy,
    /// Sampled crosshair and telemetry placement.
    pub(super) targeting: super::targeting::Policy,
    /// Physical viewport.
    pub(super) viewport: [f32; 2],
    pub(super) dpi_scale: f32,
    /// Hero overlay scale, independent of the compact HUD's historical clamp.
    pub(super) hero_scale: f32,
    pub(super) low_health: bool,
    pub(super) low_ammo: bool,
    pub(super) pulse: f32,
    pub(super) crosshair_alpha: f32,
    pub(super) crosshair_teammate: bool,
    pub(super) lagometer: &'a sjk_client::LagometerSamples,
    /// What `cg_lagometer` 2 and 3 add ([`super::lagometer`]).
    pub(super) lagometer_readout: &'a super::lagometer::Readout,
}

pub(super) fn emit(
    draw_list: &mut DrawList,
    theme: Theme,
    widget: &HudWidget,
    rect: Rect,
    context: &EmitContext<'_>,
    output: &mut HudLayout,
) {
    let binding = widget.binding.as_deref();
    let mut foreground = widget.style.foreground.unwrap_or(theme.foreground);
    if (matches!(binding, Some("health_value" | "health_ratio")) && context.low_health)
        || (matches!(binding, Some("ammo_value")) && context.low_ammo)
    {
        foreground = theme.critical;
        foreground.a *= context.pulse;
    }
    if binding == Some("weapon_value") {
        foreground.a *= context.data.weapon_alpha;
    }
    // A saber-style label with no colour of its own takes the style's.
    if binding == Some("style_value") && widget.style.foreground.is_none() {
        if let Some(style) = context.data.saber_style {
            foreground = super::radial::saber_style_color(style);
        }
    }
    let radius = widget.style.radius.unwrap_or(theme.radii.sm);
    match widget.kind {
        HudWidgetKind::Panel => emit_panel(draw_list, theme, widget, rect, radius),
        HudWidgetKind::Text if binding == Some("crosshair_name") => {
            emit_crosshair_name(draw_list, theme, widget, rect, context)
        }
        HudWidgetKind::Text => {
            let Some(text) = text_id(binding) else { return };
            let align = if let Some(align) = widget.style.align {
                align
            } else if widget.style.centered
                || matches!(
                    binding,
                    Some(
                        "weapon_text"
                            | "ammo_text"
                            | "weapon_value"
                            | "ammo_value"
                            | "match_timer"
                            | "warmup_text"
                            | "connection_interrupted"
                    )
                )
            {
                TextAlign::Center
            } else if binding == Some("style_value") {
                TextAlign::End
            } else {
                TextAlign::Start
            };
            let _ = draw_list.push(DrawCommand::Text {
                rect,
                text,
                size: theme.typography.value
                    * widget.style.type_scale.unwrap_or(1.0)
                    * context.dpi_scale,
                color: foreground,
                align,
                overflow: TextOverflow::Ellipsis,
                weight: widget.style.weight.unwrap_or(FontWeight::Regular),
                letter_spacing: widget.style.letter_spacing.unwrap_or(0.0),
            });
        }
        HudWidgetKind::Meter if widget.style.segments.is_some_and(|cells| cells > 1) => {
            let ratio = context
                .data
                .number(binding.unwrap_or_default())
                .unwrap_or(0.0);
            let track = widget
                .style
                .background
                .unwrap_or(Color::new(1.0, 1.0, 1.0, 0.20));
            for (cell, alpha) in segment_cells(rect, widget.style.segments.unwrap_or(1), ratio) {
                let _ = draw_list.push(DrawCommand::RoundedRect {
                    rect: cell,
                    radius: radius.min(cell.height * 0.5),
                    color: track,
                });
                if alpha > 0.0 {
                    let _ = draw_list.push(DrawCommand::RoundedRect {
                        rect: cell,
                        radius: radius.min(cell.height * 0.5),
                        color: Color::new(
                            foreground.r,
                            foreground.g,
                            foreground.b,
                            foreground.a * alpha,
                        ),
                    });
                }
            }
        }
        HudWidgetKind::Meter => {
            let ratio = context
                .data
                .number(binding.unwrap_or_default())
                .unwrap_or(0.0);
            let _ = draw_list.push(DrawCommand::RoundedRect {
                rect,
                radius,
                color: widget
                    .style
                    .background
                    .unwrap_or(Color::new(1.0, 1.0, 1.0, 0.20)),
            });
            let _ = draw_list.push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    rect.x,
                    rect.y,
                    rect.width * ratio.clamp(0.0, 1.0),
                    rect.height,
                ),
                radius,
                color: foreground,
            });
            // Over the maximum (overheal, overshield): an inner band in a deeper shade.
            let over = (ratio - 1.0).clamp(0.0, 1.0);
            if over > 0.0 {
                let band = super::nameplate_math::overflow_band(rect);
                let _ = draw_list.push(DrawCommand::RoundedRect {
                    rect: Rect::new(band.x, band.y, band.width * over, band.height),
                    radius: radius.min(band.height * 0.5),
                    color: super::nameplate_math::saturated(foreground),
                });
            }
            let packed = [rect.x, rect.y, rect.right(), rect.bottom()];
            match binding {
                Some("health_ratio") => output.health_bar = packed,
                Some("armor_ratio") => output.armor_bar = packed,
                Some("force_ratio") => output.force_bar = packed,
                _ => {}
            }
        }
        HudWidgetKind::Repeater if binding == Some("vote_panel") => {
            super::vote::emit(draw_list, theme, widget, rect, context)
        }
        // The kill feed is drawn by `super::kill_feed` under the top right's other
        // readouts, not by a layout widget; one left in a `hud.json` draws nothing.
        HudWidgetKind::Repeater if binding == Some("kill_feed") => {}
        HudWidgetKind::Repeater if binding == Some("lagometer") => {
            // Scaling changes presentation, not the number of retained samples shown.
            let columns = rect.width.floor().clamp(1.0, 64.0) as usize;
            let rect = context.targeting.lagometer_rect(rect, context.viewport);
            super::lagometer::emit(
                draw_list,
                rect,
                &super::lagometer::Graph {
                    samples: context.lagometer,
                    readout: context.lagometer_readout,
                    mode: context.targeting.lagometer_mode(),
                    columns,
                    // The numbers grow with the HUD, as its other text, and the graph.
                    text_scale: context.dpi_scale * context.targeting.lagometer_scale(),
                    screen: context.viewport,
                },
                &mut super::lagometer::panel(theme, widget),
            )
        }
        // The team overlay ("team_rows") was removed (Sol's request, 13/10/2026): team
        // games show teammates by their logos and outlines instead. A repeater left in
        // a `hud.json` with that or another binding draws nothing.
        HudWidgetKind::Repeater => {}
        HudWidgetKind::Arc => super::radial::emit(draw_list, theme, widget, rect, context),
        HudWidgetKind::Image => {
            let _ = draw_list.push(DrawCommand::TexturedQuad {
                rect,
                texture: TextureId(0),
                color: foreground,
            });
        }
    }
}

/// The cells of a segmented meter in `rect` and how lit each is for `ratio`: `count`
/// cells as wide as the meter allows with a gap of half its height between them, the
/// ones the ratio covers at 1, the one it part covers at that part, the rest at 0.
pub(super) fn segment_cells(
    rect: Rect,
    count: u8,
    ratio: f32,
) -> impl Iterator<Item = (Rect, f32)> {
    let count = count.max(1);
    let cells = f32::from(count);
    let gap = (rect.height * 0.5).min(rect.width / (cells * 2.0));
    let width = ((rect.width - gap * (cells - 1.0)) / cells).max(0.0);
    let covered = if ratio.is_finite() {
        ratio.clamp(0.0, 1.0) * cells
    } else {
        0.0
    };
    (0..count).map(move |index| {
        let index = f32::from(index);
        let cell = Rect::new(rect.x + index * (width + gap), rect.y, width, rect.height);
        (cell, (covered - index).clamp(0.0, 1.0))
    })
}

/// Line height of the crosshair name at the HUD's 1080-line reference, in
/// physical pixels. `CG_DrawCrosshairNames` (`cg_draw.c`) draws the name with
/// `CG_DrawProportionalString`: `FONT_MEDIUM` (`ergoec`, point size 20) at
/// scale 1.0 in the 640x480 virtual screen, so 20 * 1080 / 480 = 45 px at 1080p.
const CROSSHAIR_NAME_LINE: f32 = 20.0 * 1_080.0 / 480.0;

/// Crosshair name line height for a layout's `type_scale` and the HUD's
/// resolution and `cg_hudScale` factor, so it scales like the other HUD text.
fn crosshair_name_size(type_scale: f32, dpi_scale: f32) -> f32 {
    CROSSHAIR_NAME_LINE * type_scale * dpi_scale
}

fn emit_crosshair_name(
    draw_list: &mut DrawList,
    theme: Theme,
    widget: &HudWidget,
    rect: Rect,
    context: &EmitContext<'_>,
) {
    let mut color = if !context.family.name_colors {
        theme.foreground
    } else if context.crosshair_teammate {
        Color::new(0.38, 1.0, 0.58, 1.0)
    } else {
        Color::new(1.0, 0.38, 0.34, 1.0)
    };
    color.a = context.crosshair_alpha * context.family.name_alpha;
    let _ = draw_list.push(DrawCommand::Text {
        rect,
        text: TextId(310),
        size: crosshair_name_size(widget.style.type_scale.unwrap_or(1.0), context.dpi_scale),
        color,
        align: TextAlign::Center,
        overflow: TextOverflow::Ellipsis,
        weight: FontWeight::Semibold,
        letter_spacing: 0.0,
    });
}

pub(super) fn emit_panel(
    draw_list: &mut DrawList,
    theme: Theme,
    widget: &HudWidget,
    rect: Rect,
    radius: f32,
) {
    let start = widget.style.background.unwrap_or(theme.surface);
    let _ = draw_list.push(match widget.style.background_end {
        Some(end) => DrawCommand::GradientRect {
            rect,
            radius,
            gradient: Gradient {
                start,
                end,
                vertical: true,
            },
        },
        None => DrawCommand::RoundedRect {
            rect,
            radius,
            color: start,
        },
    });
    if let Some(color) = widget.style.border {
        let _ = draw_list.push(DrawCommand::Border {
            rect,
            radius,
            width: widget.style.border_width.unwrap_or(1.0),
            color,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{Rect, crosshair_name_size, segment_cells};

    /// The HUD's resolution factor before `cg_hudScale`, as `HudOverlay::layout`.
    fn dpi(height: f32) -> f32 {
        (height / 1_080.0).clamp(2.0 / 3.0, 4.0 / 3.0)
    }

    #[test]
    fn crosshair_name_matches_stock_virtual_screen_size() {
        // Stock: ergoec point size 20 in the 480-line virtual screen.
        for height in [720.0_f32, 1_080.0, 1_440.0] {
            let stock = 20.0 * height / 480.0;
            assert!((crosshair_name_size(1.0, dpi(height)) - stock).abs() < 1e-3);
        }
    }

    #[test]
    fn crosshair_name_follows_layout_and_hud_scale() {
        assert!((crosshair_name_size(0.9, 1.0) - 40.5).abs() < 1e-3);
        assert!((crosshair_name_size(1.0, 1.0 * 1.5) - 67.5).abs() < 1e-3);
    }

    /// Ten cells share the meter with gaps of half its height, and the ratio lights
    /// them in order, the part-covered one faded.
    #[test]
    fn segmented_meters_light_their_cells_in_order() {
        let cells: Vec<_> = segment_cells(Rect::new(0.0, 0.0, 145.0, 10.0), 10, 0.25).collect();
        assert_eq!(cells.len(), 10);
        assert_eq!(cells[0].0, Rect::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(cells[9].0.right(), 145.0);
        let lit: Vec<_> = cells.iter().map(|(_, alpha)| *alpha).collect();
        assert_eq!(lit[..4], [1.0, 1.0, 0.5, 0.0]);
        assert!(
            segment_cells(Rect::new(0.0, 0.0, 145.0, 10.0), 10, f32::NAN)
                .all(|(_, alpha)| alpha == 0.0)
        );
        assert!(
            segment_cells(Rect::new(0.0, 0.0, 145.0, 10.0), 10, 2.0).all(|(_, alpha)| alpha == 1.0)
        );
    }
}
