//! The Collection's Toys tab: things to use in a match, everyone's. SJK's Illuminate
//! holocron (`docs/client.md`, Illuminate), the cardboard box (`docs/client.md`,
//! Cardboard box), Order 66 (`docs/client.md`, Order 66), the Hax (`docs/client.md`,
//! Hax), the Ragdoll (`docs/client.md`, Ragdoll), the Drone (`docs/client.md`, Drone),
//! the Boombox (`docs/client.md`, Boombox), the Monitor (`docs/client.md`, Monitor), Box
//! hunt (`docs/client.md`, Box hunt) the Banana peel (`docs/client.md`, Banana peel),
//! Fishing (`docs/client.md`, Fishing) and Quests (`docs/quests.md`),
//! a row to each on the left (Sol's pick of 25/10/2026, the Collection revamp's canvas):
//! its picture, its name and a line of what it does, the key bound to it and its own
//! switch, which runs it in place (`toy_illuminate`, `toy_box`, `toy_order66`, `toy_hax`,
//! `toy_ragdoll`, `toy_drone`, `toy_boombox`, `toy_monitor`, `toy_boxhunt`,
//! `toy_banana`, `toy_fishing`, `toy_quests`). The chosen one is described on a dark card beside the
//! model, where its key is bound without the console (a click or K, then the key). The
//! holocron floats lit by the model while it is chosen.

use super::view::{LEFT_X, RIGHT_X};
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign, TextureId};

/// The rows: their left edge and width, where the first section's head stands (the
/// column heads on its line), a section head's room above its rows and the room before
/// the next head, a row's height and the room between rows, and its picture's side.
/// Fourteen rows in three sections fit over the keys (09/11/2026; 45 tall 2 apart in one
/// list before).
const ROWS_X: f32 = LEFT_X - 12.0;
const ROWS_WIDTH: f32 = 860.0;
const HEADS_Y: f32 = 300.0;
const HEAD_STEP: f32 = 26.0;
const SECTION_GAP: f32 = 6.0;
const ROW_HEIGHT: f32 = 42.0;
const ROW_GAP: f32 = 2.0;
const ROW_ART: f32 = 38.0;
/// A row's author column, key column (their widths), and the room the switch keeps at
/// the row's right.
const AUTHOR_WIDTH: f32 = 110.0;
const KEY_WIDTH: f32 = 150.0;
const SWITCH_ROOM: f32 = 130.0;

/// The list's sections (Sol's pick of 09/11/2026, once there were fourteen toys): each
/// head and how many toys of [`ORDER`] stand under it.
const SECTIONS: [(&str, usize); 3] = [("Around you", 5), ("With others", 4), ("Earn and win", 5)];
/// The toys in the list's order, as indices into [`TOYS`] (whose order the panel, the
/// tokens and the bits keep): around you, the ones that need others with them on, and
/// the ones that earn or win something.
pub(super) const ORDER: [usize; TOY_COUNT] = [0, 1, 4, 5, 6, 2, 3, 7, 9, 8, 10, 12, 13, 11];

/// Where each toy's row stands, in the order of [`TOYS`].
fn row_tops() -> [f32; TOY_COUNT] {
    let mut tops = [0.0; TOY_COUNT];
    let mut y = HEADS_Y;
    let mut next = 0;
    for (_, count) in SECTIONS {
        y += HEAD_STEP;
        for &toy in &ORDER[next..next + count] {
            tops[toy] = y;
            y += ROW_HEIGHT + ROW_GAP;
        }
        next += count;
        y += SECTION_GAP;
    }
    tops
}

/// The heads' tops, in the order of [`SECTIONS`].
fn head_tops() -> [f32; SECTIONS.len()] {
    let tops = row_tops();
    let mut heads = [0.0; SECTIONS.len()];
    let mut next = 0;
    for (index, (_, count)) in SECTIONS.iter().enumerate() {
        heads[index] = tops[ORDER[next]] - HEAD_STEP;
        next += count;
    }
    heads
}

/// The toy `by` rows on from `toy` in the list's order: wrapping round, or stopping at
/// the ends.
pub(super) fn step(toy: usize, by: isize, wrap: bool) -> usize {
    let count = TOY_COUNT as isize;
    let at = ORDER.iter().position(|&known| known == toy).unwrap_or(0) as isize;
    let next = if wrap {
        (at + by).rem_euclid(count)
    } else {
        (at + by).clamp(0, count - 1)
    };
    ORDER[next as usize]
}

/// Who made `toy`, for its card: "Made by Sol", and whose idea when someone else's.
fn signature(toy: &Toy) -> String {
    match toy.idea {
        Some(idea) => format!("Made by {}, {idea}'s idea", toy.author),
        None => format!("Made by {}", toy.author),
    }
}
/// The chosen toy's card beside the model: its left edge and width, its foot and the
/// highest its top rises; the room inside it.
const CARD_X: f32 = 1_360.0;
const CARD_WIDTH: f32 = RIGHT_X - CARD_X;
const CARD_BOTTOM: f32 = 990.0;
const CARD_TOP_MIN: f32 = 300.0;
const CARD_PAD: f32 = 26.0;
/// A line of the card's description holds this many characters at the ordinary text
/// size, a line of its other words this many.
const ABOUT_CHARS: f32 = 46.0;
const NOTE_CHARS: f32 = 52.0;
/// The card's "Made by" line under its title.
const SIGNATURE_ROW: f32 = 28.0;
/// The card's "See" row under its switch.
const SIGHT_ROW: f32 = 48.0;
/// The Fishing card's fish log under its switch: its head (the fish caught), its line of
/// junk and the Sell junk row (`fishing_log` prints every kind in the console).
const LOG_HEAD: f32 = 30.0;
const LOG_LINE: f32 = 22.0;
const SELL_ROW: f32 = 42.0;

