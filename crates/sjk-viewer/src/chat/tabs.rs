//! The channels the chat composer offers (29/10/2026): All always,
//! then Team, SJK, Admin and PM, each behind its own switch, so every player sets up the
//! row they use. Admin is JA+'s and jaPRO's `amsay`, a message only the server's admins
//! read (JoF EJK `Cmd_Amsay_f`, `SAY_ADMIN`); it is offered only on a server whose mod
//! has the command, and has its own key ([`ADMIN_COMMAND`]). PM whispers to the last
//! player who whispered you (`chat/reply.rs`). A hidden channel is only left off the row
//! and out of Tab: its key still opens it, and a whisper still opens from a name.
//!
//! Each button is also a tab, as in other games' chat windows (Dave, 29/10/2026): while
//! the composer is on Team, SJK, Admin or a whisper, the feed shows only that channel's
//! messages (whispers both ways, encrypted ones too), and All shows everything; the
//! feed shows everything again once the composer closes. [`FILTER_CVAR`] turns it off:
//! every chat stays in view, each line labelled with its channel, and the buttons only
//! pick where the message goes.
//!
//! The clan chats (03/11/2026, Sol's decision from Lex's idea, `docs/hub-chat.md`
//! "Clan chats"): JoF and S, hub chats like SJK, have a button each once the player
//! joined them ([`JOF_JOIN_CVAR`], [`SITH_JOIN_CVAR`]; the hub says who may). Since then
//! every switchable button but PM takes a third value, 2: shown, but left out of Tab's
//! cycle ([`Shown`]), so a player keeps Team or Admin clickable without Tab stopping on
//! it.
use super::*;
use crate::console::ViewerConsole;
use crate::text::UiFont;

/// Team chat in the composer's row.
pub(crate) const TEAM_CVAR: &str = "cg_chatTabTeam";
/// SJK chat in the composer's row.
pub(crate) const SJK_CVAR: &str = "cg_chatTabSjk";
/// Admin chat (`amsay`) in the composer's row, on servers that have it.
pub(crate) const ADMIN_CVAR: &str = "cg_chatTabAdmin";
/// JoF's clan chat in the composer's row, once joined.
pub(crate) const JOF_CVAR: &str = "cg_chatTabJof";
/// The Sith clan's chat (S) in the composer's row, once joined.
pub(crate) const SITH_CVAR: &str = "cg_chatTabS";
/// Whether the player reads JoF's clan chat: -1 (the default) when it is their home
/// channel (a JoF player's, or staff's), 1 joined, 0 left. The hub decides who may.
pub(crate) const JOF_JOIN_CVAR: &str = "cl_sjkChatJof";
/// [`JOF_JOIN_CVAR`] for the Sith clan's chat (S).
pub(crate) const SITH_JOIN_CVAR: &str = "cl_sjkChatS";
/// The key command that opens the composer on JoF's clan chat.
pub(crate) const JOF_COMMAND: &str = "messagemode7";
/// The key command that opens the composer on the Sith clan's chat (S).
pub(crate) const SITH_COMMAND: &str = "messagemode8";
/// A PM button (a whisper to the last whisperer) in the composer's row.
pub(crate) const PM_CVAR: &str = "cg_chatTabPm";
/// The key command that opens the composer on Admin chat, beside `messagemode5` (SJK).
pub(crate) const ADMIN_COMMAND: &str = "messagemode6";
/// Room between two of the row's words, in 1080p pixels at the chat's scale (58 px apart
/// whatever the word before 29/10/2026, which left wide holes after the short ones).
pub(super) const ROW_GAP: f32 = 22.0;
/// The row's text size, in 1080p pixels at the chat's scale.
const LABEL_SIZE: f32 = 14.0;

/// A channel's word in the row: All is the game's say chat.
pub(super) fn label(channel: Channel) -> &'static str {
    match channel {
        Channel::Global => "All",
        Channel::Team => "Team",
        Channel::Sjk => "SJK",
        Channel::Admin => "Admin",
        Channel::Whisper => "PM",
        Channel::Jof => "JoF",
        Channel::Sith => "S",
    }
}

