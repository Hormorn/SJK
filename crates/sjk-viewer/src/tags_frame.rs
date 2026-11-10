//! The frame glue of tags (`tags.rs`): `toy_tag` traces the crosshair to a wall and
//! sticks the chosen tag there; each frame the tags on show (the own and those the hub
//! relayed in other players' looks) are handed to the decal store, which projects one
//! only when it changed.

use super::*;

/// `CONTENTS_SOLID`: what a tag sticks to (not clips, which nobody sees).
const TAG_MASK: u32 = 1;
/// The sound source of the sprays.
const SPRAY_SOURCE: sjk_audio::SourceId = sjk_audio::SourceId(0x5500_0000);

impl GpuState {
    fn tag_centre(&mut self, text: &str) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text.to_owned(),
            None,
            Instant::now(),
        );
    }

    /// The server and map the session is on, the server `None` out of a game.
    fn tag_place(&self) -> (Option<std::net::SocketAddr>, Option<String>) {
        let Some(session) = self.live_session.as_ref() else {
            return (None, None);
        };
        (
            Some(session.server()),
            crate::ghost_replay::current_map(session.game_state()),
        )
    }

    /// The player's eye, where the tag's reach is counted from.
    fn tag_eye(&self) -> Option<Vec3> {
        let session = self.live_session.as_ref()?;
        let player = &session.latest_snapshot().player;
        let origin = self
            .local_prediction
            .predicted_state()
            .map_or(Vec3::from_array(player.origin()), |state| {
                Vec3::from(state.origin)
            });
        Some(origin + Vec3::Z * player.view_height() as f32)
    }

    /// What the player holds of tag or card `id`: what the profile lists (every finish in
    /// a local game, which has no hub).
    fn tag_held(&self, id: &str) -> crate::tags::Held {
        let local = self
            .live_session
            .as_ref()
            .is_some_and(ClientSession::is_local);
        if local && (crate::tags::get(id).is_some() || crate::tags::is_card_id(id)) {
            crate::tags::Held::LOCAL
        } else {
            player_identity::tag_held(id)
        }
    }

    /// `toy_tag`: stick the chosen tag where the crosshair meets a wall or the floor.
    pub(crate) fn stick_tag(&mut self) {
        let now = Instant::now();
        let (Some(server), Some(map)) = self.tag_place() else {
            self.tag_centre("Join a server to tag");
            return;
        };
        let chosen = self
            .console
            .as_ref()
            .and_then(|console| console.text_cvar(crate::tags::CVAR).ok())
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .unwrap_or(crate::tags::DEFAULT)
            .to_owned();
        if chosen == crate::tags::BLANK {
            self.tag_centre("A blank tag takes a GIF first: make it in Collection > Tags");
            return;
        }
        if !crate::tags::stickable(&chosen) {
            self.tag_centre("No such tag: choose one in the Collection");
            return;
        }
        if crate::tags::gif_id(&chosen).is_some() && !self.gif_tags_on() {
            self.tag_centre("GIF tags are off: turn them on in Collection > Tags");
            return;
        }
        let held = self.tag_held(&chosen);
        if held.copies == 0 {
            self.tag_centre("You do not hold that tag yet");
            return;
        }
        // The finish chosen, when a copy in it is held; Normal otherwise.
        let finish = self
            .console
            .as_ref()
            .and_then(|console| console.text_cvar(crate::tags::FINISH_CVAR).ok())
            .map_or(&crate::cards::FINISHES[0], |wanted| {
                held.finish(wanted.trim())
            });
        if let Err(wait) = self.tags.may_stick(now) {
            self.tag_centre(&format!("Tag again in {} s", wait.as_secs() + 1));
            return;
        }
        let Some(camera) = self.frame_view else {
            return;
        };
        let (start, forward) = self.ping_ray(camera);
        let reach = crate::tags::reach_from(start, self.tag_eye());
        let trace = self.bsp.trace_box_with(
            &mut self.trace_scratch,
            start.to_array(),
            (start + forward * reach).to_array(),
            sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).expect("point bounds are valid"),
            TAG_MASK,
        );
        let normal = trace
            .plane
            .as_ref()
            .map(|plane| Vec3::from_array(plane.normal));
        let Some(normal) = normal.filter(|_| {
            forward != Vec3::ZERO
                && trace.fraction < 1.0
                && !trace.start_solid
                && trace.surface_flags & weather::cover::SURF_SKY == 0
        }) else {
            self.tag_centre("Nothing to tag there: get closer to a wall");
            return;
        };
        let normal = normal.normalize_or(Vec3::Z);
        let spin = crate::tags::spin_for(normal, crate::tags::up_for(normal, forward));
        let tag = sjk_identity::PlacedTag {
            id: chosen,
            map,
            at: Vec3::from_array(trace.end_position).to_array(),
            normal: normal.to_array(),
            spin,
            finish: (finish.index != 0).then(|| finish.id.to_owned()),
        };
        // Glass, water, grates, moving surfaces take no tag (`decal_marks::tag_unfit`):
        // one aimed only at such would stand unseen.
        let fits = crate::tags::decal(&tag).is_some_and(|request| {
            let mut scratch = crate::decal_marks::DecalScratch::for_tags();
            crate::decal_marks::project(
                &self.decal_surfaces,
                &self.bsp,
                &request,
                &mut scratch,
                |_| {},
            ) > 0
        });
        if !fits {
            self.tag_centre("A tag does not hold on that surface");
            return;
        }
        self.tags.sprays.push(Vec3::from_array(tag.at));
        self.tags.stick(server, tag, now);
    }

    /// Whether GIF tags are on on this PC ([`crate::tags::GIF_SHOW_CVAR`], off by default).
    fn gif_tags_on(&self) -> bool {
        self.console
            .as_ref()
            .and_then(|console| console.bool_cvar(crate::tags::GIF_SHOW_CVAR))
            .unwrap_or(false)
    }

    /// `toy_tag clear`: take the own tag off.
    pub(crate) fn clear_tag(&mut self) {
        if self.tags.clear() {
            self.tag_centre("Tag taken off");
        }
    }

    /// The own tag, while it stands where the player is and they still hold it.
    pub(crate) fn own_tag(&mut self) -> Option<sjk_identity::PlacedTag> {
        let (server, map) = self.tag_place();
        let mut tags = std::mem::take(&mut self.tags);
        let own = tags
            .own(server, map.as_deref(), |id| self.tag_held(id))
            .cloned();
        self.tags = tags;
        own
    }

    /// Hand the decal store the tags on show: the own, and each other player's on this
    /// map unless [`crate::tags::SHOW_CVAR`] hides them.
    pub(crate) fn update_tags(&mut self) {
        // Out of a game the tags go once; the store is left alone after (world shots
        // stick their own).
        if self.live_session.is_none() {
            if std::mem::take(&mut self.tags.in_game) {
                for slot in 0..crate::looks::SLOTS {
                    self.effect_aux.decals.set_tag(slot, None);
                }
            }
            return;
        }
        self.tags.in_game = true;
        let own = self.own_tag();
        let (_, map) = self.tag_place();
        let show = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(crate::tags::SHOW_CVAR))
            .unwrap_or(true);
        // GIF tags are opt-in: off, none is drawn, the own included, nor fetched.
        let gifs = self.gif_tags_on();
        crate::chat_gifs::set_walls(gifs);
        let shown = |tag: &&sjk_identity::PlacedTag| gifs || crate::tags::gif_id(&tag.id).is_none();
        let own_slot = self.looks.own_slot().map(usize::from);
        for slot in 0..crate::looks::SLOTS {
            let tag = if Some(slot) == own_slot {
                own.as_ref().filter(shown)
            } else if show && map.is_some() {
                self.looks
                    .tag(slot)
                    .filter(|tag| Some(tag.map.as_str()) == map.as_deref())
                    .filter(shown)
            } else {
                None
            };
            self.effect_aux.decals.set_tag(slot, tag);
        }
        self.hear_other_sprays(show, gifs, map.as_deref(), own_slot);
    }

    /// Queue the spray of each other player who put a new tag up on this map since the
    /// last frame (not one whose look just arrived with it, nor any while tags are
    /// hidden).
    fn hear_other_sprays(
        &mut self,
        show: bool,
        gifs: bool,
        map: Option<&str>,
        own_slot: Option<usize>,
    ) {
        self.tags.seen.resize(crate::looks::SLOTS, None);
        for slot in 0..crate::looks::SLOTS {
            let now = if Some(slot) == own_slot {
                None
            } else {
                self.looks
                    .tag_known(slot)
                    .map(|tag| tag.filter(|tag| Some(tag.map.as_str()) == map).cloned())
            };
            let before = std::mem::replace(&mut self.tags.seen[slot], now.clone());
            if let (Some(before), Some(Some(tag))) = (before, now)
                && show
                && (gifs || crate::tags::gif_id(&tag.id).is_none())
                && before.as_ref() != Some(&tag)
            {
                self.tags.sprays.push(Vec3::from_array(tag.at));
            }
        }
    }

    /// Play the sprays of the frame where the tags went up.
    pub(crate) fn play_tag_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.tags.sprays.clear();
            return;
        };
        if self.tags.sound_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [crate::tags::SPRAY_SOUND]);
            self.tags.sound_registered = Some(audio.generation());
        }
        for at in self.tags.sprays.drain(..) {
            self.tags.sprays_played = self.tags.sprays_played.wrapping_add(1);
            audio.play_toy(
                crate::toy_sounds::Toy::Tag,
                crate::tags::SPRAY_SOUND,
                1.0,
                Some(at.to_array()),
                sjk_audio::SourceId(SPRAY_SOURCE.0 + self.tags.sprays_played % 8),
                sjk_audio::ChannelId(0),
            );
        }
    }
}