/// Every command a toy is run by, in the order the page is told their keys
/// ([`Inputs::toy_keys`]): each toy's switch, Order 66's order after its own, the
/// Monitor's throw (held, it charges) after its own, and the Banana peel's drop after its
/// own.
pub(crate) const COMMANDS: [&str; 17] = [
    "toy_illuminate",
    "toy_box",
    "toy_order66",
    "order66",
    "toy_hax",
    "toy_ragdoll",
    "toy_drone",
    "toy_boombox",
    "toy_monitor",
    "+monitor_throw",
    "toy_boxhunt",
    "toy_banana",
    "banana_drop",
    crate::fishing::COMMAND,
    crate::wheel_of_doom::COMMAND,
    crate::mining::COMMAND,
    crate::quests::COMMAND,
];

/// One toy as the tab shows it.
struct Toy {
    /// Its quick wheel icon's name (the pictures are uploaded into the atlas at start).
    icon: &'static str,
    /// On its row.
    name: &'static str,
    /// Its row's line under the name: what it does, in a few words.
    tag: &'static str,
    /// On its card.
    title: &'static str,
    what: &'static str,
    /// What more its card says under its keys, when there is more to know.
    more: Option<&'static str>,
    /// The commands it is run by (the switch's first), and what each key is called.
    commands: &'static [(&'static str, &'static str)],
    /// The switch's words.
    switch: &'static str,
    /// The cvar of the card's second switch, which hides other players' toy on this PC,
    /// and its words ([`crate::toy_sight`]); none for a toy that only acts on players
    /// who have it on.
    sight: Option<(&'static str, &'static str)>,
    /// The card's own switches under those, each an on/off cvar of the player's and its
    /// words (the Hax's trap, moved here from Settings > Gameplay on 03/11/2026, Lex's
    /// idea); at most [`super::TOY_OPTION_TOKENS`].
    options: &'static [(&'static str, &'static str)],
    token: u16,
    /// Who made it, signed on its row and its card (Sol's list of 09/11/2026: Illuminate
    /// to Box hunt and the WHEEL OF DOOM are Sol's), and whose idea it was when that is
    /// someone else's.
    author: &'static str,
    idea: Option<&'static str>,
}

