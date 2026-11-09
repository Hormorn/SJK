//! A grand find's blade in the game's chat (`docs/holocrons.md`, "Chat lines"; 01/11/2026,
//! Sol's request): a find of a blade skin draws the blade small after the holocron's or
//! crystal's picture, live as the Collection's swatches draw it
//! ([`crate::collection_panel::swatch::strip`]), fading with its line. While the composer
//! is open the pointer is free, and resting it on that blade shows it large in a card
//! above the line: its tier in the tier's colour, its name and the blade, as long as the
//! pointer stays on the small one.

use super::*;
use crate::console::collection_panel::swatch;
use crate::menu::sjk::Frame;
use crate::unlockables::Unlockable;
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign};

/// The console command that puts a made-up grand find of a blade in the feed.
pub(crate) const COMMAND: &str = "debug_find";
/// Its line in the command list and browser.
pub(crate) const HELP: &str =
    "Rehearse a grand find's chat line, nothing sent: debug_find [tier] [blade id or #number]";

/// `debug_find`'s words: the holocron or crystal tier (Legendary Kyber Crystal when
/// left out) and the blade (a Mythical one when left out), in either order.
pub(crate) fn parse(
    args: &[String],
) -> Result<(&'static crate::holocrons::Tier, &'static Unlockable), String> {
    let mut tier = None;
    let mut skin = None;
    for word in args {
        if let Some(found) = crate::holocrons::any(word) {
            tier = Some(found);
        } else if let Some(found) = crate::unlockables::blade_skin_named(word) {
            skin = Some(found);
        } else {
            return Err(format!(
                "{word}: not a holocron or kyber tier, nor a blade id or #number"
            ));
        }
    }
    let tier = match tier {
        Some(tier) => tier,
        None => crate::holocrons::any("kyber_legendary").ok_or("no kyber tier")?,
    };
    let skin = match skin {
        Some(skin) => skin,
        None => crate::unlockables::blade_skins_by_tier()
            .iter()
            .copied()
            .find(|skin| skin.tier == crate::unlockables::Rarity::Mythical)
            .ok_or("no Mythical blade")?,
    };
    Ok((tier, skin))
}

/// The blades' pointer targets, one a shown line (`BLADE` + its token), after the card's.
pub(super) const BLADE: u16 = 140;

/// A find's blade on show: the skin, where its small blade is and its prize's colour.
pub(super) type Shown = (&'static Unlockable, Rect, Color);

/// The large card's size against the feed's text size, the room round its blade, and
/// its tier's and name's text sizes.
const CARD: [f32; 2] = [24.0, 6.6];
const PADDING: f32 = 0.8;
const LABEL: f32 = 0.7;
const NAME: f32 = 1.1;

/// Screen pixels: the swatches take a frame's, which here is the window's own.
const SCREEN: Frame = Frame {
    s: 1.0,
    origin: [0.0, 0.0],
};

/// Draw `skin` live across `rect` from its hilt, at `alpha`.
pub(super) fn blade(list: &mut DrawList, skin: &'static Unlockable, rect: Rect, alpha: f32) {
    let skins = crate::sjk_packs::skins();
    let look = skins.get(skin.id);
    let _ = list.push(DrawCommand::PushOpacity(alpha));
    let _ = swatch::strip(
        list,
        &SCREEN,
        [rect.x, rect.y, rect.width, rect.height],
        look,
        true,
        // The file's own colours (white once left a chroma so; it greys one now).
        0.0,
    );
    let _ = list.push(DrawCommand::PopOpacity);
}

/// Where the large card of a blade at `anchor` goes: above it, from its left, kept in
/// the window (below it when there is no room above).
fn card_origin(anchor: Rect, size: [f32; 2], viewport: [f32; 2], gap: f32) -> [f32; 2] {
    let x = anchor.x.min(viewport[0] - size[0]).max(0.0);
    let above = anchor.y - gap - size[1];
    let y = if above >= 0.0 {
        above
    } else {
        (anchor.bottom() + gap).min(viewport[1] - size[1]).max(0.0)
    };
    [x, y]
}

impl ChatOverlay {
    /// The blade under the pointer, among those the feed drew this frame.
    fn hovered_blade(&self) -> Option<Shown> {
        let pointer = self.pointer_at;
        self.visible_blades
            .iter()
            .enumerate()
            .find_map(|(token, shown)| {
                let shown = (*shown)?;
                (self.ui.token_hovered(BLADE + token as u16)
                    && pointer.is_none_or(|at| shown.1.contains(at)))
                .then_some(shown)
            })
    }

    /// While the pointer rests on a find's small blade, draw it large above its line.
    pub(super) fn build_find_blade(&mut self, g: &layout::Geometry, viewport: [f32; 2]) {
        if !self.is_typing() && !self.options.history {
            return;
        }
        let Some((skin, anchor, colour)) = self.hovered_blade() else {
            return;
        };
        crate::blade_pictures::focus(skin.id);
        let u = g.font.max(4.0);
        let size = [CARD[0] * u, CARD[1] * u];
        let [x, y] = card_origin(anchor, size, viewport, 0.35 * u);
        let card = Rect::new(x, y, size[0], size[1]);
        // Text draws over every shape: the feed's under the card is left out.
        self.ui.cover(card);
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect: card,
            radius: 0.55 * u,
            color: Color::new(0.03, 0.05, 0.09, 1.0),
        });
        let _ = list.push(DrawCommand::RoundedRect {
            rect: Rect::new(x + 0.7 * u, y, size[0] - 1.4 * u, 0.17 * u),
            radius: 0.085 * u,
            color: colour,
        });
        let _ = list.push(DrawCommand::Border {
            rect: card,
            radius: 0.55 * u,
            width: 0.08 * u,
            color: Color::new(colour.r, colour.g, colour.b, 0.75),
        });
        let pad = PADDING * u;
        let label = LABEL * u;
        let name = NAME * u;
        self.ui.text_fmt_aligned(
            format_args!("{}", skin.tier.label().to_uppercase()),
            Rect::new(x + pad, y + pad * 0.7, size[0] - 2.0 * pad, label * 1.4),
            label,
            colour,
            FontWeight::Semibold,
            label * 0.12,
            TextAlign::Start,
        );
        self.ui.text_fmt_aligned(
            format_args!("{}", crate::text::Plain(skin.name)),
            Rect::new(
                x + pad,
                y + pad * 0.7 + label * 1.4,
                size[0] - 2.0 * pad,
                name * 1.3,
            ),
            name,
            Color::new(0.98, 0.985, 0.995, 1.0),
            FontWeight::Semibold,
            0.0,
            TextAlign::Start,
        );
        let top = y + pad * 0.7 + label * 1.4 + name * 1.3 + 0.2 * u;
        let blade = Rect::new(
            x + pad,
            top,
            size[0] - 2.0 * pad,
            (y + size[1] - pad * 0.8 - top).max(0.5 * u),
        );
        self::blade(self.ui.draw_list_mut(), skin, blade, 1.0);
    }
}

