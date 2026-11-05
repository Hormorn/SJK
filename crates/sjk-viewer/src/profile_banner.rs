//! Original static profile-cover art and its public catalogue, mirrored by the hub.
//! Shapes use fixed-size draw commands: no downloads, locks or per-frame allocations.
use crate::menu_widgets::MenuCanvas;
use crate::unlockables::Rarity;
use sjk_ui::{Color, DrawCommand, Gradient, Rect};

/// A permanent banner identity; the empty id is the always-available neutral cover.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Banner {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) tier: Rarity,
    art: Art,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Art {
    Neutral,
    Archives,
    Sunset,
    Eclipse,
    Hyperspace,
    Nebula,
}
/// First edition, in Collection order. Keep ids/names/tiers aligned with hub unlocks.
pub(crate) const ALL: [Banner; 6] = [
    Banner {
        id: "",
        name: "SJK",
        tier: Rarity::Common,
        art: Art::Neutral,
    },
    Banner {
        id: "banner_jedi_archives",
        name: "Jedi Archives",
        tier: Rarity::Uncommon,
        art: Art::Archives,
    },
    Banner {
        id: "banner_binary_sunset",
        name: "Binary Sunset",
        tier: Rarity::Rare,
        art: Art::Sunset,
    },
    Banner {
        id: "banner_sith_eclipse",
        name: "Sith Eclipse",
        tier: Rarity::Rare,
        art: Art::Eclipse,
    },
    Banner {
        id: "banner_hyperspace",
        name: "Hyperspace",
        tier: Rarity::Legendary,
        art: Art::Hyperspace,
    },
    Banner {
        id: "banner_living_nebula",
        name: "Living Nebula",
        tier: Rarity::Mythical,
        art: Art::Nebula,
    },
];
/// A known banner, leaving future ids to the neutral fallback.
pub(crate) fn get(id: &str) -> Option<&'static Banner> {
    ALL.iter().find(|banner| banner.id == id)
}
/// Whether `id` is the neutral cover (or one this client does not know, drawn as it):
/// the profile card and the Profile page draw no cover for it, only their text.
pub(crate) fn plain(id: &str) -> bool {
    get(id).is_none_or(|banner| banner.art == Art::Neutral)
}
/// Current inventory count; the neutral cover is free.
pub(crate) fn count(profile: Option<&sjk_identity::Profile>, id: &str) -> u32 {
    if id.is_empty() {
        return 1;
    }
    profile
        .and_then(|profile| profile.unlocks.iter().find(|unlock| unlock.id == id))
        .map_or(0, |unlock| unlock.count)
}
/// Display only a supported equipped banner that the same profile still owns.
pub(crate) fn equipped(profile: Option<&sjk_identity::Profile>) -> &'static str {
    profile
        .and_then(|profile| {
            get(&profile.banner).filter(|banner| count(Some(profile), banner.id) > 0)
        })
        .map_or("", |banner| banner.id)
}

/// Draw a cover in any rectangle, centre-cropping the same 3.6:1 composition.
/// Art stays behind a readability veil; names and avatar badges are drawn afterwards.
pub(crate) fn draw(canvas: &mut MenuCanvas, target: Rect, id: &str) {
    draw_cover(canvas, target, id, None);
}

/// [`draw`] on a card with rounded corners of `radius` window pixels: the cover's
/// gradient and veil take the card's corners. The UI renderer cuts only lines to a
/// rounded clip and rectangles to its bounding rectangle, so without this the cover
/// squares off the card's corners.
pub(crate) fn draw_rounded(canvas: &mut MenuCanvas, target: Rect, id: &str, radius: f32) {
    draw_cover(canvas, target, id, Some(radius));
}

