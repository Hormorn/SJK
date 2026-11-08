//! Looks (`docs/unlockables.md`, "Receiving"): what each player on the game server
//! wears that SJK draws, the blade skin and whether their Illuminate holocron is lit.
//! Other players' looks come through the SJK hub ([`sjk_identity::Look`]): from the
//! presence roster when it changes and from the feed's look events as they come. The
//! local player's own look comes from its own settings (`cg_saberSkin`, gated by the
//! unlocks of its hub profile, and its Illuminate), never from the hub. Blade skins are
//! named by their ids in the catalogue ([`crate::unlockables`]).
//!
//! A hub look counts only while the game shows, in its slot, the name the claim was
//! made under (the badges' and emotes' rule, [`sjk_identity::names_match`]). The
//! table is rebuilt when a roster or an event comes and twice a second, so a player
//! who leaves a slot loses the look at once for the renderer; a frame only reads the
//! fixed [`Worn`] table.
//!
//! The renderer reads [`Looks::saber_skin_id`], [`Looks::illuminated`] and
//! [`Looks::own_saber_skin`] on `GpuState::looks`, and [`Looks::revision`] tells it
//! when what is worn changed (`GpuState::sync_saber_skins`). A player muted on this PC
//! (`docs/hub-chat.md`, "Muting a player") wears nothing: the stock blade, its sounds,
//! no holocron ([`Looks::set_muted`]).

use sjk_identity::Look;
use sjk_protocol::{GameState, PlayerState};

/// Slots the table holds, as the emotes' do.
pub(crate) const SLOTS: usize = 64;

/// Whose slots a session's view involves: the local player's own (the game state's
/// `client_num`) and the one whose player state the snapshot carries. They differ
/// while the local player follows (spectates) someone: the server then sends the
/// followed player's state, flagged `PMF_FOLLOW`, under their `client_num`, and leaves
/// their entity out of the snapshot. The own look belongs in the own slot only; the
/// followed player wears their hub look.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ViewSlots {
    /// The local player's own slot.
    pub(crate) own: Option<usize>,
    /// The slot whose player state the view shows.
    pub(crate) viewed: usize,
}

impl ViewSlots {
    /// The slots of a session with `game_state` whose latest player state is `player`.
    pub(crate) fn of(game_state: &GameState, player: &PlayerState) -> Self {
        Self {
            own: usize::try_from(game_state.client_num).ok(),
            viewed: usize::from(player.client_num()),
        }
    }

    /// The player the view follows, when it is not the local player's own.
    pub(crate) fn followed(self) -> Option<usize> {
        (Some(self.viewed) != self.own).then_some(self.viewed)
    }
}

/// The catalogue's id of the blade skin `id` names, if this client knows it; a look
/// naming any other id draws the stock blade.
fn blade_skin(id: &str) -> Option<&'static str> {
    crate::unlockables::blade_skin(id).map(|unlockable| unlockable.id)
}

/// The toys a look can have out that this client draws, by id (`PROTOCOL.md`, "Looks"):
/// a toy's bit in [`Worn::toys`] is its place here. A look naming another id (a newer
/// client's toy) draws nothing more.
pub(crate) const TOYS: [&str; 12] = [
    crate::cardboard_box::TOY,
    crate::order66::TOY,
    crate::hax::TOY,
    crate::hax::AWAY_TOY,
    crate::drone::TOY,
    crate::boombox::TOY,
    // The Monitor's switch: others foresee its monitors striking this player.
    crate::monitor::TOY,
    // Box hunt's switch: the hub picks the mark among the players whose look has it.
    crate::box_hunt::TOY,
    // The Banana peel's switch (since 30/10/2026: hubs before took 8 toy ids at most).
    crate::banana::TOY,
    // The fishing rod out: others draw the rod, the bucket and the float.
    crate::fishing::TOY,
    // The wearer's model in toon shading, their choice (`cg_toonModel`).
    crate::toon_choice::TOY,
    // The pickaxe in hand: others draw it.
    crate::mining::TOY,
];
// A bit each in a `u32`: as many as a hub look holds (32 toy ids since 30/10/2026).
const _: () = assert!(TOYS.len() <= u32::BITS as usize);

/// The bit of toy `id` in [`Worn::toys`], if this client knows it.
fn toy_bit(id: &str) -> Option<u32> {
    TOYS.iter()
        .position(|toy| *toy == id)
        .map(|place| 1 << place)
}

/// What one player wears, as this client draws it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Worn {
    /// The blade skin, a catalogue id; `None` for the stock blade.
    pub(crate) saber_skin: Option<&'static str>,
    /// The Illuminate holocron is lit.
    pub(crate) illuminate: bool,
    /// The blade skin's options its wearer switched off (`crate::saber_skin_options`).
    pub(crate) options_off: crate::saber_skin_options::OptionsOff,
    /// The toys out, a bit per [`TOYS`] entry.
    pub(crate) toys: u32,
    /// What the second saber wears (since 21/10/2026: `saber2`).
    pub(crate) second: SecondSkin,
    /// The second saber's skin's options its wearer switched off.
    pub(crate) options_off2: crate::saber_skin_options::OptionsOff,
    /// The saber sounds chosen apart from the blade (`crate::saber_sound_choice`).
    pub(crate) sounds: crate::saber_sound_choice::SoundChoice,
    /// The finisher worn (`crate::finishers`), a catalogue id; `None` for none.
    pub(crate) finisher: Option<&'static str>,
}

/// Archived: the local player's second saber's blade skin, for two sabers: empty for the
/// first saber's (`cg_saberSkin`), [`STOCK_SECOND`] for the stock blade, else a blade
/// skin's id, worn only while the profile lists it (else the stock blade, as the first
/// saber's skin and as the hub makes a second saber whose skin is taken back).
pub(crate) const SECOND_CVAR: &str = "cg_saberSkin2";
/// [`SECOND_CVAR`]'s value for a stock second blade.
pub(crate) const STOCK_SECOND: &str = "stock";

