//! Collection's static covers, ownership counts and authoritative equip action.
//!
//! Since 05/11/2026 (headshotsmcgee117's idea: duplicates had no use) a banner held
//! breaks down for kyber shards on a hub whose economy says `banner_shatter`, through
//! the shaders' `POST /v1/unlocks/shatter`: a duplicate, or the last copy on a hub that
//! also says `shatter_last` (it leaves the collection, and the profile when worn). Each
//! asks twice, as the Forge does: the first press arms the button, a second within five
//! seconds sends it.
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use crate::profile_card::banner::{self, ALL, Banner};
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The Banners tab's Break down: what the hub takes, as the last frame read it, and the
/// press armed or sent.
#[derive(Debug, Default)]
pub(super) struct Break {
    /// The shards a copy gives by tier index, when the hub breaks banners down and the
    /// player may trade now.
    yields: Option<[Option<u64>; 5]>,
    /// Whether the hub breaks a last copy down too (`shatter_last`).
    last: bool,
    /// Why breaking down is closed while the hub takes banners (opening for staff, an
    /// update needed).
    why_not: Option<String>,
    /// The last trade's outcome.
    trade: Option<sjk_identity::TradeOutcome>,
    /// The banner armed, and when.
    armed: Option<(&'static str, Instant)>,
    /// The banner sent, after the outcome serial it waits past.
    sent: Option<(&'static str, u64)>,
}

impl Break {
    /// Read what the hub takes from the snapshot (`None` offline: nothing breaks down, and
    /// the last outcome is kept so a sent banner still finds its answer).
    fn read(&mut self, snapshot: Option<&sjk_identity::Snapshot>) {
        let Some(snapshot) = snapshot else {
            self.yields = None;
            self.why_not = None;
            return;
        };
        let economy = snapshot.economy.as_ref();
        let takes = economy.is_some_and(|tables| tables.can(sjk_identity::BANNER_SHATTER_ABILITY));
        let may = crate::economy::may_act(snapshot);
        self.yields = (takes && may).then(|| {
            std::array::from_fn(|tier| economy.and_then(|tables| tables.banner_yield(tier)))
        });
        self.last = economy.is_some_and(|tables| tables.can("shatter_last"));
        self.why_not = (takes && !may).then(|| match crate::economy::update_to(snapshot) {
            Some(min) => {
                format!("Update SJK to break banners down: this needs SJK {min} or newer.")
            }
            None => crate::terms::say("Banners break down once holocrons open: soon.").into_owned(),
        });
        self.trade = snapshot.trade.clone();
    }

    fn serial(&self) -> u64 {
        self.trade.as_ref().map_or(0, |trade| trade.serial)
    }

    /// Whether a banner sent waits for the hub's answer.
    fn waiting(&self) -> bool {
        self.sent.is_some_and(|(_, asked)| self.serial() <= asked)
    }

    /// The hub's answer to the banner sent from here, once it came.
    fn answer(&self) -> Option<(&'static str, &sjk_identity::TradeOutcome)> {
        let (id, asked) = self.sent?;
        self.trade
            .as_ref()
            .filter(|trade| trade.serial > asked && trade.id == id)
            .map(|trade| (id, trade))
    }

    fn armed(&self, id: &str, now: Instant) -> bool {
        self.armed.is_some_and(|(armed, at)| {
            armed == id && now.saturating_duration_since(at) < forge::CONFIRM
        })
    }

    /// The shards a copy of `banner` gives, when it breaks down now.
    fn gives(&self, banner: &Banner) -> Option<u64> {
        if banner.id.is_empty() {
            return None;
        }
        self.yields?.get(banner.tier as usize).copied().flatten()
    }
}

impl Panel {
    /// The chosen banner's Break down at `now`: the first press arms it, a second within
    /// five seconds sends it. Nothing for the neutral cover, a banner not held, a last copy
    /// the hub keeps, or while another waits.
    pub(super) fn break_banner(&mut self, now: Instant) -> PanelAction {
        let Some(chosen) = ALL.get(self.banner) else {
            return PanelAction::None;
        };
        let count = self.banner_counts.get(self.banner).copied().unwrap_or(0);
        let parts = &mut self.banner_break;
        if parts.waiting()
            || parts.gives(chosen).is_none()
            || count == 0
            || (count == 1 && !parts.last)
        {
            return PanelAction::None;
        }
        if !parts.armed(chosen.id, now) {
            parts.armed = Some((chosen.id, now));
            return PanelAction::None;
        }
        parts.armed = None;
        parts.sent = Some((chosen.id, parts.serial()));
        if count == 1 {
            PanelAction::ShatterLast(chosen.id)
        } else {
            PanelAction::Shatter(chosen.id)
        }
    }

    /// The chosen banner's Break down button and what the hub answered, under Equip.
    fn draw_break(&mut self, frame: &Frame, chosen: &Banner) {
        let now = Instant::now();
        let count = self.banner_counts[self.banner];
        let parts = &self.banner_break;
        let gives = parts.gives(chosen);
        let mut line: Option<(String, sjk_ui::Color)> = None;
        if let Some((id, outcome)) = parts.answer() {
            let name = banner::get(id).map_or(id, |banner| banner.name);
            let colour = if outcome.done() {
                color::GOLD_BRIGHT
            } else {
                color::EMBER
            };
            line = Some((crate::economy::outcome_words(outcome, name), colour));
        }
        if let Some(shards) = gives.filter(|_| count > 0) {
            let shards = crate::economy::shards_text(shards);
            let armed = parts.armed(chosen.id, now);
            let label = if parts.waiting() {
                "Breaking down...".to_owned()
            } else if count == 1 && !parts.last {
                "Your last copy stays".to_owned()
            } else if armed && count == 1 {
                format!("Confirm: lose {} for good", chosen.name)
            } else if armed {
                "Confirm: break down a copy".to_owned()
            } else if count == 1 {
                format!("Break down last copy  +{shards}")
            } else {
                format!("Break down a copy  +{shards}")
            };
            let usable = !parts.waiting() && (count > 1 || parts.last);
            kit::button(
                &mut self.ui,
                frame,
                [1330.0, 740.0, 460.0, 54.0],
                &label,
                false,
                usable,
                armed,
                BANNER_BREAK,
            );
            if line.is_none() && armed && count == 1 {
                line = Some((
                    format!(
                        "Your only {} leaves your collection{}.",
                        chosen.name,
                        if chosen.id == self.banner_current {
                            " and your profile"
                        } else {
                            ""
                        }
                    ),
                    color::EMBER,
                ));
            }
        } else if count > 0
            && let Some(why) = parts.why_not.as_deref()
            && line.is_none()
        {
            line = Some((why.to_owned(), color::MUTED));
        }
        if let Some((words, colour)) = line {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(1050.0, 800.0, 740.0, 26.0),
                18.0 * frame.s,
                colour,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    pub(super) fn wear_banner(&self) -> PanelAction {
        if !self.banner_owned.get(self.banner).copied().unwrap_or(false) {
            return PanelAction::None;
        }
        let Some(chosen) = ALL.get(self.banner) else {
            return PanelAction::None;
        };
        if chosen.id.is_empty() && self.banner_current.is_empty() {
            return PanelAction::None;
        }
        PanelAction::Banner(if chosen.id == self.banner_current {
            ""
        } else {
            chosen.id
        })
    }

    pub(super) fn banners(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let snapshot = inputs.snapshot.filter(|_| inputs.enabled);
        let profile = snapshot.and_then(|snapshot| snapshot.me.as_ref());
        let online =
            snapshot.is_some_and(|snapshot| snapshot.status == sjk_identity::Status::Online);
        self.banner = self.banner.min(ALL.len() - 1);
        self.banner_current = banner::equipped(profile);
        self.banner_owned = std::array::from_fn(|index| {
            online && profile.is_some() && banner::count(profile, ALL[index].id) > 0
        });
        self.banner_counts = std::array::from_fn(|index| {
            if online && profile.is_some() && !ALL[index].id.is_empty() {
                banner::count(profile, ALL[index].id)
            } else {
                0
            }
        });
        self.banner_break.read(online.then_some(snapshot).flatten());
        self.lead(
            frame,
            "Profile banners",
            "Collectible profile covers. Preview any cover; equip one you own.",
        );
        for (index, cover) in ALL.iter().enumerate() {
            let x = 96.0 + (index % 2) as f32 * 440.0;
            let y = 304.0 + (index / 2) as f32 * 200.0;
            let rect = frame.rect(x, y, 412.0, 114.0);
            banner::draw(&mut self.ui, rect, cover.id);
            let count = banner::count(profile, cover.id);
            if count == 0 {
                let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
                    rect,
                    color: color::alpha(color::SPACE, 0.55),
                });
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 8.0 * frame.s,
                width: if index == self.banner { 2.0 } else { 1.0 } * frame.s,
                color: if index == self.banner {
                    color::GOLD_BRIGHT
                } else {
                    color::alpha(color::HOLO, 0.3)
                },
            });
            self.ui
                .hit_region(BANNER_BASE + index as u16, frame.rect(x, y, 412.0, 170.0));
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", cover.name),
                frame.rect(x, y + 124.0, 290.0, 26.0),
                22.0 * frame.s,
                color::TEXT,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{}  x{}{}",
                    cover.tier.label(),
                    count,
                    if cover.id == self.banner_current {
                        "  Equipped"
                    } else {
                        ""
                    }
                ),
                frame.rect(x, y + 152.0, 412.0, 24.0),
                16.0 * frame.s,
                cover.tier.colour(),
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let chosen = &ALL[self.banner];
        banner::draw(
            &mut self.ui,
            frame.rect(1050.0, 304.0, 740.0, 206.0),
            chosen.id,
        );
        let name = profile.map_or("Your profile", |profile| profile.name.as_str());
        crate::profile_card::avatar(
            &mut self.ui,
            frame.point(1120.0, 506.0),
            52.0 * frame.s,
            &crate::profile_card::Avatar {
                key_id: profile.map_or("", |profile| profile.key_id.as_str()),
                version: profile.map_or("", |profile| profile.avatar.as_str()),
                name,
                verified: profile.is_some_and(|profile| profile.verified),
                tick: profile.map_or(sjk_identity::Tick::None, |profile| profile.tick),
                trust: profile.and_then(|profile| profile.trust),
                preview: false,
                lit: false,
            },
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(1200.0, 476.0, 570.0, 50.0),
            30.0 * frame.s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", chosen.name),
            frame.rect(1050.0, 600.0, 740.0, 48.0),
            38.0 * frame.s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{} cover  |  {} copies",
                chosen.tier.label(),
                banner::count(profile, chosen.id)
            ),
            frame.rect(1050.0, 660.0, 740.0, 28.0),
            20.0 * frame.s,
            chosen.tier.colour(),
            FontWeight::Regular,
            TextAlign::Start,
        );
        let usable = self.banner_owned[self.banner]
            && (!chosen.id.is_empty() || !self.banner_current.is_empty());
        let label = if !online {
            "Connect to equip"
        } else if banner::count(profile, chosen.id) == 0 {
            "Locked"
        } else if chosen.id == self.banner_current && !chosen.id.is_empty() {
            "Unequip"
        } else if chosen.id.is_empty() && self.banner_current.is_empty() {
            "Equipped"
        } else {
            "Equip"
        };
        kit::button(
            &mut self.ui,
            frame,
            [1050.0, 740.0, 260.0, 54.0],
            label,
            true,
            usable,
            false,
            BANNER_WEAR,
        );
        self.draw_break(frame, chosen);
        let notice = snapshot
            .and_then(|snapshot| snapshot.notice.as_deref())
            .unwrap_or("Banners are cosmetic. The neutral SJK cover is always available.");
        for (row, words) in crate::menu::sjk::wrap(notice, 68).enumerate() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(1050.0, 830.0 + row as f32 * 26.0, 740.0, 26.0),
                18.0 * frame.s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn locked_selection_never_equips_and_owned_selection_toggles() {
        let mut panel = Panel::new();
        panel.banner = 1;
        assert!(matches!(panel.wear_banner(), PanelAction::None));
        panel.banner_owned[1] = true;
        assert!(matches!(
            panel.wear_banner(),
            PanelAction::Banner("banner_jedi_archives")
        ));
        panel.banner_current = ALL[1].id;
        assert!(matches!(panel.wear_banner(), PanelAction::Banner("")));
        panel.banner = 2;
        assert!(matches!(panel.wear_banner(), PanelAction::None));
        panel.banner = 0;
        panel.banner_owned[0] = true;
        assert!(matches!(panel.wear_banner(), PanelAction::Banner("")));
        panel.banner_current = "";
        assert!(matches!(panel.wear_banner(), PanelAction::None));
    }
    fn outcome(serial: u64, id: &str, message: &str) -> sjk_identity::TradeOutcome {
        sjk_identity::TradeOutcome {
            serial,
            trade: sjk_identity::Trade::Shatter,
            id: id.to_owned(),
            amount: 5,
            copies: 1,
            message: message.to_owned(),
        }
    }

    /// A duplicate breaks down after two presses within five seconds; the last copy only
    /// on a hub that takes it; nothing for the neutral cover, a banner not held, a hub
    /// that does not break banners, or while one waits for the hub.
    #[test]
    fn a_banner_breaks_down_on_the_second_press_and_the_last_copy_only_when_the_hub_says() {
        let now = Instant::now();
        let mut panel = Panel::new();
        panel.banner = 1;
        panel.banner_counts[1] = 3;
        // A hub that does not break banners down.
        assert!(matches!(panel.break_banner(now), PanelAction::None));
        assert!(panel.banner_break.armed.is_none());
        panel.banner_break.yields = Some([None, Some(5), Some(12), Some(30), Some(75)]);
        assert_eq!(panel.banner_break.gives(&ALL[1]), Some(5));
        assert_eq!(panel.banner_break.gives(&ALL[0]), None, "the neutral cover");
        assert!(matches!(panel.break_banner(now), PanelAction::None), "arms");
        // Too late: arms again.
        let late = now + forge::CONFIRM + Duration::from_millis(1);
        assert!(matches!(panel.break_banner(late), PanelAction::None));
        assert!(matches!(
            panel.break_banner(late + Duration::from_secs(1)),
            PanelAction::Shatter("banner_jedi_archives")
        ));
        // Waiting for the hub: nothing arms or sends.
        assert!(panel.banner_break.waiting());
        assert!(matches!(panel.break_banner(late), PanelAction::None));
        assert!(matches!(panel.break_banner(late), PanelAction::None));
        // Another trade's answer is not this one's.
        panel.banner_break.trade = Some(outcome(1, "saber_tidal", ""));
        assert!(panel.banner_break.answer().is_none());
        panel.banner_break.trade = Some(outcome(1, "banner_jedi_archives", ""));
        assert!(!panel.banner_break.waiting());
        assert_eq!(
            panel.banner_break.answer().map(|(id, _)| id),
            Some("banner_jedi_archives")
        );
        // The last copy: kept by a hub without shatter_last, broken down by one with it.
        panel.banner_counts[1] = 1;
        for _ in 0..2 {
            assert!(matches!(panel.break_banner(now), PanelAction::None));
        }
        panel.banner_break.last = true;
        assert!(matches!(panel.break_banner(now), PanelAction::None));
        assert!(matches!(
            panel.break_banner(now),
            PanelAction::ShatterLast("banner_jedi_archives")
        ));
        // A banner not held, and the neutral cover.
        panel.banner_break.trade = Some(outcome(2, "banner_jedi_archives", ""));
        panel.banner = 2;
        for _ in 0..2 {
            assert!(matches!(panel.break_banner(now), PanelAction::None));
        }
        panel.banner = 0;
        panel.banner_counts[0] = 1;
        for _ in 0..2 {
            assert!(matches!(panel.break_banner(now), PanelAction::None));
        }
    }

    #[test]
    fn the_offline_picker_previews_every_cover_but_cannot_equip() {
        let inputs = super::super::tests::inputs(Holdings::Known(&[]), "");
        let panel = super::super::tests::drawn(Tab::Banners, &inputs);
        assert!(!panel.overflowed());
        for index in 0..ALL.len() {
            assert!(panel.ui.rect_for(BANNER_BASE + index as u16).is_some());
        }
        assert!(matches!(panel.wear_banner(), PanelAction::None));
    }
}