fn draw_cover(canvas: &mut MenuCanvas, target: Rect, id: &str, corners: Option<f32>) {
    let banner = get(id).unwrap_or(&ALL[0]);
    let scale = (target.width / 720.0).max(target.height / 200.0);
    let corner = corners.unwrap_or(0.0);
    let x = target.x + (target.width - 720.0 * scale) * 0.5;
    let y = target.y + (target.height - 200.0 * scale) * 0.5;
    let rect = |a: f32, b: f32, w: f32, h: f32| {
        Rect::new(x + a * scale, y + b * scale, w * scale, h * scale)
    };
    let rgb = crate::holocrons::rgb;
    let list = canvas.draw_list_mut();
    let _ = list.push(DrawCommand::PushRoundedClip {
        rect: target,
        radius: corners.unwrap_or(8.0 * scale),
    });
    let (top, bottom) = match banner.art {
        Art::Neutral => (0x152237, 0x080D16),
        Art::Archives => (0x0F2940, 0x152438),
        Art::Sunset => (0x492D5E, 0xDD8248),
        Art::Eclipse => (0x100E20, 0x501F30),
        Art::Hyperspace => (0x07152D, 0x12365E),
        Art::Nebula => (0x25143E, 0x164D59),
    };
    let _ = list.push(DrawCommand::GradientRect {
        rect: target,
        radius: corner,
        gradient: Gradient {
            start: rgb(top),
            end: rgb(bottom),
            vertical: true,
        },
    });
    let mut shape = |r: Rect, radius: f32, c: Color| {
        let _ = list.push(DrawCommand::RoundedRect {
            rect: r,
            radius: radius * scale,
            color: c,
        });
    };
    match banner.art {
        Art::Neutral => {
            shape(
                rect(0.0, 146.0, 720.0, 1.0),
                0.0,
                Color::new(0.55, 0.67, 0.8, 0.2),
            );
            shape(
                rect(490.0, 38.0, 100.0, 2.0),
                0.0,
                Color::new(0.65, 0.75, 0.9, 0.15),
            );
        }
        Art::Archives => {
            for i in 0..12 {
                let a = 30.0 + i as f32 * 62.0;
                shape(
                    rect(a, 22.0, 2.0, 156.0),
                    0.0,
                    Color::new(0.76, 0.62, 0.35, 0.25),
                );
                for j in 0..4 {
                    shape(
                        rect(a + 10.0, 35.0 + j as f32 * 34.0, 30.0, 2.0),
                        0.0,
                        Color::new(0.4, 0.68, 0.8, 0.3),
                    );
                }
            }
        }
        Art::Sunset => {
            shape(rect(445.0, 37.0, 64.0, 64.0), 32.0, rgb(0xFFCF83));
            shape(rect(526.0, 54.0, 44.0, 44.0), 22.0, rgb(0xF29A68));
            for i in 0..8 {
                shape(
                    rect(
                        -20.0 + i as f32 * 100.0,
                        145.0 + (i % 3) as f32 * 7.0,
                        180.0,
                        100.0,
                    ),
                    50.0,
                    rgb(0x503745),
                );
            }
        }
        Art::Eclipse => {
            shape(
                rect(458.0, 14.0, 154.0, 154.0),
                77.0,
                Color::new(0.85, 0.13, 0.18, 0.16),
            );
            shape(rect(474.0, 30.0, 122.0, 122.0), 61.0, rgb(0xE64C4B));
            shape(rect(478.0, 28.0, 116.0, 116.0), 58.0, rgb(0x13101C));
            for i in 0..15 {
                shape(
                    rect(
                        20.0 + (i * 43 % 690) as f32,
                        25.0 + (i * 67 % 170) as f32,
                        2.0,
                        4.0,
                    ),
                    1.0,
                    rgb(0xD87459),
                );
            }
        }
        Art::Hyperspace => {
            for i in 0..26 {
                let a = i as f32 * 2.399_963;
                let dx = a.cos();
                let dy = a.sin();
                let _ = list.push(DrawCommand::Line {
                    from: [
                        x + (470.0 + dx * 65.0) * scale,
                        y + (95.0 + dy * 30.0) * scale,
                    ],
                    to: [
                        x + (470.0 + dx * 560.0) * scale,
                        y + (95.0 + dy * 245.0) * scale,
                    ],
                    width: (1.0 + (i % 3) as f32) * scale,
                    color: Color::new(0.36, 0.72, 1.0, 0.45),
                });
            }
        }
        Art::Nebula => {
            for i in 0..12 {
                let a = i as f32;
                shape(
                    rect(250.0 + a * 30.0, 25.0 + (i * 31 % 70) as f32, 200.0, 75.0),
                    38.0,
                    Color::new(0.35 + 0.03 * a, 0.18 + 0.025 * a, 0.65, 0.13),
                );
            }
            for i in 0..26 {
                shape(
                    rect(
                        15.0 + (i * 97 % 700) as f32,
                        12.0 + (i * 53 % 175) as f32,
                        2.0,
                        2.0,
                    ),
                    1.0,
                    Color::new(0.85, 0.92, 1.0, 0.7),
                );
            }
        }
    }
    let _ = list.push(DrawCommand::GradientRect {
        rect: target,
        radius: corner,
        gradient: Gradient {
            start: Color::new(0.0, 0.0, 0.0, 0.1),
            end: Color::new(0.0, 0.0, 0.0, 0.72),
            vertical: true,
        },
    });
    let _ = list.push(DrawCommand::PopClip);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_supported_owned_equipment_survives_and_old_json_is_neutral() {
        let mut profile:sjk_identity::Profile=serde_json::from_str(r#"{"key_id":"0123456789abcdef","key":"","name":"Player","bio":"","verified":false,"created":1}"#).unwrap();
        assert_eq!(equipped(Some(&profile)), "");
        profile.banner = ALL[1].id.into();
        assert_eq!(equipped(Some(&profile)), "");
        profile.unlocks.push(sjk_identity::Unlock {
            id: ALL[1].id.to_owned(),
            granted: 1,
            note: String::new(),
            count: 2,
            medal: None,
            card: None,
            holo: 0,
            solar: 0,
        });
        assert_eq!(equipped(Some(&profile)), ALL[1].id);
        assert!(std::sync::Arc::ptr_eq(
            &profile.banner,
            &profile.clone().banner
        ));
        profile.banner = "future_cover".into();
        assert_eq!(equipped(Some(&profile)), "");
        assert_eq!(equipped(None), "");
        assert_eq!(ALL.iter().filter(|banner| !banner.id.is_empty()).count(), 5);
    }
    #[test]
    fn every_cover_balances_its_clip_and_fits_the_existing_canvas_budget() {
        for banner in &ALL {
            let mut canvas = MenuCanvas::new();
            draw(&mut canvas, Rect::new(0.0, 0.0, 720.0, 200.0), banner.id);
            assert!(!canvas.overflowed());
            let commands = canvas.draw_list().commands();
            assert!(matches!(
                commands.first(),
                Some(DrawCommand::PushRoundedClip { .. })
            ));
            assert!(matches!(commands.last(), Some(DrawCommand::PopClip)));
        }
    }

    #[test]
    fn only_the_neutral_or_an_unknown_cover_is_plain() {
        assert!(plain(""));
        assert!(plain("banner_from_a_later_client"));
        for banner in &ALL[1..] {
            assert!(!plain(banner.id), "{}", banner.id);
        }
    }

    #[test]
    fn a_rounded_card_rounds_the_cover_gradient_and_veil() {
        for banner in &ALL {
            let mut canvas = MenuCanvas::new();
            let card = Rect::new(0.0, 0.0, 474.0, 96.0);
            draw_rounded(&mut canvas, card, banner.id, 18.0);
            let commands = canvas.draw_list().commands();
            assert!(matches!(
                commands.first(),
                Some(DrawCommand::PushRoundedClip { radius, .. }) if *radius == 18.0
            ));
            let whole: Vec<f32> = commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::GradientRect { rect, radius, .. } if *rect == card => {
                        Some(*radius)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(whole, [18.0, 18.0], "{}", banner.id);
        }
    }
}