/// What a player's second saber wears: the first saber's skin, or its own (`None`: the
/// stock blade), a catalogue id.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SecondSkin {
    #[default]
    Same,
    Own(Option<&'static str>),
}

impl SecondSkin {
    /// From a look's `saber2`: absent is the first's skin, `""` (or a skin this client does
    /// not know) the stock blade.
    fn of(saber2: Option<&str>) -> Self {
        saber2.map_or(Self::Same, |id| Self::Own(blade_skin(id)))
    }

    /// The local choice in [`SECOND_CVAR`], with `owned` saying which skins the profile
    /// lists. The first saber's skin chosen for the second stays the second's own, so it
    /// does not change when the first does. A skin the profile does not list (taken back,
    /// or never held) is the stock blade, as the first saber's is and as the hub makes a
    /// second saber's skin taken back (`saber2` `""`), so the player sees what others see
    /// (review of 22/10/2026: it followed the first here and was stock for the others).
    fn own(setting: &str, owned: impl Fn(&str) -> bool) -> Self {
        match setting.trim() {
            "" => Self::Same,
            STOCK_SECOND => Self::Own(None),
            id => Self::Own(blade_skin(id).filter(|id| owned(id))),
        }
    }
}

impl Worn {
    /// What `look` draws as here: an unknown skin is the stock blade, an unknown toy
    /// nothing.
    pub(crate) fn of(look: &Look) -> Self {
        let saber_skin = blade_skin(&look.saber);
        Self {
            saber_skin,
            illuminate: look.illuminate,
            options_off: saber_skin.map_or(0, |skin| {
                crate::saber_skin_options::look_bits(
                    skin,
                    look.saber_off.iter().map(String::as_str),
                    look.saber_on.iter().map(String::as_str),
                )
            }),
            second: SecondSkin::of(look.saber2.as_deref()),
            options_off2: crate::saber_skin_options::look_bits(
                look.saber2.as_deref().unwrap_or(&look.saber),
                look.saber2_off.iter().map(String::as_str),
                look.saber2_on.iter().map(String::as_str),
            ),
            toys: look
                .toys
                .iter()
                .filter_map(|id| toy_bit(id))
                .fold(0, |toys, bit| toys | bit),
            sounds: crate::saber_sound_choice::SoundChoice::parse(&look.saber_sounds),
            finisher: crate::finishers::get(&look.finisher).map(|finisher| finisher.id),
        }
    }

    /// The same look with the finisher `setting` (`cg_saberFinisher`) chooses, while the
    /// own profile lists it (`owned`).
    pub(crate) fn with_finisher(mut self, setting: &str, owned: impl Fn(&str) -> bool) -> Self {
        self.finisher = crate::finishers::own(setting, owned);
        self
    }

    /// The blade skin whose sounds play for this player's sabers (`None` the stock
    /// sounds): the blade's, unless they chose others.
    pub(crate) fn sound_skin(&self) -> Option<&'static str> {
        self.sounds.sound_skin(self.saber_skin)
    }

    /// The same look with the saber sounds `setting` (`cg_saberSounds`) chooses, a
    /// shader's only while the own profile lists it (`owned`).
    pub(crate) fn with_sounds(mut self, setting: &str, owned: impl Fn(&str) -> bool) -> Self {
        self.sounds = crate::saber_sound_choice::SoundChoice::own(setting, owned);
        self
    }

    /// Whether toy `id` is out.
    pub(crate) fn has_toy(&self, id: &str) -> bool {
        toy_bit(id).is_some_and(|bit| self.toys & bit != 0)
    }

    /// The same look with toy `id` out (`true`) or not.
    pub(crate) fn with_toy(mut self, id: &str, out: bool) -> Self {
        if let Some(bit) = toy_bit(id) {
            self.toys = if out {
                self.toys | bit
            } else {
                self.toys & !bit
            };
        }
        self
    }

    /// The look the hub is told the local player wears: the skin only when the own
    /// profile lists it (`owned`), with the options `options` (`cg_saberSkinOptions`)
    /// switches off for it, and the local holocron's lit state.
    pub(crate) fn own(
        skin_setting: &str,
        options: &str,
        owned: impl Fn(&str) -> bool,
        lit: bool,
    ) -> Self {
        let saber_skin = blade_skin(skin_setting).filter(|skin| owned(skin));
        Self {
            saber_skin,
            illuminate: lit,
            options_off: saber_skin
                .map_or(0, |skin| crate::saber_skin_options::off_bits(options, skin)),
            toys: 0,
            second: SecondSkin::Same,
            options_off2: 0,
            sounds: Default::default(),
            finisher: None,
        }
    }

    /// The same look with the second saber wearing what `setting` ([`SECOND_CVAR`]) says,
    /// when the profile lists it (`owned`), with the options `options` switches off for it.
    pub(crate) fn with_second(
        self,
        setting: &str,
        options: &str,
        owned: impl Fn(&str) -> bool,
    ) -> Self {
        let second = SecondSkin::own(setting, owned);
        let options_off2 = match second {
            SecondSkin::Own(Some(skin)) => crate::saber_skin_options::off_bits(options, skin),
            _ => 0,
        };
        Self {
            second,
            options_off2,
            ..self
        }
    }

    /// The wire form, for `Service::set_look`, with the options `options` switches from
    /// their default for its skin (its parts off, its extras on).
    pub(crate) fn to_look(self, options: &str) -> Look {
        let saber = self.saber_skin.unwrap_or_default();
        Look {
            saber: saber.to_owned(),
            illuminate: self.illuminate,
            saber_off: if saber.is_empty() {
                Vec::new()
            } else {
                crate::saber_skin_options::off_list(options, saber)
            },
            saber_on: if saber.is_empty() {
                Vec::new()
            } else {
                crate::saber_skin_options::on_list(options, saber)
            },
            toys: TOYS
                .iter()
                .filter(|toy| self.has_toy(toy))
                .map(|toy| (*toy).to_owned())
                .collect(),
            // Sent only when the second differs from the first: the first's skin chosen for
            // the second draws the same as no `saber2`.
            saber2: match self.second {
                SecondSkin::Own(skin) if skin != self.saber_skin => {
                    Some(skin.unwrap_or_default().to_owned())
                }
                _ => None,
            },
            saber2_off: match self.second {
                SecondSkin::Own(Some(skin)) if Some(skin) != self.saber_skin => {
                    crate::saber_skin_options::off_list(options, skin)
                }
                _ => Vec::new(),
            },
            saber2_on: match self.second {
                SecondSkin::Own(Some(skin)) if Some(skin) != self.saber_skin => {
                    crate::saber_skin_options::on_list(options, skin)
                }
                _ => Vec::new(),
            },
            saber_sounds: self.sounds.wire().to_owned(),
            // The tag goes on by the frame (`looks_frame.rs`): it is not part of [`Worn`].
            tag: None,
            finisher: self.finisher.unwrap_or_default().to_owned(),
        }
    }
}