/// A channel's button in the row.
pub(super) fn token(channel: Channel) -> u16 {
    match channel {
        Channel::Global => interaction::GLOBAL,
        Channel::Team => interaction::TEAM,
        Channel::Sjk => interaction::SJK,
        Channel::Admin => interaction::ADMIN,
        Channel::Whisper => interaction::PM,
        Channel::Jof => interaction::JOF,
        Channel::Sith => interaction::SITH,
    }
}

/// The composer's channel also filters the feed while it is open.
pub(crate) const FILTER_CVAR: &str = "cg_chatTabFilter";
/// What the composer says when PM is chosen and no one has whispered yet.
pub(super) const NO_PM: &str =
    "No one has whispered you yet: click a player's name to whisper them.";
/// What the composer says when the Admin key is used on a server without admin chat.
pub(super) const NO_ADMIN: &str = "This server has no admin chat (amsay is JA+ and jaPRO's).";
/// What the composer says when a clan chat's key is used without that chat joined.
pub(super) const NO_CLAN: &str =
    "You have not joined that clan chat: Settings > Chat, for that clan's verified players.";

/// How a switchable channel's button is kept: its cvar's 0, 1 or 2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Shown {
    /// 0: not in the row, and Tab skips it (its key still opens it).
    Off,
    /// 1: in the row and in Tab's cycle.
    On,
    /// 2: in the row, but Tab skips it.
    NotInTab,
}

impl Shown {
    fn of(value: Option<i64>, default: Self) -> Self {
        match value {
            Some(0) => Self::Off,
            Some(2) => Self::NotInTab,
            Some(_) => Self::On,
            None => default,
        }
    }

    /// Its button is in the row.
    pub(super) fn shown(self) -> bool {
        self != Self::Off
    }

    /// Tab stops on it.
    fn cycled(self) -> bool {
        self == Self::On
    }
}

/// The names [`Shown`]'s values have in Settings.
const SHOWN_CHOICES: [(&str, &str); 3] = [("0", "Off"), ("1", "On"), ("2", "Not in Tab")];
/// The names a clan chat's join setting's values have in Settings.
const JOIN_CHOICES: [(&str, &str); 3] = [("-1", "Auto"), ("0", "Left"), ("1", "Joined")];

/// The name Settings shows for `value` of one of this module's choice settings.
pub(crate) fn choice_name(cvar: &str, value: &str) -> Option<&'static str> {
    let choices: &[(&str, &str)] = if [TEAM_CVAR, SJK_CVAR, ADMIN_CVAR, JOF_CVAR, SITH_CVAR]
        .iter()
        .any(|name| name.eq_ignore_ascii_case(cvar))
    {
        &SHOWN_CHOICES
    } else if [JOF_JOIN_CVAR, SITH_JOIN_CVAR]
        .iter()
        .any(|name| name.eq_ignore_ascii_case(cvar))
    {
        &JOIN_CHOICES
    } else {
        return None;
    };
    choices
        .iter()
        .find(|(choice, _)| *choice == value.trim())
        .map(|(_, name)| *name)
}

/// Which of the switchable channels the player keeps in the row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Tabs {
    pub(super) team: Shown,
    pub(super) sjk: Shown,
    pub(super) admin: Shown,
    pub(super) jof: Shown,
    pub(super) sith: Shown,
    pub(super) pm: bool,
    /// The composer's channel filters the feed ([`FILTER_CVAR`]).
    pub(super) filter: bool,
}

impl Tabs {
    pub(super) fn read(console: Option<&ViewerConsole>) -> Self {
        let value = |name| console.and_then(|console| console.integer_cvar(name));
        let on = |name, default| value(name).map_or(default, |value| value != 0);
        Self {
            team: Shown::of(value(TEAM_CVAR), Shown::On),
            sjk: Shown::of(value(SJK_CVAR), Shown::On),
            admin: Shown::of(value(ADMIN_CVAR), Shown::On),
            jof: Shown::of(value(JOF_CVAR), Shown::On),
            sith: Shown::of(value(SITH_CVAR), Shown::On),
            pm: on(PM_CVAR, false),
            filter: on(FILTER_CVAR, true),
        }
    }

    /// How `channel`'s button is kept; All is always on, PM by its own switch.
    pub(super) fn of(self, channel: Channel) -> Shown {
        match channel {
            Channel::Global => Shown::On,
            Channel::Team => self.team,
            Channel::Sjk => self.sjk,
            Channel::Admin => self.admin,
            Channel::Jof => self.jof,
            Channel::Sith => self.sith,
            Channel::Whisper if self.pm => Shown::On,
            Channel::Whisper => Shown::Off,
        }
    }
}