/// The toys, in the order of [`Panel::toy`].
const TOYS: [Toy; TOY_COUNT] = [
    Toy {
        icon: "illuminate",
        name: "Illuminate",
        tag: "A holocron by your shoulder lights dark maps.",
        title: "Illuminate holocron",
        what: "A holocron floats by your left shoulder and lights the way in dark maps. Other SJK players on your server see it lit.",
        more: None,
        commands: &[("toy_illuminate", "Your key")],
        switch: "Holocron lit",
        sight: Some((crate::toy_sight::ILLUMINATE, "See others' holocrons")),
        options: &[],
        token: TOY_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "cardboard_box",
        name: "Cardboard box",
        tag: "Hide in a carton; someone may lift it.",
        title: "Cardboard box",
        what: "Hide in a carton: you crouch in it with its music. Other SJK players see the box; one who walks up and presses Use lifts it. Die in it and everyone hears.",
        more: None,
        commands: &[("toy_box", "Your key")],
        switch: "In the box",
        sight: Some((crate::toy_sight::CARDBOARD_BOX, "See others' boxes")),
        options: &[],
        token: BOX_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "order66",
        name: "Order 66",
        tag: "One order, and everyone in it falls.",
        title: "Order 66",
        what: "When one player with it on executes the order, everyone with it on dies three seconds later and blows apart. One order every 30 seconds.",
        more: Some("Join with its switch or key; the order key executes it for everyone in it."),
        commands: &[("toy_order66", "Join key"), ("order66", "Order key")],
        switch: "In the order",
        sight: None,
        options: &[],
        token: ORDER_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "hax",
        name: "Hax",
        tag: "Hack a player and steer them for five seconds.",
        title: "The Hax",
        what: "Hold Use on a player who has it on too: the siege hack bar fills, then you steer them for five seconds, run, jump and swing included. Turn it off to break free.",
        more: Some("With it on, hold Use on a player who has it on too."),
        commands: &[("toy_hax", "Your key")],
        switch: "Hacker",
        sight: None,
        options: &[
            (crate::hax::TRAP_CVAR, "Trap: a hacker blows up with you"),
            (crate::hax::AWAY_CVAR, "Trap while away"),
        ],
        token: HAX_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "ragdoll",
        name: "Ragdoll",
        tag: "Go limp and fall where physics takes you.",
        title: "Ragdoll",
        what: "Go limp and fall where physics takes you, sandbox style, though you are not dead; the camera follows the body. Other SJK players see the same fall. Getting hurt gets you up.",
        more: Some("Press its key again to get up."),
        commands: &[("toy_ragdoll", "Your key")],
        switch: "Limp",
        sight: Some((crate::toy_sight::RAGDOLL, "See others fall limp")),
        options: &[],
        token: RAGDOLL_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "drone",
        name: "Drone",
        tag: "Fly a remote others see; let them peek at you.",
        title: "Drone",
        what: "Your free camera and Peek fly a training remote that other SJK players see, your name over it. With this on, players may peek at you: their drone hovers behind you.",
        more: Some("freecam and peek fly it, with the keys you have for them."),
        commands: &[("toy_drone", "Your key")],
        switch: "Peek me",
        sight: Some((crate::toy_sight::DRONE, "See others' drones")),
        options: &[],
        token: DRONE_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "boombox",
        name: "Boombox",
        tag: "Play your tracks to the players close by.",
        title: "Boombox",
        what: "Carry an 80s stereo on your shoulder, playing your tracks to the SJK players standing close by. Tracks are sound/boombox/<name>.mp3 files in your pk3s; others hear one when they have it too. For verified players.",
        more: Some(
            "boombox in the console lists your tracks; boombox next skips one. The place key stands it on the floor in front of you, playing from there (quieter behind it); again picks it up.",
        ),
        commands: &[
            ("toy_boombox", "Your key"),
            (crate::boombox::PLACE_COMMAND, "Place key"),
        ],
        switch: "Play",
        sight: Some((crate::toy_sight::BOOMBOX, "See and hear others' boomboxes")),
        options: &[],
        token: BOOMBOX_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "monitor",
        name: "Monitor",
        tag: "Throw an old monitor; it sends players flying.",
        title: "Monitor",
        what: "Throw a big old CRT monitor. One that hits a player who has it on too knocks them down as a ragdoll and sends them flying, the way it struck them; they get up once they land. With it off, monitors pass through you.",
        more: Some(
            "Its switch lets you throw and be hit; the throw key throws one, harder the longer you hold it.",
        ),
        commands: &[("toy_monitor", "Your key"), ("+monitor_throw", "Throw key")],
        switch: "Hit me",
        sight: Some((crate::monitor::SHOW_CVAR, "See monitors")),
        options: &[],
        token: MONITOR_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "box_hunt",
        name: "Box hunt",
        tag: "Hunt the mark for a free mystery box roll.",
        title: "Box hunt",
        what: "Every 10 minutes one player with Box hunt on becomes the mark. Kill the mark to win a free mystery box roll. Only players with it on see the mark or can be it.",
        more: Some("Its switch is all there is: on, you hunt and may be the mark."),
        commands: &[("toy_boxhunt", "Your key")],
        switch: "Hunt",
        sight: None,
        options: &[],
        token: BOX_HUNT_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "banana",
        name: "Banana peel",
        tag: "Drop peels; players who walk onto them slip.",
        title: "Banana peel",
        what: "Drop a banana peel at your feet, or toss one ahead as you run. A player who has it on too and walks onto a peel slips and falls as a ragdoll, cartoon style; the peel is gone. With it off, you walk over peels.",
        more: Some(
            "Three peels down at a time, each for two minutes, none by a spawn point. Its switch lets you drop them and slip on them.",
        ),
        commands: &[
            (crate::banana::COMMAND, "Your key"),
            (crate::banana::DROP_COMMAND, "Drop key"),
        ],
        switch: "Slip on peels",
        sight: Some((crate::banana::SHOW_CVAR, "See banana peels")),
        options: &[],
        token: BANANA_TOKEN,
        author: "Sol",
        idea: Some("Kushina"),
    },
    Toy {
        icon: "fishing",
        name: "Fishing",
        tag: "Cast a line, wait for the bite, reel it in.",
        title: "Fishing",
        what: "Stand still and cast into the water ahead, or a pond where there is none. When the float dips, press Use in time to land a fish or some junk. An Echo dropped to you while you fish bites too. Other SJK players see your rod.",
        more: Some(
            "Moving or getting hurt puts the rod away. Junk sells to the hub for a few credits, a small sum a day.",
        ),
        commands: &[(crate::fishing::COMMAND, "Your key")],
        switch: "Rod out",
        sight: None,
        options: &[],
        token: FISHING_TOKEN,
        author: "Slash",
        idea: Some("Sanji"),
    },
    Toy {
        icon: "wheel_of_doom",
        name: "WHEEL OF DOOM",
        tag: "Spin it for credits: a Mythical holocron, or doom.",
        title: "WHEEL OF DOOM",
        what: "A big dark wheel rises before you and spins. It may land on a Mythical holocron, on nothing, or on something terrible: your keys or mouse turned against you, the world mirrored or upside down, darkness, no saber, quacks, a curse, or a blast. Other SJK players see it spin.",
        more: Some(
            "Its switch shows the price; press it again within 5 s to pay and spin. One spin every 5 minutes, not in a duel. What it does lasts a few seconds and ends on a new map or when you die.",
        ),
        commands: &[(crate::wheel_of_doom::COMMAND, "Your key")],
        switch: "Spin",
        sight: None,
        options: &[],
        token: WHEEL_OF_DOOM_TOKEN,
        author: "Sol",
        idea: None,
    },
    Toy {
        icon: "mining",
        name: "Mining",
        tag: "Break the boulders that come up round you for ore.",
        title: "Mining",
        what: "Take a pickaxe and boulders come up round you. Strike one with Attack (hold it to keep swinging) to break it for ore or rubble. When it glints, strike: a clean strike counts twice and digs out rarer ore. Other SJK players see your pickaxe.",
        more: Some(
            "A weapon key or getting hurt puts it away. Rubble sells to the hub for a few credits, a small sum a day shared with junk.",
        ),
        commands: &[(crate::mining::COMMAND, "Your key")],
        switch: "Pickaxe out",
        sight: None,
        options: &[],
        token: MINING_TOKEN,
        author: "Slash",
        idea: Some("vinry"),
    },
    Toy {
        icon: "quests",
        name: "Quests",
        tag: "Take quests from the hub for Echoes.",
        title: "Quests",
        what: "Now and then one player with Quests on is given a quest with a minute to finish it, such as hugging two other players who have it on (/amhug). The players it names see it; a finished quest wins an Echo, a few times a day.",
        more: Some(
            "Its switch is all there is: on, you may be given a quest or named in one. A quest's Echo is one of the Echoes you find a day by playing, and you finish one quest a day with the same player. Quests run on JA+ servers.",
        ),
        commands: &[(crate::quests::COMMAND, "Your key")],
        switch: "Take quests",
        sight: None,
        options: &[],
        token: QUESTS_TOKEN,
        author: "Slash",
        idea: Some("wonner67"),
    },
];

/// The Toy Master's achievements the player holds: the toys they found.
pub(super) fn toy_master_found(
    standings: &[crate::achievements::Standing],
) -> impl Iterator<Item = &crate::achievements::Standing> {
    standings.iter().filter(|standing| {
        standing.kind.category == crate::achievements::Category::ToyMaster
            && standing.unlocked.is_some()
    })
}