/// A look the hub relayed for a slot, with the name its claim was made under.
#[derive(Clone, Debug, PartialEq)]
struct Claimed {
    claimed_name: String,
    worn: Worn,
    /// The tag they stuck on the world (`tags.rs`), if any.
    tag: Option<sjk_identity::PlacedTag>,
}

/// Every slot's look on the current game server.
pub(crate) struct Looks {
    /// What the hub said each slot wears.
    claimed: [Option<Claimed>; SLOTS],
    /// What each slot is drawn wearing: the claimed looks whose names match, and the
    /// local player's own.
    worn: [Worn; SLOTS],
    /// The local player's slot on the server, if in a game.
    own_slot: Option<u8>,
    /// The slots whose hub look's claimed name the game shows there (bits): the SJK
    /// players on the server, as far as the hub has said.
    matched: u64,
    /// The local player's own look, gated.
    own: Worn,
    /// The roster's revision last taken, so it is read again only when it changes.
    pub(crate) roster_revision: Option<u64>,
    /// The server the hub looks are for.
    pub(crate) server: Option<std::net::SocketAddr>,
    /// The feed's reading the hub looks are from (`ReceivedLooks::generation`).
    feed_generation: Option<u64>,
    /// Counts changes to [`Self::worn`] and [`Self::muted`], so a reader can follow
    /// it cheaply.
    revision: u64,
    /// Client slots muted on this PC (bits, `muted_players.rs`): their looks are not
    /// drawn.
    muted: u32,
    /// Other players' toys not shown on this PC ([`crate::toy_sight`]): bits of
    /// [`Unseen`].
    unseen: u8,
    /// `cg_saberSkinOptions` as last read (twice a second), for the skins the
    /// Collection shows on the model.
    own_options: String,
    /// `cg_saberSkin` and `cg_saberSkin2` as last seen, so a change (Equip in the
    /// Collection) is taken at once rather than at the next half second.
    own_settings: (String, String),
    /// World shots own every unlock, having no hub.
    #[cfg(test)]
    pub(crate) shot_owns_unlocks: bool,
    /// World shots of toon models: every player chose toon shading (`toon_choice.rs`).
    #[cfg(test)]
    pub(crate) shot_toon: bool,
}

impl Default for Looks {
    fn default() -> Self {
        Self {
            claimed: std::array::from_fn(|_| None),
            worn: [Worn::default(); SLOTS],
            own_slot: None,
            matched: 0,
            own: Worn::default(),
            roster_revision: None,
            server: None,
            feed_generation: None,
            revision: 0,
            muted: 0,
            unseen: 0,
            own_options: String::new(),
            own_settings: (String::new(), String::new()),
            #[cfg(test)]
            shot_owns_unlocks: false,
            #[cfg(test)]
            shot_toon: false,
        }
    }
}

