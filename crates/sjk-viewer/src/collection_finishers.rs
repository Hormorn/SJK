//! Collection > Shaders, Finishers (Sol, 04/11/2026: "a new finisher, make a category for
//! it"): the kinds' pills over the Shaders tab switch between the saber's shaders and the
//! finishers (`crate::finishers`), what a saber kill does to the body. Each finisher is a
//! card: its name in its tier's colour, its tier and maker, what it does, and Equip or
//! Unequip while the player holds it (`cg_saberFinisher`), how to get it otherwise. One is
//! worn at a time, with any blade.

use super::view::{LEAD_Y, LEFT_X};
use super::*;
use crate::finishers::{CATALOGUE, Finisher};
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use crate::symbols;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The kinds' pills: the saber's shaders and the finishers, clickable.
pub(super) const SABER_KIND_TOKEN: u16 = 8_500;
pub(super) const FINISHERS_KIND_TOKEN: u16 = 8_501;
/// Each finisher's Equip or Unequip, by its place in [`CATALOGUE`].
pub(super) const FINISHER_WEAR_BASE: u16 = 8_510;
const _: () = assert!(CATALOGUE.len() <= 40);

/// The cards' column and size (frame pixels).
const CARD_X: f32 = LEFT_X - 12.0;
const CARD_TOP: f32 = 400.0;
const CARD_WIDTH: f32 = 860.0;
const CARD_HEIGHT: f32 = 168.0;
const CARD_GAP: f32 = 16.0;

/// Whether the player holds finisher `id` (or `debug_collectibles` shows everything as
/// held).
fn held(inputs: &Inputs<'_>, id: &str) -> bool {
    inputs.holdings.debug() || inputs.holdings.unlock(id).is_some()
}