/// The command toy `toy`'s key `slot` runs (0 its switch's, 1 Order 66's order).
pub(super) fn command(toy: usize, slot: usize) -> Option<&'static str> {
    TOYS.get(toy)
        .and_then(|toy| toy.commands.get(slot))
        .map(|(command, _)| *command)
}

/// The cvar of toy `toy`'s "See" switch, if it has one.
pub(super) fn sight_cvar(toy: usize) -> Option<&'static str> {
    TOYS.get(toy)?.sight.map(|(cvar, _)| cvar)
}

/// The toys' own card switches ([`Toy::options`]) that `on` says are on, as bits in the
/// order of every toy's switches one after the other.
pub(crate) fn options_on(on: impl Fn(&str) -> bool) -> u16 {
    TOYS.iter()
        .flat_map(|toy| toy.options)
        .enumerate()
        .filter(|(_, (cvar, _))| on(cvar))
        .fold(0, |bits, (index, _)| bits | 1 << index)
}

/// Toy `toy`'s own switch `index`: its cvar and its bit in [`options_on`].
pub(super) fn option(toy: usize, index: usize) -> Option<(&'static str, usize)> {
    let before: usize = TOYS.get(..toy)?.iter().map(|toy| toy.options.len()).sum();
    let (cvar, _) = TOYS.get(toy)?.options.get(index)?;
    Some((cvar, before + index))
}

/// The toys whose other players' are hidden, as bits in the tab's order: those whose
/// "See" cvar `seen` says off.
pub(crate) fn unseen(seen: impl Fn(&str) -> bool) -> u16 {
    TOYS.iter()
        .enumerate()
        .filter(|(_, toy)| toy.sight.is_some_and(|(cvar, _)| !seen(cvar)))
        .fold(0, |bits, (index, _)| bits | 1 << index)
}

/// The keys bound to `command`, as the page was told them; empty for none.
fn keys_of<'a>(inputs: &'a Inputs<'_>, command: &str) -> &'a str {
    COMMANDS
        .iter()
        .position(|&known| known == command)
        .and_then(|index| inputs.toy_keys.get(index))
        .map_or("", String::as_str)
}

/// A toy's picture: the quick wheel's, uploaded into the atlas at start.
fn icon(name: &str) -> TextureId {
    crate::ui_renderer::wheel_icon(
        crate::quick_wheel::catalog::icon_index(name)
            .expect("the quick wheel's icons hold every toy's"),
    )
}

/// The player's text size (`ui_textScale`).
fn text_scale() -> f32 {
    crate::text::style::current().scale.max(0.5)
}

/// A row of dots across the left side at `y`: the room for more.
pub(super) fn room_for_more(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    y: f32,
    words: &str,
) {
    let mut x = LEFT_X;
    while x < LEFT_X + 820.0 {
        let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(x, y, 8.0, 1.0),
            color: color::alpha(color::HOLO, 0.22),
        });
        x += 16.0;
    }
    text(
        canvas,
        TextFamily::Body,
        format_args!("{words}"),
        frame.rect(LEFT_X, y + 16.0, 820.0, 24.0),
        15.0 * frame.s,
        color::QUIET,
        FontWeight::Regular,
        TextAlign::Start,
    );
}

/// Whether each toy is on now, as `inputs` says, in the order of [`TOYS`].
fn states(inputs: &Inputs<'_>) -> [bool; TOY_COUNT] {
    [
        inputs.illuminate,
        inputs.cardboard_box,
        inputs.order66,
        inputs.hax,
        inputs.ragdoll,
        inputs.drone,
        inputs.boombox,
        inputs.monitor,
        inputs.box_hunt,
        inputs.banana,
        inputs.fishing,
        inputs.wheel_of_doom,
        inputs.mining,
        inputs.quests,
    ]
}

/// The Toys tab's headline, "12 toys": counted from the toys, as the tab's count is, so
/// a new toy cannot leave it behind (hub report #265).
fn headline() -> String {
    format!("{TOY_COUNT} toys")
}

impl Panel {
    /// The Toys tab.
    pub(super) fn toys(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let on = states(inputs);
        let lit = on.iter().filter(|on| **on).count();
        let line = match lit {
            0 => "Switch them here or in a match; other SJK players on your server see what you turn on.".to_owned(),
            lit => format!("{lit} on now. Switch them here or in a match; other SJK players on your server see what you turn on."),
        };
        self.lead(frame, &headline(), &line);
        self.toy_heads(frame);
        let chosen = self.toy.min(TOY_COUNT - 1);
        let tops = row_tops();
        for (index, toy) in TOYS.iter().enumerate() {
            let y = tops[index];
            let key = keys_of(inputs, toy.commands[0].0);
            self.toy_row(frame, index, toy, y, on[index], key, index == chosen);
        }
        self.toy_card(frame, inputs, chosen, on[chosen]);
        self.toy_master_badge(frame, inputs);
    }

    /// The Toy Master's "?" at the right of the column heads (Sol, 30/10/2026): the
    /// toys SJK's creator hides on maps are secrets, so it shows nothing of them but a
    /// question mark, gold once the player found one; its hover card says the rest.
    fn toy_master_badge(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let found = toy_master_found(inputs.standings).count();
        let side = 40.0;
        let x = ROWS_X + ROWS_WIDTH - side;
        let y = HEADS_Y - side - 14.0;
        let area = frame.rect(x, y, side, side);
        let hovered = self.ui.token_hovered(TOY_MASTER_TOKEN);
        let tint = if found > 0 {
            color::GOLD_BRIGHT
        } else {
            color::alpha(color::HOLO, 0.85)
        };
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 10.0 * s,
            color: color::alpha(color::SPACE, if hovered { 0.85 } else { 0.6 }),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 10.0 * s,
            width: if hovered { 1.6 } else { 1.0 } * s,
            color: if hovered {
                tint
            } else {
                color::alpha(tint, 0.6)
            },
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("?"),
            frame.rect(x, y + 5.0, side, 30.0),
            26.0 * s,
            tint,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        self.ui.hit_region(TOY_MASTER_TOKEN, area);
    }