/// The player's choice for clan chat `channel` from its join cvar: joined, left, or
/// `None` for the default (joined when it is their home channel).
pub(crate) fn join_choice(
    console: &ViewerConsole,
    channel: sjk_identity::HubChannel,
) -> Option<bool> {
    let cvar = match channel {
        sjk_identity::HubChannel::Jof => JOF_JOIN_CVAR,
        sjk_identity::HubChannel::Sith => SITH_JOIN_CVAR,
        _ => return None,
    };
    match console.integer_cvar(cvar) {
        Some(0) => Some(false),
        Some(1) => Some(true),
        _ => None,
    }
}

/// Whether a server of this kind takes `amsay`: JA+ and jaPRO (and its TaystJK lineage,
/// JoF's servers among them).
pub(crate) fn server_has_admin_chat(kind: sjk_mod::ServerKind) -> bool {
    matches!(
        kind,
        sjk_mod::ServerKind::JaPlus | sjk_mod::ServerKind::JaPro
    )
}

/// Whether a game chat line is jaPRO's admin chat: its name starts `<Admin>` (JoF EJK
/// `G_Say`, `SAY_ADMIN`: `^3<Admin>^7(name)`), colour codes aside.
pub(super) fn is_admin_line(display: &str) -> bool {
    super::options::clean_body(display, 1)
        .trim_start()
        .starts_with("<Admin>(")
}

/// Every channel the row may hold, in its order: the hub's chats after Team, the server's
/// admins and whispers last.
pub(super) const ROW: [Channel; 7] = [
    Channel::Global,
    Channel::Team,
    Channel::Sjk,
    Channel::Jof,
    Channel::Sith,
    Channel::Admin,
    Channel::Whisper,
];

impl ChatOverlay {
    /// Whether the feed shows `line` now: the PM-only switch (`chat/pm_only.rs`), then
    /// the open composer's tab.
    pub(super) fn shown(&self, line: &ChatLine) -> bool {
        if !self.pm_visible(line) || !self.quiet_visible(line) {
            return false;
        }
        let Some(input) = self.input.as_ref().filter(|_| self.options.tabs.filter) else {
            return true;
        };
        match input.channel {
            Channel::Global => true,
            Channel::Whisper => {
                line.channel == Channel::Whisper || line.encrypted || line.server_private
            }
            channel => line.channel == channel,
        }
    }