impl Looks {
    /// The blade skin the player in `client` wears, a catalogue id; `None` for the
    /// stock blade (and for a slot out of range or muted on this PC).
    pub(crate) fn saber_skin_id(&self, client: usize) -> Option<&'static str> {
        self.shown(client)?.saber_skin
    }

    /// What the second saber of the player in `client` wears.
    pub(crate) fn second_skin(&self, client: usize) -> SecondSkin {
        self.shown(client)
            .map_or(SecondSkin::Same, |worn| worn.second)
    }

    /// The options the player in `client` switched off on their second saber's skin.
    pub(crate) fn options_off2(&self, client: usize) -> crate::saber_skin_options::OptionsOff {
        self.shown(client).map_or(0, |worn| worn.options_off2)
    }

    /// The blade skin whose sounds play for the player in `client` (`None` the stock
    /// sounds, and for a slot out of range or muted on this PC): their blade's, unless
    /// they chose others ([`Worn::sound_skin`]).
    pub(crate) fn sound_skin_id(&self, client: usize) -> Option<&'static str> {
        self.shown(client)?.sound_skin()
    }

    /// The options the player in `client` switched off on their blade skin.
    pub(crate) fn options_off(&self, client: usize) -> crate::saber_skin_options::OptionsOff {
        self.shown(client).map_or(0, |worn| worn.options_off)
    }

    /// The finisher the player in `client` wears (`None` for none, and for a slot out of
    /// range or muted on this PC).
    pub(crate) fn finisher(&self, client: usize) -> Option<&'static crate::finishers::Finisher> {
        crate::finishers::get(self.shown(client)?.finisher?)
    }

    /// Whether the player in `client` has their Illuminate holocron lit (and is not
    /// muted on this PC).
    pub(crate) fn illuminated(&self, client: usize) -> bool {
        !self.unseen_by(client, Unseen::ILLUMINATE)
            && self.shown(client).is_some_and(|worn| worn.illuminate)
    }

    /// Whether the player in `client` has toy `id` out (and is not muted on this PC, nor
    /// is the toy hidden here, [`Self::set_unseen`]).
    pub(crate) fn toy(&self, client: usize, id: &str) -> bool {
        !self.unseen_by(client, Unseen::of(id))
            && self.shown(client).is_some_and(|worn| worn.has_toy(id))
    }

    /// Other players' toys `unseen` (bits of [`Unseen`]) are not shown on this PC, never
    /// the local player's own (called every frame, [`crate::toy_sight`]).
    pub(crate) fn set_unseen(&mut self, unseen: u8) {
        self.unseen = unseen;
    }

    /// Whether `bit` of [`Unseen`] hides it on the player in `client`, another player.
    fn unseen_by(&self, client: usize, bit: u8) -> bool {
        self.unseen & bit != 0 && self.own_slot.map(usize::from) != Some(client)
    }

    /// The tag another player in `client` stuck on the world, as the hub relayed it:
    /// only while the game shows their claimed name there and they are not muted on this
    /// PC; never the local player's own (`tags.rs` keeps it).
    pub(crate) fn tag(&self, client: usize) -> Option<&sjk_identity::PlacedTag> {
        let matched = client < SLOTS && self.matched & (1 << client) != 0;
        if !matched || self.own_slot.map(usize::from) == Some(client) {
            return None;
        }
        self.shown(client)?;
        self.claimed.get(client)?.as_ref()?.tag.as_ref()
    }

    /// [`Self::tag`], told apart from a look not known yet (`None`): `Some(None)` when
    /// the player's look is known and has no tag.
    pub(crate) fn tag_known(&self, client: usize) -> Option<Option<&sjk_identity::PlacedTag>> {
        let matched = client < SLOTS && self.matched & (1 << client) != 0;
        if !matched || self.own_slot.map(usize::from) == Some(client) {
            return None;
        }
        self.shown(client)?;
        Some(self.claimed.get(client)?.as_ref()?.tag.as_ref())
    }

    /// What the player in `client` is drawn wearing: nothing while they are muted on
    /// this PC, which never mutes the local player's own slot.
    fn shown(&self, client: usize) -> Option<&Worn> {
        let muted = client < 32
            && self.muted & (1 << client) != 0
            && self.own_slot.map(usize::from) != Some(client);
        self.worn.get(client).filter(|_| !muted)
    }

    /// The client slots muted on this PC changed (or not: this is called every frame):
    /// their looks are no longer drawn, or are again.
    pub(crate) fn set_muted(&mut self, slots: u32) {
        if self.muted != slots {
            self.muted = slots;
            self.revision += 1;
        }
    }

    /// The local player's own blade skin, gated by its profile's unlocks: for the
    /// first-person blade and the Character page's preview, in a game or not.
    pub(crate) fn own_saber_skin(&self) -> Option<&'static str> {
        self.own.saber_skin
    }

    /// The local player's own look.
    pub(crate) fn own(&self) -> Worn {
        self.own
    }

    /// Keep `cg_saberSkinOptions` as read; a change counts as one to what is worn, so
    /// the skins are taken again.
    pub(crate) fn set_own_options(&mut self, options: String) {
        if options != self.own_options {
            self.own_options = options;
            self.revision += 1;
        }
    }

    /// The options the local player switched off on blade skin `skin`, worn or only
    /// shown (the Collection's preview on the model).
    pub(crate) fn own_options_off(&self, skin: &str) -> crate::saber_skin_options::OptionsOff {
        crate::saber_skin_options::off_bits(&self.own_options, skin)
    }

    /// Whether `cg_saberSkin` or `cg_saberSkin2` now say other than when last asked; no
    /// allocation unless they changed.
    pub(crate) fn own_settings_changed(&mut self, first: &str, second: &str) -> bool {
        if self.own_settings.0 == first && self.own_settings.1 == second {
            return false;
        }
        self.own_settings = (first.to_owned(), second.to_owned());
        true
    }

    /// Changes whenever what a slot is drawn wearing (or the own look) changes.
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// Forget every hub look: another server, or none.
    pub(crate) fn clear(&mut self, server: Option<std::net::SocketAddr>) {
        self.claimed = std::array::from_fn(|_| None);
        self.roster_revision = None;
        self.server = server;
        self.rebuild(|_| None);
    }

    /// Follow the feed's reading: when its `generation` changes (the identity went off
    /// or on, another hub or server), every hub look goes and the roster is read
    /// again, so no look from before, a late poll's included, stays. True when the
    /// looks were cleared.
    pub(crate) fn follow_feed(&mut self, generation: u64) -> bool {
        if self.feed_generation == Some(generation) {
            return false;
        }
        self.feed_generation = Some(generation);
        self.clear(self.server);
        true
    }

    /// Take the presence roster's looks in place of every hub look: `(slot, claimed
    /// name, look)` for each live claim, a claim without a look wearing none.
    pub(crate) fn replace_roster<'a>(
        &mut self,
        roster: impl IntoIterator<Item = (u8, &'a str, Option<&'a Look>)>,
    ) {
        self.claimed = std::array::from_fn(|_| None);
        for (slot, claimed_name, look) in roster {
            if let Some(look) = look {
                self.set(slot, claimed_name, look);
            }
        }
    }

    /// Take a look event from the feed: newer than the roster, so it wins until the
    /// roster changes again.
    pub(crate) fn apply_event(&mut self, slot: u8, claimed_name: &str, look: &Look) {
        self.set(slot, claimed_name, look);
    }

    fn set(&mut self, slot: u8, claimed_name: &str, look: &Look) {
        if let Some(entry) = self.claimed.get_mut(usize::from(slot)) {
            *entry = Some(Claimed {
                claimed_name: claimed_name.to_owned(),
                worn: Worn::of(look),
                tag: look.tag.clone(),
            });
        }
    }

    /// The SJK players on the server (bits by slot): those whose hub look's claimed name
    /// the game shows, the local player's own slot included.
    pub(crate) fn sjk_slots(&self) -> u64 {
        self.matched | self.own_slot.map_or(0, |slot| 1 << slot)
    }

    /// The local player's slot on the server, if in a game.
    pub(crate) fn own_slot(&self) -> Option<u8> {
        self.own_slot
    }

    /// The local player's slot (`None` out of a game) and own look. A slot it left
    /// wears nothing until the next [`Looks::rebuild`].
    pub(crate) fn set_own(&mut self, slot: Option<u8>, own: Worn) {
        if (slot, own) != (self.own_slot, self.own) {
            self.revision += 1;
        }
        if slot != self.own_slot
            && let Some(left) = self
                .own_slot
                .and_then(|left| self.worn.get_mut(usize::from(left)))
        {
            *left = Worn::default();
        }
        self.own_slot = slot;
        self.own = own;
        if let Some(entry) = slot.and_then(|slot| self.worn.get_mut(usize::from(slot))) {
            *entry = own;
        }
    }

    /// Rebuild what each slot is drawn wearing; `shown` is the name the game shows in
    /// a slot. Only slots with a hub look ask for it.
    pub(crate) fn rebuild(&mut self, shown: impl Fn(u8) -> Option<String>) {
        let before = self.worn;
        self.matched = 0;
        for (slot, (worn, claimed)) in self.worn.iter_mut().zip(&self.claimed).enumerate() {
            let matched = claimed.as_ref().filter(|claimed| {
                u8::try_from(slot)
                    .ok()
                    .and_then(&shown)
                    .is_some_and(|shown| sjk_identity::names_match(&claimed.claimed_name, &shown))
            });
            if matched.is_some() {
                self.matched |= 1 << slot;
            }
            *worn = matched.map_or_else(Worn::default, |claimed| claimed.worn);
        }
        let (slot, own) = (self.own_slot, self.own);
        self.set_own(slot, own);
        if self.worn != before {
            self.revision += 1;
        }
    }
}