/// The finisher worn: the setting's, while it is held.
fn worn(inputs: &Inputs<'_>) -> Option<&'static str> {
    crate::finishers::own(inputs.finisher, |id| held(inputs, id))
}

impl Panel {
    /// The kinds' pills over the Shaders tab: the saber's shaders (held of total), the
    /// finishers (held of total), both clickable to switch the page, and the body's,
    /// still to come.
    pub(super) fn kind_pills(
        &mut self,
        frame: &Frame,
        saber: (usize, usize),
        finishers: (usize, usize),
    ) {
        let s = frame.s;
        let x = LEFT_X + 560.0;
        let y = LEAD_Y - 2.0;
        let pills: [(&str, String, Option<u16>, bool); 3] = [
            (
                "Saber",
                format!("{}/{}", saber.0, saber.1),
                Some(SABER_KIND_TOKEN),
                !self.finishers_shown,
            ),
            (
                "Finishers",
                format!("{}/{}", finishers.0, finishers.1),
                Some(FINISHERS_KIND_TOKEN),
                self.finishers_shown,
            ),
            ("Body", "soon".to_owned(), None, false),
        ];
        let mut left = x;
        for (name, count, token, lit) in pills {
            let width = if name == "Finishers" { 168.0 } else { 132.0 };
            let rect = frame.rect(left, y, width, 36.0);
            let hovered = token.is_some_and(|token| self.ui.token_hovered(token));
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 18.0 * s,
                width: 1.2 * s,
                color: if lit {
                    color::GOLD
                } else if hovered {
                    color::alpha(color::HOLO, 0.6)
                } else {
                    color::alpha(color::HOLO, 0.25)
                },
            });
            let active = token.is_some();
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(left + 18.0, y + 5.0, width - 70.0, 26.0),
                19.0 * s,
                if lit {
                    color::GOLD_BRIGHT
                } else if active {
                    color::TEXT
                } else {
                    color::QUIET
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{count}"),
                frame.rect(left + width - 64.0, y + 7.0, 50.0, 22.0),
                14.0 * s,
                if lit { color::GOLD } else { color::QUIET },
                FontWeight::Regular,
                TextAlign::End,
            );
            if let Some(token) = token {
                self.ui.hit_region(token, rect);
            }
            left += width + 12.0;
        }
    }

    /// The finishers page in place of the shaders, with the saber's counts for its pill.
    pub(super) fn finishers_page(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        saber: (usize, usize),
    ) {
        let owned = CATALOGUE.iter().filter(|f| held(inputs, f.id)).count();
        let headline = format!("{owned} of {} owned", CATALOGUE.len());
        self.lead(
            frame,
            &headline,
            "What your saber kills do to the body, for every SJK player on your server. One at a time, with any blade.",
        );
        self.kind_pills(frame, saber, (owned, CATALOGUE.len()));
        let worn = worn(inputs);
        for (index, finisher) in CATALOGUE.iter().enumerate() {
            let y = CARD_TOP + index as f32 * (CARD_HEIGHT + CARD_GAP);
            self.finisher_card(frame, inputs, index, finisher, worn == Some(finisher.id), y);
        }
    }

    /// One finisher's card at `y`.
    fn finisher_card(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        index: usize,
        finisher: &'static Finisher,
        is_worn: bool,
        y: f32,
    ) {
        let s = frame.s;
        let held = held(inputs, finisher.id);
        let tier = finisher.tier.colour();
        kit::band(&mut self.ui, frame, [CARD_X, y, CARD_WIDTH, CARD_HEIGHT]);
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: frame.rect(CARD_X, y, CARD_WIDTH, CARD_HEIGHT),
            radius: 14.0 * s,
            width: 1.4 * s,
            color: color::alpha(tier, if held { 0.9 } else { 0.4 }),
        });
        let left = CARD_X + 24.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", finisher.name),
            frame.rect(left, y + 16.0, 420.0, 34.0),
            26.0 * s,
            tier,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let state = if is_worn {
            format!("{}  Worn", symbols::SABER)
        } else if held {
            format!("{}  Yours", symbols::TICK)
        } else {
            "Not owned".to_owned()
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}  ·  by {}  ·  {state}",
                finisher.tier.label(),
                finisher.smith
            ),
            frame.rect(left, y + 52.0, 560.0, 22.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let words = if held {
            finisher.description.to_owned()
        } else {
            format!("{} {}", finisher.description, finisher.how_to_get)
        };
        super::shaders::paragraph(
            &mut self.ui,
            frame,
            &words,
            [left, y + 82.0, CARD_WIDTH - 260.0],
            68.0,
            3,
            15.0,
            color::TEXT,
            FontWeight::Regular,
        );
        if held {
            let label = if is_worn { "Unequip" } else { "Equip" };
            kit::button(
                &mut self.ui,
                frame,
                [
                    CARD_X + CARD_WIDTH - 204.0,
                    y + CARD_HEIGHT - 64.0,
                    180.0,
                    44.0,
                ],
                label,
                !is_worn,
                true,
                false,
                FINISHER_WEAR_BASE + index as u16,
            );
        }
    }

    /// Show the finishers page, for the world shots.
    #[cfg(test)]
    pub(crate) fn show_finishers(&mut self) {
        self.finishers_shown = true;
    }

    /// A click on the finishers' pills or cards, if it was one: the page switched, or the
    /// finisher worn or taken off.
    pub(super) fn finishers_click(&mut self, token: u16) -> Option<PanelAction> {
        match token {
            SABER_KIND_TOKEN => {
                self.finishers_shown = false;
                Some(PanelAction::None)
            }
            FINISHERS_KIND_TOKEN => {
                self.finishers_shown = true;
                Some(PanelAction::None)
            }
            token if self.finishers_shown => {
                let finisher =
                    CATALOGUE.get(usize::from(token.checked_sub(FINISHER_WEAR_BASE)?))?;
                Some(PanelAction::WearFinisher(
                    if self.finisher_worn == Some(finisher.id) {
                        ""
                    } else {
                        finisher.id
                    },
                ))
            }
            _ => None,
        }
    }
}