    /// The channels the composer's row offers now, in order.
    pub(super) fn offered(&self) -> impl Iterator<Item = Channel> + '_ {
        ROW.into_iter().filter(|channel| self.offers(*channel))
    }

    /// Whether the row offers `channel`.
    pub(super) fn offers(&self, channel: Channel) -> bool {
        self.options.tabs.of(channel).shown() && self.available(channel)
    }

    /// Whether `channel` can be used here, whatever its button: Admin on a server that
    /// takes `amsay`, a clan chat once joined.
    pub(super) fn available(&self, channel: Channel) -> bool {
        match channel {
            Channel::Admin => self.admin_server,
            Channel::Jof => self.clans[0],
            Channel::Sith => self.clans[1],
            _ => true,
        }
    }

    /// Say which clan chats the player reads now (`sjk_identity::channels::joined`),
    /// JoF's then S's. Leaving the one the composer is on moves it to All.
    pub(crate) fn set_clan_chats(&mut self, joined: [bool; 2]) {
        self.clans = joined;
        let left = self
            .input
            .as_ref()
            .is_some_and(|input| input.channel.clan().is_some() && !self.available(input.channel));
        if left && let Some(input) = self.input.as_mut() {
            input.channel = Channel::Global;
        }
    }

    /// Open the composer on clan chat `clan` for its key, whether or not its button is
    /// in the row; without it joined, on All with a word why.
    pub(crate) fn open_clan(&mut self, clan: sjk_identity::HubChannel) {
        let channel = Channel::of_hub(clan);
        self.open(false);
        if self.available(channel) {
            if let Some(input) = self.input.as_mut() {
                input.channel = channel;
            }
        } else {
            self.notice = NO_CLAN;
        }
    }

    /// Say whether the server being played takes `amsay`. Leaving one that does moves a
    /// composer on Admin to All.
    pub(crate) fn set_admin_server(&mut self, admin: bool) {
        self.admin_server = admin;
        if !admin
            && let Some(input) = self.input.as_mut()
            && input.channel == Channel::Admin
        {
            input.channel = Channel::Global;
        }
    }

    /// The channel Tab moves to from `channel`: the next one in the row Tab stops on
    /// ([`Shown`]), then the last whisperer (`chat/reply.rs`) when the row has no PM
    /// button, then All again. `None` is the last whisperer.
    pub(super) fn next_channel(&self, channel: Channel) -> Option<Channel> {
        let mut row = self.offered().skip_while(|offered| *offered != channel);
        row.next();
        let tabs = self.options.tabs;
        for next in row.filter(|next| tabs.of(*next).cycled()) {
            match next {
                // The PM button stands for the last whisperer, skipped when there is none.
                Channel::Whisper if self.reply_target().is_ok() => return None,
                Channel::Whisper => {}
                next => return Some(next),
            }
        }
        if channel != Channel::Whisper
            && !self.offers(Channel::Whisper)
            && self.reply_target().is_ok()
        {
            return None;
        }
        Some(Channel::Global)
    }

    /// The step of Tab's cycle before `channel`, with [`Self::next_channel`]'s `None`;
    /// a channel outside the cycle (a hidden Team opened by its key) goes to its end.
    pub(super) fn previous_channel(&self, channel: Channel) -> Option<Channel> {
        let step = |from| self.next_channel(from).unwrap_or(Channel::Whisper);
        let mut before = Channel::Global;
        let mut at = step(before);
        // The cycle only moves along the row, so it comes back to All.
        while at != Channel::Global && at != channel {
            before = at;
            at = step(at);
        }
        (before != Channel::Whisper).then_some(before)
    }

    /// The players who whispered you, newest first, each once, while they are still
    /// in the game: the PM button's choices.
    pub(super) fn whisperers(&self) -> Vec<ChatTarget> {
        let mut found: Vec<ChatTarget> = Vec::new();
        let senders = self.last_whisperer.into_iter().chain(
            self.lines
                .iter()
                .rev()
                .filter(|line| line.channel == Channel::Whisper)
                .filter_map(|line| line.sender),
        );
        for sender in senders {
            if Some(sender.slot()) != self.own_slot
                && self.roster.name(sender).is_some()
                && !found.contains(&sender)
            {
                found.push(sender);
            }
        }
        found
    }

    /// The PM button: a whisper to the last whisperer. Left-clicked again while on one
    /// (`step` 1), the next player who whispered you, round to the first; right-clicked
    /// (`step` -1), the one before, round to the last. With no whisperer, a word on how
    /// to whisper.
    pub(super) fn step_pm(&mut self, step: isize) {
        let whisperers = self.whisperers();
        if whisperers.is_empty() {
            self.notice = NO_PM;
            return;
        }
        let Some(input) = self.input.as_mut() else {
            return;
        };
        let current = match (input.channel, input.recipient) {
            (Channel::Whisper, Some(current)) => {
                whisperers.iter().position(|target| *target == current)
            }
            _ => None,
        };
        let count = whisperers.len() as isize;
        let at = current.map_or(0, |at| (at as isize + step).rem_euclid(count) as usize);
        input.channel = Channel::Whisper;
        input.recipient = Some(whisperers[at]);
        input.pm_list = false;
        self.notice = "";
    }

    /// Where the composer's whisper stands among the players who whispered you, as
    /// "2/3", when more than one did and it is one of them.
    #[cfg(test)]
    pub(super) fn pm_place(&self) -> Option<(usize, usize)> {
        self.pm_place_among(&self.whisperers())
    }

    /// [`Self::pm_place`] among `whisperers`, [`Self::whisperers`] as already found.
    pub(super) fn pm_place_among(&self, whisperers: &[ChatTarget]) -> Option<(usize, usize)> {
        let recipient = self.input.as_ref()?.recipient?;
        (whisperers.len() > 1)
            .then(|| whisperers.iter().position(|target| *target == recipient))
            .flatten()
            .map(|at| (at + 1, whisperers.len()))
    }

    /// Open the composer on Admin chat for its key, whether or not its button is in the
    /// row; on a server without it, on All with a word why.
    pub(crate) fn open_admin(&mut self) {
        self.open(false);
        if self.admin_server {
            if let Some(input) = self.input.as_mut() {
                input.channel = Channel::Admin;
            }
        } else {
            self.notice = NO_ADMIN;
        }
    }

    /// The row as drawn: each offered channel with its left edge and its word's width,
    /// in 1080p pixels at the chat's scale, and where the row ends (the whisper's
    /// "To ..." and the SJK chat's red note start there).
    pub(super) fn row_layout(
        &self,
        font: &UiFont,
    ) -> ([(Channel, f32, f32); ROW.len()], usize, f32) {
        let mut row = [(Channel::Global, 0.0, 0.0); ROW.len()];
        let mut count = 0;
        let mut x = 0.0;
        for channel in self.offered() {
            let width = crate::text::visible_text_width_face(
                font,
                label(channel),
                LABEL_SIZE / font.height,
                crate::text::TextFace::Semibold,
            );
            row[count] = (channel, x, width);
            count += 1;
            x += width + ROW_GAP;
        }
        (row, count, x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_ui::{InputEvent, PointerButton};

    fn chat(team: bool, sjk: bool, admin: bool, server: bool) -> ChatOverlay {
        let shown = |on| if on { Shown::On } else { Shown::Off };
        let mut chat = ChatOverlay::new();
        chat.options.tabs = Tabs {
            team: shown(team),
            sjk: shown(sjk),
            admin: shown(admin),
            jof: Shown::On,
            sith: Shown::On,
            pm: false,
            filter: true,
        };
        chat.admin_server = server;
        chat
    }

    /// The labels the open composer draws in its row, after a frame.
    fn drawn(chat: &mut ChatOverlay) -> Vec<String> {
        chat.build(true, &crate::text::test_font(), [1920.0, 1080.0], 1_000);
        ["All", "Team", "SJK", "Admin", "PM"]
            .into_iter()
            .filter(|label| chat.ui.text_runs().any(|text| text == *label))
            .map(str::to_owned)
            .collect()
    }

    fn click(chat: &mut ChatOverlay, token: u16) {
        press(chat, token, PointerButton::Primary);
    }

    fn press(chat: &mut ChatOverlay, token: u16, button: PointerButton) {
        chat.build(true, &crate::text::test_font(), [1920.0, 1080.0], 1_000);
        let rect = chat.ui.rect_for(token).expect("a button");
        let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        chat.pointer(InputEvent::PointerMove(at));
        chat.pointer(InputEvent::PointerPress {
            position: at,
            button,
        });
        chat.pointer(InputEvent::PointerRelease {
            position: at,
            button,
        });
    }

    #[test]
    fn the_row_keeps_all_and_the_channels_switched_on() {
        let every = chat(true, true, true, true);
        assert_eq!(
            every.offered().collect::<Vec<_>>(),
            [Channel::Global, Channel::Team, Channel::Sjk, Channel::Admin]
        );
        // A row without SJK or Team chat.
        let lean = chat(false, false, true, true);
        assert_eq!(
            lean.offered().collect::<Vec<_>>(),
            [Channel::Global, Channel::Admin]
        );
        let font = crate::text::test_font();
        let (_, _, lean_end) = lean.row_layout(&font);
        let (row, count, every_end) = every.row_layout(&font);
        assert!(lean_end < every_end);
        // An even gap between the words, whatever their length.
        for pair in row[..count].windows(2) {
            assert!((pair[1].1 - (pair[0].1 + pair[0].2) - ROW_GAP).abs() < 0.01);
        }
        // Admin only where the server takes amsay.
        let base = chat(true, true, true, false);
        assert!(!base.offers(Channel::Admin));
        assert_eq!(base.row_layout(&font).1, 3);
    }

    #[test]
    fn the_settings_switch_the_drawn_buttons() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut chat = ChatOverlay::new();
        chat.set_admin_server(true);
        chat.configure(Some(&console));
        chat.open(false);
        assert_eq!(drawn(&mut chat), ["All", "Team", "SJK", "Admin"]);
        assert!(console.set_cvar(TEAM_CVAR, "0"));
        assert!(console.set_cvar(SJK_CVAR, "0"));
        assert!(console.set_cvar(PM_CVAR, "1"));
        chat.configure(Some(&console));
        assert_eq!(drawn(&mut chat), ["All", "Admin", "PM"]);
        assert!(console.set_cvar(ADMIN_CVAR, "0"));
        chat.configure(Some(&console));
        assert_eq!(drawn(&mut chat), ["All", "PM"]);
    }

    #[test]
    fn the_admin_button_can_be_clicked() {
        let mut chat = chat(true, true, true, true);
        chat.open(false);
        click(&mut chat, interaction::ADMIN);
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Admin);
        click(&mut chat, interaction::TEAM);
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Team);
    }

    #[test]
    fn the_pm_button_whispers_the_last_whisperer_or_says_how() {
        let mut chat = chat(true, true, true, true);
        chat.options.tabs.pm = true;
        chat.open(false);
        click(&mut chat, interaction::PM);
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Global);
        assert_eq!(chat.notice, NO_PM);
    }

    #[test]
    fn tab_walks_only_the_offered_channels() {
        let lean = chat(false, true, true, true);
        assert_eq!(lean.next_channel(Channel::Global), Some(Channel::Sjk));
        assert_eq!(lean.next_channel(Channel::Sjk), Some(Channel::Admin));
        assert_eq!(lean.next_channel(Channel::Admin), Some(Channel::Global));
        // The team key opened Team though it is hidden: Tab goes back to All.
        assert_eq!(lean.next_channel(Channel::Team), Some(Channel::Global));
        assert_eq!(lean.next_channel(Channel::Whisper), Some(Channel::Global));
    }

    #[test]
    fn the_admin_key_opens_admin_chat_even_with_its_button_hidden() {
        let mut hidden = chat(true, true, false, true);
        hidden.open_admin();
        assert_eq!(hidden.input.as_ref().unwrap().channel, Channel::Admin);
        hidden.set_admin_server(true);
        assert_eq!(hidden.input.as_ref().unwrap().channel, Channel::Admin);
        // Not on a server without it.
        let mut base = chat(true, true, true, false);
        base.open_admin();
        assert_eq!(base.input.as_ref().unwrap().channel, Channel::Global);
        assert_eq!(base.notice, NO_ADMIN);
    }

    #[test]
    fn the_admin_key_is_a_bindable_control() {
        assert!(crate::input::GameplayInput::recognizes(ADMIN_COMMAND));
    }

    #[test]
    fn the_switches_default_on_but_pm_and_read_zero_as_off() {
        assert_eq!(
            Tabs::read(None),
            Tabs {
                team: Shown::On,
                sjk: Shown::On,
                admin: Shown::On,
                jof: Shown::On,
                sith: Shown::On,
                pm: false,
                filter: true
            }
        );
    }

    #[test]
    fn clan_chats_are_offered_once_joined_and_tab_skips_what_it_should() {
        let mut chat = chat(true, true, true, true);
        assert!(!chat.offers(Channel::Jof), "not joined");
        chat.set_clan_chats([true, true]);
        assert_eq!(
            chat.offered().collect::<Vec<_>>(),
            [
                Channel::Global,
                Channel::Team,
                Channel::Sjk,
                Channel::Jof,
                Channel::Sith,
                Channel::Admin
            ]
        );
        // Team and Admin kept as buttons, out of Tab's cycle.
        chat.options.tabs.team = Shown::NotInTab;
        chat.options.tabs.admin = Shown::NotInTab;
        assert!(chat.offers(Channel::Team));
        assert_eq!(chat.next_channel(Channel::Global), Some(Channel::Sjk));
        assert_eq!(chat.next_channel(Channel::Sjk), Some(Channel::Jof));
        assert_eq!(chat.next_channel(Channel::Sith), Some(Channel::Global));
        // Clicked onto a button Tab skips, Tab goes on from there.
        assert_eq!(chat.next_channel(Channel::Team), Some(Channel::Sjk));
        // Leaving the chat the composer is on moves it to All.
        chat.open(false);
        chat.choose(Channel::Sith);
        chat.set_clan_chats([true, false]);
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Global);
        assert!(!chat.offers(Channel::Sith));
        // Its key says why it does not open.
        chat.open_clan(sjk_identity::HubChannel::Sith);
        assert_eq!(chat.notice, NO_CLAN);
        chat.open_clan(sjk_identity::HubChannel::Jof);
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Jof);
    }

    #[test]
    fn a_clan_chat_message_goes_to_the_hub() {
        let mut chat = chat(true, true, true, true);
        chat.set_clan_chats([false, true]);
        chat.open(false);
        click(&mut chat, interaction::SITH);
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Sith);
        chat.input.as_mut().unwrap().text = " for the clan ".to_owned();
        assert!(matches!(
            chat.submit(),
            ChatInputResult::Clan(text, sjk_identity::HubChannel::Sith) if text == "for the clan"
        ));
        assert!(chat.input.is_none());
    }

    #[test]
    fn the_clan_keys_are_bindable_controls() {
        for command in [JOF_COMMAND, SITH_COMMAND] {
            assert!(crate::input::GameplayInput::recognizes(command));
        }
        assert_eq!(choice_name(TEAM_CVAR, "2"), Some("Not in Tab"));
        assert_eq!(choice_name(SITH_JOIN_CVAR, "-1"), Some("Auto"));
        assert_eq!(choice_name("cg_drawChat", "1"), None);
    }

    #[test]
    fn button_switches_read_off_on_and_not_in_tab() {
        assert_eq!(Shown::of(Some(0), Shown::On), Shown::Off);
        assert_eq!(Shown::of(Some(1), Shown::Off), Shown::On);
        assert_eq!(Shown::of(Some(2), Shown::On), Shown::NotInTab);
        assert_eq!(Shown::of(None, Shown::On), Shown::On);
        assert!(Shown::NotInTab.shown() && !Shown::NotInTab.cycled());
    }

    #[test]
    fn admin_chat_is_offered_on_ja_plus_and_japro_servers() {
        use sjk_mod::ServerKind;
        assert!(server_has_admin_chat(ServerKind::JaPlus));
        assert!(server_has_admin_chat(ServerKind::JaPro));
        assert!(!server_has_admin_chat(ServerKind::BaseJka));
        assert!(!server_has_admin_chat(ServerKind::Other));
    }

    #[test]
    fn leaving_an_admin_server_moves_the_composer_to_all() {
        let mut chat = chat(true, true, true, true);
        chat.open(false);
        chat.input.as_mut().unwrap().channel = Channel::Admin;
        chat.set_admin_server(false);
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Global);
    }

    #[test]
    fn admin_chat_goes_out_as_amsay() {
        assert_eq!(
            sjk_client::chat_command(sjk_client::ChatDestination::Admin, "need help \"here\"")
                .as_deref(),
            Some("amsay \"need help ''here''\"")
        );
        let mut chat = chat(true, true, true, true);
        chat.open(false);
        chat.choose(Channel::Admin);
        chat.input.as_mut().unwrap().text = "hi admins".to_owned();
        assert!(matches!(
            chat.submit(),
            ChatInputResult::Submit(command) if command == "amsay \"hi admins\""
        ));
    }

    fn feed(chat: &mut ChatOverlay) -> Vec<String> {
        chat.build(true, &crate::text::test_font(), [1920.0, 1080.0], 1_000);
        ["public", "teamwords", "adminwords", "whisperwords"]
            .into_iter()
            .filter(|word| chat.ui.text_runs().any(|text| text.contains(word)))
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn each_tab_shows_only_its_own_messages_while_typing() {
        let mut chat = chat(true, true, true, true);
        chat.options.lines = 10;
        let now = std::time::Instant::now();
        for (kind, text, private) in [
            (sjk_client::ServerEventKind::Chat, "Fox: public", false),
            (
                sjk_client::ServerEventKind::TeamChat,
                "(Fox): teamwords",
                false,
            ),
            (
                sjk_client::ServerEventKind::Chat,
                "^3<Admin>^7(Fox^7)^7: ^2adminwords",
                false,
            ),
            (
                sjk_client::ServerEventKind::Chat,
                "\u{19}[Fox]: whisperwords",
                true,
            ),
        ] {
            chat.receive_classified(kind, text.to_owned(), None, private, now);
        }
        assert!(chat.lines.iter().any(|line| line.channel == Channel::Admin));
        // Closed: everything, as before.
        assert_eq!(feed(&mut chat).len(), 4);
        chat.open(false);
        assert_eq!(feed(&mut chat).len(), 4, "All shows everything");
        chat.choose(Channel::Team);
        assert_eq!(feed(&mut chat), ["teamwords"]);
        chat.choose(Channel::Admin);
        assert_eq!(feed(&mut chat), ["adminwords"]);
        chat.input.as_mut().unwrap().channel = Channel::Whisper;
        assert_eq!(feed(&mut chat), ["whisperwords"]);
        // The switch off: the tabs only choose where a message goes.
        chat.options.tabs.filter = false;
        assert_eq!(feed(&mut chat).len(), 4);
    }

    #[test]
    fn admin_lines_are_told_from_their_admin_name() {
        assert!(is_admin_line("^3<Admin>^7(Fox^7)^7: ^2hi"));
        assert!(!is_admin_line("Fox: <Admin>(nope)"));
        assert!(!is_admin_line("Fox: hi"));
    }

    #[test]
    fn the_pm_button_steps_through_everyone_who_whispered() {
        let mut chat = chat(true, true, true, true);
        chat.options.tabs.pm = true;
        let mut game = sjk_protocol::GameState::empty_local(0);
        for (slot, name) in [(3usize, "Fox"), (4, "Wolf"), (5, "Bear")] {
            game.replace_config_string(1_131 + slot, format!("n\\{name}\\t\\0").into_bytes())
                .expect("a player");
        }
        chat.update_roster(&game);
        let now = std::time::Instant::now();
        for (name, slot) in [("Fox", 3u16), ("Wolf", 4), ("Bear", 5)] {
            chat.receive_classified(
                sjk_client::ServerEventKind::Chat,
                format!("\u{19}[{name}\u{19}]: hi"),
                Some(slot),
                true,
                now,
            );
        }
        let names = |chat: &ChatOverlay| {
            chat.whisperers()
                .iter()
                .map(|target| chat.roster.name(*target).unwrap().to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&chat), ["Bear", "Wolf", "Fox"], "newest first");
        chat.open(false);
        let recipient = |chat: &ChatOverlay| {
            let target = chat.input.as_ref().unwrap().recipient.unwrap();
            chat.roster.name(target).unwrap().to_owned()
        };
        chat.step_pm(1);
        assert_eq!(recipient(&chat), "Bear");
        assert_eq!(chat.pm_place(), Some((1, 3)));
        chat.step_pm(1);
        assert_eq!(recipient(&chat), "Wolf");
        chat.step_pm(1);
        assert_eq!(recipient(&chat), "Fox");
        chat.step_pm(1);
        assert_eq!(recipient(&chat), "Bear", "round to the first");
        // A right click goes back, round to the last.
        chat.step_pm(-1);
        assert_eq!(recipient(&chat), "Fox");
        chat.step_pm(-1);
        assert_eq!(recipient(&chat), "Wolf");
        // The same with the mouse on the PM button.
        click(&mut chat, interaction::PM);
        assert_eq!(recipient(&chat), "Fox");
        press(&mut chat, interaction::PM, PointerButton::Secondary);
        assert_eq!(recipient(&chat), "Wolf");
    }

    #[test]
    fn choosing_a_tab_with_the_pointer_still_on_it_opens_no_card() {
        let mut chat = chat(true, true, true, true);
        chat.options.lines = 10;
        let mut game = sjk_protocol::GameState::empty_local(0);
        for (slot, name) in [(1usize, "ArdanHett"), (2, "Padawan")] {
            game.replace_config_string(1_131 + slot, format!("n\\{name}\\t\\0").into_bytes())
                .expect("a player");
        }
        chat.update_roster(&game);
        let now = std::time::Instant::now();
        for text in ["ArdanHett: hi", "Padawan: yo", "Padawan: there"] {
            chat.receive_classified(
                sjk_client::ServerEventKind::Chat,
                text.to_owned(),
                None,
                false,
                now,
            );
        }
        chat.open(false);
        // Team hides them all; then All, clicked where the pointer rests.
        click(&mut chat, interaction::TEAM);
        click(&mut chat, interaction::GLOBAL);
        let font = crate::text::test_font();
        for ms in [1_100, 1_200, 1_500] {
            chat.build(true, &font, [1920.0, 1080.0], ms);
            assert!(chat.card.is_none(), "no player's card at {ms}");
        }
    }
}