/// The toys [`Looks::set_unseen`] hides on other players, as bits.
pub(crate) struct Unseen;

impl Unseen {
    pub(crate) const ILLUMINATE: u8 = 1;
    pub(crate) const CARDBOARD_BOX: u8 = 1 << 1;
    pub(crate) const BOOMBOX: u8 = 1 << 2;

    /// The bit of toy `id`, none for a toy that is not hidden so.
    fn of(id: &str) -> u8 {
        match id {
            crate::cardboard_box::TOY => Self::CARDBOARD_BOX,
            crate::boombox::TOY => Self::BOOMBOX,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(saber: &str, illuminate: bool) -> Look {
        Look {
            saber: saber.to_owned(),
            illuminate,
            saber_off: Vec::new(),
            toys: Vec::new(),
            saber2: None,
            saber2_off: Vec::new(),
            saber_on: Vec::new(),
            saber2_on: Vec::new(),
            saber_sounds: String::new(),
            tag: None,
            finisher: String::new(),
        }
    }

    #[test]
    fn toys_go_both_ways_and_unknown_ones_draw_nothing() {
        let boxed = Look {
            toys: vec!["a_hat_from_later".to_owned(), "cardboard_box".to_owned()],
            ..Look::default()
        };
        let worn = Worn::of(&boxed);
        assert!(worn.has_toy("cardboard_box") && !worn.has_toy("a_hat_from_later"));
        assert_eq!(worn.to_look("").toys, ["cardboard_box"]);
        let off = worn.with_toy("cardboard_box", false);
        assert!(off.to_look("").toys.is_empty());
        let mut looks = Looks::default();
        looks.replace_roster([(3, "^2Sol", Some(&boxed))]);
        looks.rebuild(shown);
        assert!(looks.toy(3, "cardboard_box") && !looks.toy(5, "cardboard_box"));
        looks.set_muted(1 << 3);
        assert!(!looks.toy(3, "cardboard_box"), "muted here: nothing drawn");
    }

    /// Every toy this client knows goes out in the look and comes back from it, all of
    /// them at once too: ten since the Banana peel and Fishing, more than the 8 hubs took
    /// before 30/10/2026.
    #[test]
    fn every_toy_goes_both_ways_all_at_once() {
        assert!(TOYS.contains(&crate::banana::TOY) && TOYS.contains(&crate::fishing::TOY));
        let all = TOYS
            .iter()
            .fold(Worn::default(), |worn, toy| worn.with_toy(toy, true));
        let look = all.to_look("");
        assert_eq!(look.toys, TOYS);
        let back = Worn::of(&look);
        assert!(TOYS.iter().all(|toy| back.has_toy(toy)));
        let peel_away = back.with_toy(crate::banana::TOY, false);
        assert!(!peel_away.has_toy(crate::banana::TOY) && peel_away.has_toy(crate::box_hunt::TOY));
        let rest: Vec<&str> = TOYS
            .iter()
            .copied()
            .filter(|toy| *toy != crate::banana::TOY)
            .collect();
        assert_eq!(peel_away.to_look("").toys, rest);
    }

    /// Other players' boxes, boomboxes and holocrons hidden on this PC are not drawn on
    /// them; the player's own still are, and other toys are not touched.
    #[test]
    fn toys_hidden_here_are_not_drawn_on_others() {
        let worn = Look {
            toys: vec![
                "cardboard_box".to_owned(),
                "boombox".to_owned(),
                "hax".to_owned(),
            ],
            illuminate: true,
            ..Look::default()
        };
        let mut looks = Looks::default();
        looks.replace_roster([(3, "^2Sol", Some(&worn))]);
        looks.rebuild(shown);
        looks.set_unseen(Unseen::CARDBOARD_BOX | Unseen::BOOMBOX | Unseen::ILLUMINATE);
        assert!(!looks.toy(3, "cardboard_box") && !looks.toy(3, "boombox"));
        assert!(!looks.illuminated(3));
        assert!(looks.toy(3, "hax"), "only the toys hidden");
        // The player's own slot keeps them.
        looks.set_own(Some(3), Worn::of(&worn));
        assert!(looks.toy(3, "cardboard_box") && looks.illuminated(3));
        looks.set_own(None, Worn::default());
        looks.rebuild(shown);
        looks.set_unseen(0);
        assert!(looks.toy(3, "boombox") && looks.illuminated(3));
    }

    /// The game shows `^2Sol` in slot 3 and `Fox` in slot 5.
    fn shown(slot: u8) -> Option<String> {
        match slot {
            3 => Some("^2Sol".to_owned()),
            5 => Some("Fox".to_owned()),
            _ => None,
        }
    }

    #[test]
    fn a_hub_look_counts_only_under_the_name_the_game_shows() {
        let mut looks = Looks::default();
        let sun = look("saber_sun", true);
        looks.replace_roster([
            (3, "sol", Some(&sun)),
            (5, "Wolf", Some(&sun)),
            (7, "Kit", Some(&sun)),
            (9, "Ren", None),
        ]);
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), Some("saber_sun"));
        assert!(looks.illuminated(3));
        assert_eq!(looks.saber_skin_id(5), None, "another name in the slot");
        assert!(!looks.illuminated(5));
        assert!(!looks.illuminated(7), "nobody in the slot");
        assert!(!looks.illuminated(9));
        assert!(!looks.illuminated(200), "no such slot");
        // Someone else takes slot 3: the look goes with the next rebuild.
        looks.rebuild(|slot| (slot == 3).then(|| "Fox".to_owned()));
        assert!(!looks.illuminated(3));
    }