#[cfg(test)]
impl ChatOverlay {
    /// The first find's blade the feed drew this frame, for the world shots.
    pub(crate) fn find_blade_for_shot(&self) -> Option<Rect> {
        self.visible_blades
            .iter()
            .flatten()
            .map(|shown| shown.1)
            .next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_ui::{InputEvent, Vec2};

    /// Resting the pointer on a find's small blade, the composer open, shows its card.
    #[test]
    fn resting_on_a_finds_blade_shows_it_large() {
        let mut chat = ChatOverlay::new();
        let now = std::time::Instant::now();
        let phoenix = crate::unlockables::blade_skin("saber_phoenix").unwrap();
        let tier = crate::holocrons::any("kyber_rare").unwrap();
        chat.rehearse_find(tier, phoenix, now);
        chat.open(false);
        let font = crate::text::test_font();
        chat.build(true, &font, [1920.0, 1080.0], 1_000);
        let blade = chat.find_blade_for_shot().expect("the blade's target");
        let label = phoenix.tier.label().to_uppercase();
        let texts = |chat: &ChatOverlay| -> Vec<String> {
            chat.ui.text_runs().map(str::to_owned).collect()
        };
        assert!(!texts(&chat).contains(&label), "no card before");
        chat.pointer(InputEvent::PointerMove(Vec2::new(
            blade.x + blade.width * 0.5,
            blade.y + blade.height * 0.5,
        )));
        chat.build(true, &font, [1920.0, 1080.0], 1_000);
        let shown = texts(&chat);
        assert!(shown.contains(&label), "{shown:?}");
        assert!(shown.iter().any(|text| text == phoenix.name), "{shown:?}");
        // Closed, the feed takes no pointer: no target, no card.
        chat.close_composer();
        chat.build(true, &font, [1920.0, 1080.0], 1_000);
        assert!(chat.find_blade_for_shot().is_none());
    }

    #[test]
    fn debug_find_takes_a_tier_and_a_blade_in_either_order() {
        let words =
            |list: &[&str]| -> Vec<String> { list.iter().map(|w| (*w).to_owned()).collect() };
        let (tier, skin) = parse(&words(&["saber_phoenix", "kyber_rare"])).unwrap();
        assert_eq!((tier.id, skin.id), ("kyber_rare", "saber_phoenix"));
        let (tier, skin) = parse(&[]).unwrap();
        assert_eq!(tier.id, "kyber_legendary");
        assert_eq!(skin.tier, crate::unlockables::Rarity::Mythical);
        assert!(parse(&words(&["nonsense"])).is_err());
    }

    #[test]
    fn the_large_card_sits_above_its_blade_inside_the_window() {
        let viewport = [1920.0, 1080.0];
        let anchor = Rect::new(100.0, 600.0, 60.0, 16.0);
        assert_eq!(
            card_origin(anchor, [440.0, 112.0], viewport, 6.0),
            [100.0, 482.0]
        );
        // No room above: below it; no room at the right: pulled in.
        let high = Rect::new(1800.0, 40.0, 60.0, 16.0);
        assert_eq!(
            card_origin(high, [440.0, 112.0], viewport, 6.0),
            [1480.0, 62.0]
        );
    }
}