    /// The sections' heads, each over its rows with a rule under it; the first's line
    /// also names the columns: the author, the key and the switch.
    fn toy_heads(&mut self, frame: &Frame) {
        let s = frame.s;
        let right = ROWS_X + ROWS_WIDTH;
        let key_x = right - SWITCH_ROOM - KEY_WIDTH;
        let author_x = key_x - AUTHOR_WIDTH;
        for ((words, _), y) in SECTIONS.iter().zip(head_tops()) {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(ROWS_X + 18.0, y, 300.0, 20.0),
                15.0 * s,
                color::HOLO,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
                rect: frame.rect(ROWS_X, y + HEAD_STEP - 4.0, ROWS_WIDTH, 1.0),
                color: color::alpha(color::HOLO, 0.25),
            });
        }
        for (words, x, width, align) in [
            ("By", author_x, AUTHOR_WIDTH, TextAlign::Start),
            ("Your key", key_x, KEY_WIDTH, TextAlign::Start),
            (
                "On",
                right - SWITCH_ROOM,
                SWITCH_ROOM - 18.0,
                TextAlign::End,
            ),
        ] {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(x, HEADS_Y, width, 20.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Semibold,
                align,
            );
        }
    }

    /// Toy `index`'s row at `y`: its picture, name and line, the key bound to it (`key`,
    /// empty for none) and its switch (`on`), which runs it in place; the chosen one
    /// framed gold on a soft gold ground.
    #[allow(clippy::too_many_arguments)]
    fn toy_row(
        &mut self,
        frame: &Frame,
        index: usize,
        toy: &Toy,
        y: f32,
        on: bool,
        key: &str,
        chosen: bool,
    ) {
        let s = frame.s;
        let k = text_scale();
        let x = ROWS_X;
        let right = x + ROWS_WIDTH;
        let area = frame.rect(x, y, ROWS_WIDTH, ROW_HEIGHT);
        let hovered = self.ui.token_hovered(toy.token);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 12.0 * s,
            color: if chosen {
                color::alpha(color::GOLD, 0.09)
            } else {
                color::alpha(color::SPACE, if hovered { 0.78 } else { 0.6 })
            },
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 12.0 * s,
            width: if chosen { 1.6 } else { 1.0 } * s,
            color: if chosen {
                color::GOLD_BRIGHT
            } else {
                color::alpha(color::HOLO, if hovered { 0.35 } else { 0.16 })
            },
        });
        let art = ROW_ART;
        let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
            rect: frame.rect(x + 14.0, y + (ROW_HEIGHT - art) * 0.5, art, art),
            texture: icon(toy.icon),
            color: Color::new(1.0, 1.0, 1.0, if chosen || on { 1.0 } else { 0.78 }),
        });
        let key_x = right - SWITCH_ROOM - KEY_WIDTH;
        let author_x = key_x - AUTHOR_WIDTH;
        let words_x = x + 14.0 + art + 16.0;
        let words_width = author_x - 12.0 - words_x;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", toy.name),
            frame.rect(words_x, y + 2.0, words_width, 24.0 * k),
            21.0 * s,
            if chosen {
                color::GOLD_BRIGHT
            } else {
                color::TEXT
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", toy.tag),
            frame.rect(words_x, y + 2.0 + 22.0 * k, words_width, 18.0 * k),
            14.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let middle = y + ROW_HEIGHT * 0.5;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", toy.author),
            frame.rect(author_x, middle - 11.0, AUTHOR_WIDTH - 8.0, 22.0),
            16.0 * s,
            if chosen { color::TEXT } else { color::MUTED },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if key.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("Not bound"),
                frame.rect(key_x, middle - 11.0, KEY_WIDTH, 22.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        } else {
            let width = (crate::text::display_width(key, 16.0) * k + 22.0).min(KEY_WIDTH);
            let cap = frame.rect(key_x, middle - 15.0, width, 30.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: cap,
                radius: 6.0 * s,
                width: 1.2 * s,
                color: color::alpha(color::TEXT, 0.55),
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{key}"),
                frame.rect(key_x, middle - 11.0, width, 22.0),
                16.0 * s,
                color::TEXT,
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
        // The switch answers before the row, so a click on it is the switch's.
        let switch_right = right - 18.0;
        let left = kit::switch(&mut self.ui, frame, switch_right, middle, on, false);
        let switch_token = TOY_SWITCH_BASE + index as u16;
        self.ui.hit_region(
            switch_token,
            frame.rect(left - 6.0, middle - 20.0, switch_right - left + 12.0, 40.0),
        );
        self.ui
            .hit_region(toy.token, frame.rect(x, y, left - 12.0 - x, ROW_HEIGHT));
    }

    /// How tall the chosen toy's card stands: its words wrapped in full, its keys, the
    /// quick wheel's line and its switch.
    fn toy_card_height(&self, toy: &Toy) -> f32 {
        let k = text_scale();
        let line = |size: f32| size * 1.42 * k;
        let about = crate::menu::sjk::wrap(toy.what, (ABOUT_CHARS / k).max(12.0) as usize).count();
        let more = toy.more.map_or(0, |more| {
            crate::menu::sjk::wrap(more, (NOTE_CHARS / k).max(12.0) as usize).count()
        });
        CARD_PAD
            + 24.0
            + 50.0 * k.max(1.0)
            + SIGNATURE_ROW * k.max(1.0)
            + 10.0
            + about as f32 * line(17.0)
            + 18.0
            + toy.commands.len() as f32 * 44.0
            + 34.0
            + more as f32 * line(15.0)
            + 18.0
            + 48.0
            + if toy.sight.is_some() { SIGHT_ROW } else { 0.0 }
            + toy.options.len() as f32 * SIGHT_ROW
            + if toy.token == FISHING_TOKEN || toy.token == MINING_TOKEN {
                fish_log_height()
            } else {
                0.0
            }
            + CARD_PAD
    }

    /// The chosen toy on its card beside the model, over dark glass: what it is, its keys
    /// (each bound here: a click, or K for the first, then the key), the quick wheel and
    /// its switch (`on`: it is on now). With no model on show, its picture stands large
    /// where the model would.
    fn toy_card(&mut self, frame: &Frame, inputs: &Inputs<'_>, chosen: usize, on: bool) {
        let s = frame.s;
        let k = text_scale();
        let toy = &TOYS[chosen];
        if self.backstage == Backstage::None {
            super::view::glow(
                &mut self.ui,
                frame,
                [1_160.0, 560.0],
                150.0,
                color::GOLD,
                0.3,
            );
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(1_060.0, 460.0, 200.0, 200.0),
                texture: icon(toy.icon),
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
        }
        let height = self.toy_card_height(toy).min(CARD_BOTTOM - CARD_TOP_MIN);
        let top = CARD_BOTTOM - height;
        let area = frame.rect(CARD_X, top, CARD_WIDTH, height);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 16.0 * s,
            color: color::alpha(color::SPACE, 0.88),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(CARD_X + 24.0, top, CARD_WIDTH - 48.0, 3.0),
            radius: 1.5 * s,
            color: color::GOLD,
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 16.0 * s,
            width: 1.2 * s,
            color: color::alpha(color::HOLO, 0.3),
        });
        let x = CARD_X + CARD_PAD;
        let width = CARD_WIDTH - CARD_PAD * 2.0;
        let mut y = top + CARD_PAD;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Toy  ·  everyone's"),
            frame.rect(x, y, width, 22.0),
            16.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 24.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", toy.title),
            frame.rect(x, y, width, 50.0 * k.max(1.0)),
            40.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 50.0 * k.max(1.0);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", signature(toy)),
            frame.rect(x, y, width, 24.0 * k.max(1.0)),
            17.0 * s,
            color::MUTED,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += SIGNATURE_ROW * k.max(1.0) + 10.0;
        y = words(
            &mut self.ui,
            frame,
            toy.what,
            [x, y, width],
            ABOUT_CHARS,
            17.0,
            color::TEXT,
        );
        y += 18.0;
        for (slot, (command, label)) in toy.commands.iter().enumerate() {
            let waiting = self.bind_wait == Some(slot);
            let key = keys_of(inputs, command);
            let token = if slot == 0 {
                BIND_TOKEN
            } else {
                BIND_SECOND_TOKEN
            };
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{label}"),
                frame.rect(x, y + 8.0, 100.0, 22.0),
                15.0 * s,
                color::HOLO,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let shown = if waiting {
                "Press a key..."
            } else if key.is_empty() {
                "Bind a key"
            } else {
                key
            };
            let button_width = (crate::text::display_width(shown, 17.0) * k + 32.0).max(110.0);
            let button_x = x + 104.0;
            let rect = frame.rect(button_x, y, button_width, 36.0);
            let hovered = self.ui.token_hovered(token);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 8.0 * s,
                color: if waiting {
                    color::alpha(color::GOLD, 0.16)
                } else {
                    color::alpha(color::HOLO, if hovered { 0.14 } else { 0.05 })
                },
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 8.0 * s,
                width: 1.2 * s,
                color: if waiting {
                    color::GOLD
                } else if key.is_empty() {
                    color::alpha(color::HOLO, 0.45)
                } else {
                    color::alpha(color::TEXT, 0.55)
                },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{shown}"),
                frame.rect(button_x, y + 6.0, button_width, 24.0),
                17.0 * s,
                if waiting {
                    color::GOLD_BRIGHT
                } else {
                    color::TEXT
                },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            self.ui.hit_region(token, rect);
            let note_x = button_x + button_width + 12.0;
            let note = if waiting {
                "Esc cancels, Backspace clears"
            } else {
                command
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{note}"),
                frame.rect(note_x, y + 9.0, (x + width - note_x).max(0.0), 20.0),
                13.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 44.0;
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Quick wheel"),
            frame.rect(x, y + 4.0, 100.0, 22.0),
            15.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("On its Toys page"),
            frame.rect(x + 104.0, y + 4.0, width - 104.0, 22.0),
            15.0 * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += 34.0;
        if let Some(more) = toy.more {
            y = words(
                &mut self.ui,
                frame,
                more,
                [x, y, width],
                NOTE_CHARS,
                15.0,
                color::MUTED,
            );
        }
        y += 8.0;
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(x, y, width, 1.0),
            color: color::alpha(color::HOLO, 0.18),
        });
        y += 10.0;
        let middle = y + 24.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", toy.switch),
            frame.rect(x, middle - 15.0, width - 140.0, 30.0),
            22.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let right = x + width;
        let left = kit::switch(&mut self.ui, frame, right, middle, on, true);
        self.ui.hit_region(
            TOY_SWITCH_TOKEN,
            frame.rect(left - 4.0, middle - 19.0, right - left + 8.0, 38.0),
        );
        // Other players' toy, seen or hidden on this PC.
        let mut row = middle;
        if let Some((_, words)) = toy.sight {
            let middle = middle + SIGHT_ROW;
            row = middle;
            let seen = inputs.toys_unseen & 1 << chosen == 0;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(x, middle - 13.0, width - 140.0, 26.0),
                18.0 * s,
                color::MUTED,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let left = kit::switch(&mut self.ui, frame, right, middle, seen, false);
            self.ui.hit_region(
                TOY_SIGHT_TOKEN,
                frame.rect(left - 4.0, middle - 19.0, right - left + 8.0, 38.0),
            );
        }
        // The toy's own switches (the Hax's trap).
        for (index, (_, words)) in toy.options.iter().enumerate().take(TOY_OPTION_TOKENS) {
            let middle = row + SIGHT_ROW * (index + 1) as f32;
            let on =
                option(chosen, index).is_some_and(|(_, bit)| inputs.toy_options & 1 << bit != 0);
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(x, middle - 13.0, width - 140.0, 26.0),
                18.0 * s,
                color::MUTED,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let left = kit::switch(&mut self.ui, frame, right, middle, on, false);
            self.ui.hit_region(
                TOY_OPTION_TOKEN + index as u16,
                frame.rect(left - 4.0, middle - 19.0, right - left + 8.0, 38.0),
            );
        }
        let log = match toy.token {
            FISHING_TOKEN => inputs.fishing_card,
            MINING_TOKEN => inputs.mining_card,
            _ => None,
        };
        if let Some(card) = log {
            let below = row + SIGHT_ROW * toy.options.len() as f32 + 30.0;
            self.fish_log(frame, card, [x, below, width]);
        }
    }

    /// A toy's log from (`x`, `y`), the Fishing card's fish log or the Mining card's
    /// mining log: what was caught or dug, what sells held (junk, rubble), and the Sell
    /// button with what it pays now (when the hub buys it from the player) and the last
    /// sale's result.
    fn fish_log(
        &mut self,
        frame: &Frame,
        card: &crate::fishing_sell::Card,
        [x, mut y, width]: [f32; 3],
    ) {
        let s = frame.s;
        let k = text_scale();
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(x, y, width, 1.0),
            color: color::alpha(color::HOLO, 0.18),
        });
        y += 8.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", card.title),
            frame.rect(x, y, 110.0, 22.0),
            15.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", card.summary),
            frame.rect(x + 110.0, y, width - 110.0, 22.0),
            15.0 * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += LOG_HEAD - 8.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", card.junk_line),
            frame.rect(x, y, width, LOG_LINE * k),
            14.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += LOG_LINE + 4.0;
        let Some(pays) = card.sell else {
            return;
        };
        let has_junk = card.pieces > 0;
        let goods = card.goods;
        let shown = if card.selling {
            "Selling...".to_owned()
        } else if pays > 0 && has_junk {
            format!("Sell {goods} (+{pays} credits)")
        } else {
            format!("Sell {goods}")
        };
        let live = !card.selling && pays > 0 && has_junk;
        let button_width =
            ((crate::text::display_width(&shown, 17.0) * k + 32.0).max(140.0)).min(width);
        let rect = frame.rect(x, y, button_width, 36.0);
        let hovered = live && self.ui.token_hovered(SELL_JUNK_TOKEN);
        let ground = if hovered {
            0.22
        } else if live {
            0.12
        } else {
            0.04
        };
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 8.0 * s,
            color: color::alpha(color::GOLD, ground),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 8.0 * s,
            width: 1.2 * s,
            color: if live {
                color::GOLD
            } else {
                color::alpha(color::TEXT, 0.3)
            },
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{shown}"),
            frame.rect(x, y + 6.0, button_width, 24.0),
            17.0 * s,
            if live {
                color::GOLD_BRIGHT
            } else {
                color::QUIET
            },
            FontWeight::Semibold,
            TextAlign::Center,
        );
        if live {
            self.ui.hit_region(SELL_JUNK_TOKEN, rect);
        }
        // Beside it, the last sale's result, else why it does nothing.
        let nothing = format!("No {goods} to sell");
        let note = match &card.line {
            Some(line) => line.as_str(),
            None if !has_junk => nothing.as_str(),
            None if pays == 0 => "The hub buys no more today",
            None => "A small sum a day",
        };
        let note_x = x + button_width + 12.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{note}"),
            frame.rect(note_x, y + 9.0, (x + width - note_x).max(0.0), 20.0),
            13.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }
}