    #[test]
    fn following_someone_the_view_is_theirs_and_the_own_slot_stays() {
        // `PMF_FOLLOW`: the server sends the followed player's state.
        const PMF_FOLLOW: u16 = 0x1000;
        let game_state = GameState::empty_local(2);
        let mut player = PlayerState::zero();
        player.set_client_num(2);
        let playing = ViewSlots::of(&game_state, &player);
        assert_eq!((playing.own, playing.viewed), (Some(2), 2));
        assert_eq!(playing.followed(), None);
        player.set_client_num(5);
        player.set_movement_flags(PMF_FOLLOW);
        let following = ViewSlots::of(&game_state, &player);
        assert_eq!(following.own, Some(2), "the game state's, not the state's");
        assert_eq!(following.followed(), Some(5));
    }

    #[test]
    fn an_unknown_blade_skin_is_the_stock_blade() {
        let mut looks = Looks::default();
        looks.apply_event(3, "Sol", &look("saber_from_the_future", true));
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), None);
        assert!(looks.illuminated(3), "the rest of the look still counts");
    }

    #[test]
    fn feed_events_win_until_the_roster_changes() {
        let mut looks = Looks::default();
        looks.replace_roster([(3, "Sol", Some(&look("", false)))]);
        looks.rebuild(shown);
        assert!(!looks.illuminated(3));
        // The feed's event is newer than the roster.
        looks.apply_event(3, "Sol", &look("saber_sun", true));
        looks.apply_event(5, "Fox", &look("", true));
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), Some("saber_sun"));
        assert!(looks.illuminated(5));
        // A changed roster replaces every hub look: slot 5's claim has none now.
        looks.replace_roster([(3, "Sol", Some(&look("", true)))]);
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), None);
        assert!(looks.illuminated(3));
        assert!(!looks.illuminated(5));
    }

    #[test]
    fn a_second_saber_wears_its_own_skin_both_ways() {
        let owned = |id: &str| id == "saber_sun" || id == "saber_glitch";
        let own = |second: &str| {
            Worn::own("saber_sun", "saber_glitch.haze", owned, false).with_second(
                second,
                "saber_glitch.haze",
                owned,
            )
        };
        // Its own skin, with its options; the stock blade; the first's.
        let glitch = own("saber_glitch");
        assert_eq!(glitch.second, SecondSkin::Own(Some("saber_glitch")));
        let sent = glitch.to_look("saber_glitch.haze");
        assert_eq!(sent.saber2.as_deref(), Some("saber_glitch"));
        assert_eq!(sent.saber2_off, ["haze"]);
        assert_eq!(own(STOCK_SECOND).to_look("").saber2.as_deref(), Some(""));
        // The first's skin chosen for the second stays its own, but is sent as no `saber2`;
        // an empty setting sends none either.
        assert_eq!(own("saber_sun").second, SecondSkin::Own(Some("saber_sun")));
        assert_eq!(own("saber_sun").to_look("").saber2, None);
        assert_eq!(own("").second, SecondSkin::Same);
        assert_eq!(own("").to_look("").saber2, None);
        // A skin not held (taken back) or unknown is the stock blade, here as at the hub.
        for stock in ["saber_tidal", "saber_starfall"] {
            assert_eq!(own(stock).second, SecondSkin::Own(None), "{stock}");
            assert_eq!(
                own(stock).to_look("").saber2.as_deref(),
                Some(""),
                "{stock}"
            );
        }
        // What another player's look says.
        let mut looks = Looks::default();
        let dual = Look {
            saber2: Some("saber_glitch".to_owned()),
            saber2_off: vec!["haze".to_owned()],
            saber_on: Vec::new(),
            saber2_on: Vec::new(),
            ..look("saber_sun", false)
        };
        let stock_second = Look {
            saber2: Some(String::new()),
            ..look("saber_sun", false)
        };
        looks.replace_roster([(3, "^2Sol", Some(&dual)), (5, "Fox", Some(&stock_second))]);
        looks.rebuild(shown);
        assert_eq!(looks.second_skin(3), SecondSkin::Own(Some("saber_glitch")));
        assert_ne!(looks.options_off2(3), 0);
        assert_eq!(looks.second_skin(5), SecondSkin::Own(None));
        looks.apply_event(5, "Fox", &look("saber_sun", false));
        looks.rebuild(shown);
        assert_eq!(looks.second_skin(5), SecondSkin::Same);
    }

    #[test]
    fn another_server_clears_the_hub_looks_but_not_the_own_one() {
        let mut looks = Looks::default();
        looks.apply_event(3, "Sol", &look("saber_sun", true));
        looks.roster_revision = Some(4);
        let own = Worn {
            saber_skin: Some("saber_sun"),
            illuminate: true,
            ..Worn::default()
        };
        looks.set_own(Some(1), own);
        looks.rebuild(shown);
        assert!(looks.illuminated(3));
        let elsewhere = "5.6.7.8:29070".parse().ok();
        looks.clear(elsewhere);
        assert!(!looks.illuminated(3));
        assert_eq!(looks.roster_revision, None);
        assert_eq!(looks.server, elsewhere);
        assert_eq!(looks.own(), own);
        assert!(looks.illuminated(1), "the local player's own slot");
    }

    #[test]
    fn a_new_feed_reading_clears_the_hub_looks() {
        let mut looks = Looks::default();
        assert!(looks.follow_feed(1), "the first reading");
        assert!(!looks.follow_feed(1));
        looks.roster_revision = Some(4);
        looks.apply_event(3, "Sol", &look("saber_sun", true));
        looks.rebuild(shown);
        assert!(looks.illuminated(3));
        // The identity went off: a look applied before must not stay.
        assert!(looks.follow_feed(2));
        looks.rebuild(shown);
        assert!(!looks.illuminated(3));
        assert_eq!(looks.saber_skin_id(3), None);
        assert_eq!(looks.roster_revision, None, "the roster is read again");
    }

    #[test]
    fn the_own_look_comes_from_the_settings_not_the_hub() {
        let mut looks = Looks::default();
        // The hub's roster lists the local player's own claim too.
        looks.replace_roster([(3, "Sol", Some(&look("saber_sun", true)))]);
        looks.set_own(Some(3), Worn::default());
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), None);
        assert!(!looks.illuminated(3));
        // Out of a game the own look still holds, for the Character page.
        let own = Worn::own("saber_sun", "", |_| true, false);
        looks.set_own(None, own);
        assert_eq!(looks.own_saber_skin(), Some("saber_sun"));
    }

    #[test]
    fn a_muted_players_look_is_not_drawn() {
        let mut looks = Looks::default();
        looks.replace_roster([
            (3, "Sol", Some(&look("saber_sun", true))),
            (5, "Fox", Some(&look("saber_sun", true))),
        ]);
        looks.set_own(Some(1), Worn::own("saber_sun", "", |_| true, true));
        looks.rebuild(shown);
        let before = looks.revision();
        // Slot 3 muted on this PC: the stock blade (so no skin sounds) and no holocron.
        looks.set_muted(1 << 3 | 1 << 1);
        assert!(looks.revision() > before, "the blades follow");
        assert_eq!(looks.saber_skin_id(3), None);
        assert!(!looks.illuminated(3));
        assert_eq!(looks.saber_skin_id(5), Some("saber_sun"), "nobody else");
        assert!(looks.illuminated(5));
        // The local player's own slot is never muted.
        assert_eq!(looks.saber_skin_id(1), Some("saber_sun"));
        assert!(looks.illuminated(1));
        let muted = looks.revision();
        looks.set_muted(1 << 3 | 1 << 1);
        assert_eq!(looks.revision(), muted, "no change, no new revision");
        looks.set_muted(0);
        assert_eq!(looks.saber_skin_id(3), Some("saber_sun"));
        assert!(looks.illuminated(3));
    }

    #[test]
    fn the_revision_follows_what_is_worn() {
        let mut looks = Looks::default();
        let start = looks.revision();
        looks.rebuild(shown);
        assert_eq!(looks.revision(), start, "nothing worn changed");
        looks.apply_event(3, "Sol", &look("saber_sun", false));
        assert_eq!(looks.revision(), start, "not drawn before the rebuild");
        looks.rebuild(shown);
        let worn = looks.revision();
        assert!(worn > start);
        looks.rebuild(shown);
        assert_eq!(looks.revision(), worn);
        looks.set_own(Some(1), Worn::own("saber_sun", "", |_| true, false));
        assert!(looks.revision() > worn);
        let own = looks.revision();
        looks.set_own(Some(1), Worn::own("saber_sun", "", |_| true, false));
        assert_eq!(looks.revision(), own);
    }

    /// Other players' saber sounds from their looks: the blade's when the look has no
    /// choice (an older client), stock or another shader's when it has, none while muted
    /// here; the own choice goes in the look sent and comes back the same.
    #[test]
    fn saber_sounds_come_with_the_look_and_an_older_one_follows_the_blade() {
        let shown = |_| Some("Sol".to_owned());
        let mut looks = Looks::default();
        let with_sounds = |sounds: &str| Look {
            saber_sounds: sounds.to_owned(),
            ..look("saber_sun", false)
        };
        looks.apply_event(2, "Sol", &look("saber_sun", false));
        looks.apply_event(3, "Sol", &with_sounds("stock"));
        looks.apply_event(4, "Sol", &with_sounds("saber_void"));
        looks.apply_event(
            5,
            "Sol",
            &Look {
                saber_sounds: "saber_void".to_owned(),
                ..look("", false)
            },
        );
        looks.rebuild(shown);
        assert_eq!(looks.sound_skin_id(2), Some("saber_sun"), "an older look");
        assert_eq!(looks.sound_skin_id(3), None, "stock");
        assert_eq!(looks.sound_skin_id(4), Some("saber_void"));
        assert_eq!(looks.saber_skin_id(4), Some("saber_sun"), "the blade stays");
        assert_eq!(
            looks.sound_skin_id(5),
            Some("saber_void"),
            "on the stock blade"
        );
        looks.set_muted(1 << 4);
        assert_eq!(looks.sound_skin_id(4), None, "muted here");
        // The own choice: sent in the look, a shader's only while owned.
        let owned = |id: &str| id == "saber_sun" || id == "saber_void";
        let own = Worn::own("saber_sun", "", owned, false).with_sounds("saber_void", owned);
        assert_eq!(own.sound_skin(), Some("saber_void"));
        assert_eq!(own.to_look("").saber_sounds, "saber_void");
        assert_eq!(Worn::of(&own.to_look("")), own);
        let lacking = Worn::own("saber_sun", "", owned, false).with_sounds("saber_storm", owned);
        assert_eq!(
            lacking.sound_skin(),
            Some("saber_sun"),
            "not owned: follows"
        );
        assert_eq!(lacking.to_look("").saber_sounds, "", "and is not sent");
    }

    #[test]
    fn the_own_blade_skin_needs_the_unlock_and_a_known_id() {
        let owned = |id: &str| id == "saber_sun";
        assert_eq!(
            Worn::own(" saber_sun ", "", owned, true),
            Worn {
                saber_skin: Some("saber_sun"),
                illuminate: true,
                ..Worn::default()
            }
        );
        assert_eq!(
            Worn::own("saber_sun", "", |_| false, false).saber_skin,
            None
        );
        assert_eq!(
            Worn::own("saber_starfall", "", |_| true, false).saber_skin,
            None
        );
        assert_eq!(Worn::own("", "", owned, true).to_look(""), look("", true));
        assert_eq!(
            Worn::own("saber_sun", "", owned, false).to_look(""),
            look("saber_sun", false)
        );
    }

    #[test]
    fn the_own_look_carries_the_worn_skins_extras_switched_on() {
        let options = "saber_sun.flare_trail saber_sun.haze";
        let own = Worn::own("saber_sun", options, |_| true, false);
        let sent = own.to_look(options);
        assert_eq!(sent.saber_off, ["haze"]);
        assert_eq!(sent.saber_on, ["flare_trail"]);
        assert_eq!(
            Worn::of(&sent),
            own,
            "read back from the hub, the same choice"
        );
        // An extra named among the parts switched off (which the hub does not check) turns
        // nothing on, nor does a part named among the extras switched on.
        let sneaky = Look {
            saber_off: vec!["flare_trail".to_owned()],
            saber_on: vec!["haze".to_owned()],
            ..sent.clone()
        };
        assert_eq!(Worn::of(&sneaky).options_off, 0);
    }

    #[test]
    fn the_own_look_carries_the_worn_skins_options_switched_off() {
        let options = "saber_sun.haze saber_spectral.wisps saber_sun.glint";
        let own = Worn::own("saber_sun", options, |_| true, false);
        assert_eq!(
            own.options_off,
            crate::saber_skin_options::bits(["glint", "haze"])
        );
        let sent = own.to_look(options);
        assert_eq!(
            sent.saber_off,
            ["glint", "haze"],
            "sorted, the worn skin's only"
        );
        // Read back from the hub, the same choice.
        assert_eq!(Worn::of(&sent), own);
        // The stock blade sends none, and an unowned skin is the stock blade.
        let stock = Worn::own("saber_sun", options, |_| false, false);
        assert_eq!(stock.options_off, 0);
        assert!(stock.to_look(options).saber_off.is_empty());
        // A slot shows its wearer's choice; a muted one none.
        let mut looks = Looks::default();
        looks.apply_event(3, "Sol", &sent);
        looks.rebuild(shown);
        assert_eq!(looks.options_off(3), own.options_off);
        looks.set_muted(1 << 3);
        assert_eq!(looks.options_off(3), 0);
    }

    /// A look's finisher (04/11/2026): a known one is worn and goes back on the wire, an
    /// unknown one is none; the own one only while held.
    #[test]
    fn finishers_ride_the_look() {
        let look = Look {
            finisher: "finisher_disintegrate".into(),
            ..Look::default()
        };
        let worn = Worn::of(&look);
        assert_eq!(worn.finisher, Some("finisher_disintegrate"));
        assert_eq!(worn.to_look("").finisher, "finisher_disintegrate");
        let unknown = Look {
            finisher: "finisher_from_later".into(),
            ..Look::default()
        };
        assert_eq!(Worn::of(&unknown).finisher, None);
        let own = Worn::own("", "", |_| true, false);
        assert_eq!(
            own.with_finisher("finisher_disintegrate", |_| false)
                .finisher,
            None
        );
        let own = own.with_finisher("finisher_disintegrate", |_| true);
        assert_eq!(own.to_look("").finisher, "finisher_disintegrate");
        let mut looks = Looks::default();
        looks.set_own(Some(2), own);
        assert_eq!(
            looks.finisher(2).map(|finisher| finisher.name),
            Some("Disintegrate")
        );
        assert_eq!(looks.finisher(3), None);
    }
}