/// How tall the Fishing card's fish log (and the Mining card's log) stands below its
/// switch.
fn fish_log_height() -> f32 {
    30.0 + LOG_HEAD + LOG_LINE + 4.0 + SELL_ROW
}

/// `body` from (`x`, `y`) in a column `width` wide, wrapped at `chars` to a line at the
/// ordinary text size (fewer at a larger one), every line of it; returns where the next
/// line would stand. The Nameplates tab's card writes its words so too.
pub(super) fn words(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    body: &str,
    [x, mut y, width]: [f32; 3],
    chars: f32,
    size: f32,
    colour: Color,
) -> f32 {
    let k = text_scale();
    let step = size * 1.42 * k;
    for part in crate::menu::sjk::wrap(body, (chars / k).max(12.0) as usize) {
        text(
            canvas,
            TextFamily::Body,
            format_args!("{part}"),
            frame.rect(x, y, width, step),
            size * frame.s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += step;
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hub report #265 ({{S}}NX|LH{SW}): the page's headline names as many toys as the
    /// tab's count and the rows.
    #[test]
    fn the_headline_counts_every_toy() {
        let counts = super::super::view::counts(&super::super::tests::inputs(
            unlockables::Holdings::Known(&[]),
            "",
        ));
        assert_eq!(headline(), format!("{} toys", counts[3].total));
        assert_eq!(headline(), format!("{} toys", TOYS.len()));
    }

    /// Each toy's row, its switch and its card's keys and switch fit over the keys and
    /// answer the pointer, the rows left of the model's column and the card right of it,
    /// with and without a model, at 1080 lines, 4K, 4:3 and 21:9.
    #[test]
    fn the_toys_fit_over_the_keys() {
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        let keys = ["F5".to_owned(), String::new(), "F6 / K".to_owned()];
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            for backstage in [Backstage::None, Backstage::Preview { ready: true }] {
                for toy in 0..TOY_COUNT {
                    let inputs = Inputs {
                        toy_keys: &keys,
                        ..super::super::tests::inputs(unlockables::Holdings::Known(&[]), "")
                    };
                    let mut panel = Panel::new();
                    panel.open(Tab::Toys, true, true, ReturnTarget::InGame);
                    panel.set_backstage(backstage);
                    panel.toy = toy;
                    panel.build(&inputs, &font, viewport);
                    assert!(!panel.ui.overflowed(), "{viewport:?}");
                    let frame = Frame::new(viewport);
                    let bottom = frame.point(0.0, super::super::view::KEYS_Y)[1];
                    let stage_left = frame.point(1_000.0, 0.0)[0];
                    let stage_right = frame.point(1_320.0, 0.0)[0];
                    for (index, row) in TOY_TOKENS.iter().enumerate() {
                        for token in [*row, TOY_SWITCH_BASE + index as u16] {
                            let area = panel.ui.rect_for(token).expect("a target");
                            assert!(area.bottom() <= bottom, "{viewport:?}");
                            assert!(area.right() <= stage_left, "{viewport:?}");
                        }
                    }
                    let mut card = vec![TOY_SWITCH_TOKEN, BIND_TOKEN];
                    if toy == 2 || toy == 7 || toy == 9 {
                        card.push(BIND_SECOND_TOKEN);
                    }
                    for token in card {
                        let area = panel.ui.rect_for(token).expect("a card target");
                        assert!(area.bottom() <= bottom, "{viewport:?}");
                        assert!(area.x >= stage_right, "{viewport:?}");
                    }
                    for command in panel.ui.draw_list().commands() {
                        if let DrawCommand::Text { rect, .. } = command {
                            assert!(rect.bottom() <= bottom || rect.y >= bottom, "{rect:?}");
                            assert!(rect.right() <= viewport[0] + 1.0, "{rect:?}");
                        }
                    }
                }
            }
        }
    }

    /// The sections hold every toy once, the rows stand in the list's order without
    /// overlapping, and the arrows walk that order: down from Boombox, the last of
    /// Around you, comes Order 66.
    #[test]
    fn the_sections_hold_every_toy_once() {
        let mut seen = ORDER.to_vec();
        seen.sort_unstable();
        assert_eq!(seen, (0..TOY_COUNT).collect::<Vec<_>>());
        assert_eq!(
            SECTIONS.iter().map(|(_, count)| count).sum::<usize>(),
            TOY_COUNT
        );
        let tops = row_tops();
        for pair in ORDER.windows(2) {
            assert!(tops[pair[1]] >= tops[pair[0]] + ROW_HEIGHT, "{pair:?}");
        }
        assert!(tops[ORDER[TOY_COUNT - 1]] + ROW_HEIGHT <= super::super::view::KEYS_Y);
        assert_eq!(step(6, 1, false), 2);
        assert_eq!(step(2, -1, false), 6);
        assert_eq!(step(ORDER[TOY_COUNT - 1], 1, true), ORDER[0]);
        assert_eq!(step(ORDER[TOY_COUNT - 1], 1, false), ORDER[TOY_COUNT - 1]);
        assert_eq!(signature(&TOYS[9]), "Made by Sol, Kushina's idea");
        assert_eq!(signature(&TOYS[0]), "Made by Sol");
    }

    /// Every toy's card holds its words in full below its highest top.
    #[test]
    fn every_toy_card_holds_its_words() {
        let panel = Panel::new();
        for toy in &TOYS {
            assert!(
                panel.toy_card_height(toy) <= CARD_BOTTOM - CARD_TOP_MIN,
                "{} {} {}",
                toy.name,
                panel.toy_card_height(toy),
                text_scale()
            );
        }
    }

    /// A toy's key is bound from its card: K (or a click) waits, a key binds it to the
    /// toy's command, Backspace clears it, Escape lets go without closing the page; Order
    /// 66's order key binds `order66`.
    #[test]
    fn a_toy_key_is_bound_from_its_card() {
        let mut panel = super::super::tests::drawn(
            Tab::Toys,
            &super::super::tests::inputs(unlockables::Holdings::Known(&[]), ""),
        );
        assert_eq!(panel.key(KeyCode::KeyK, false, true), PanelAction::None);
        assert_eq!(panel.bind_wait, Some(0));
        assert_eq!(
            panel.bind_key(KeyCode::KeyK, Some("k".to_owned()), 0),
            PanelAction::Bind {
                command: "toy_illuminate",
                key: "k".to_owned(),
            }
        );
        assert_eq!(panel.bind_wait, None, "one key, then it lets go");
        assert_eq!(
            panel.bind_key(KeyCode::Backspace, Some("BACKSPACE".to_owned()), 0),
            PanelAction::Unbind("toy_illuminate")
        );
        let _ = panel.key(KeyCode::KeyK, false, true);
        assert_eq!(
            panel.bind_key(KeyCode::Escape, Some("ESCAPE".to_owned()), 0),
            PanelAction::None,
            "Escape lets go and does not close the page"
        );
        assert_eq!(panel.bind_wait, None);
        // A click elsewhere lets go too; the card's key waits again on its button.
        panel.bind_wait = Some(0);
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &super::super::tests::inputs(unlockables::Holdings::Known(&[]), ""),
            &fonts.font,
            [1_920.0, 1_080.0],
        );
        let rect = panel.ui.rect_for(TOY_SWITCH_TOKEN).expect("the switch");
        let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        for event in [
            InputEvent::PointerMove(at),
            InputEvent::PointerPress {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
            InputEvent::PointerRelease {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
        ] {
            let _ = panel.handle_pointer(event, true);
        }
        assert_eq!(panel.bind_wait, None, "a click elsewhere lets go");
        panel.toy = 2;
        assert_eq!(
            panel.bind_key(KeyCode::KeyO, Some("o".to_owned()), 1),
            PanelAction::Bind {
                command: "order66",
                key: "o".to_owned(),
            }
        );
        assert_eq!(command(2, 1), Some("order66"));
        assert_eq!(command(0, 1), None);
        assert!(COMMANDS.iter().all(|command| {
            TOYS.iter()
                .any(|toy| toy.commands.iter().any(|(known, _)| known == command))
        }));
    }
}
