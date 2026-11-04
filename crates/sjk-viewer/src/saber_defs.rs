//! Thin visible-saber adapter over the shared compatibility definition parser.

use sjk_vfs::VirtualFileSystem;
use std::collections::BTreeMap;
use std::error::Error;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Definition {
    /// `soundOn` and `soundOff`, which the Saber Lab's stage plays (`crate::menu_stage`).
    pub(crate) sound_on: Option<String>,
    pub(crate) sound_off: Option<String>,
    pub(crate) sound_spin: Option<String>,
    pub(crate) sound_swing: [Option<String>; 3],
    pub(crate) name: String,
    pub(crate) model: String,
    pub(crate) num_blades: u8,
    pub(crate) is_staff: bool,
    pub(crate) blade_lengths: [f32; 8],
    pub(crate) blade_radii: [f32; 8],
    pub(crate) blade_style2_start: u8,
    pub(crate) trail_style: u8,
    pub(crate) trail_style2: u8,
    /// Authored no-light flag, preserved for hilt light submission.
    pub(crate) no_dlight: bool,
    pub(crate) no_wall_marks: bool,
    /// `saberColor`, which an NPC's blade without a colour override wears.
    pub(crate) default_color: sjk_client::LegacySaberColor,
}

pub(crate) fn load(
    vfs: &VirtualFileSystem,
) -> Result<BTreeMap<String, Definition>, Box<dyn Error>> {
    Ok(sjk_client::legacy_saber_definitions(vfs)?
        .into_iter()
        .map(|(key, definition)| {
            (
                key,
                Definition {
                    name: definition.name,
                    sound_on: definition.sound_on,
                    sound_off: definition.sound_off,
                    sound_spin: definition.sound_spin,
                    sound_swing: definition.sound_swing,
                    model: definition.model,
                    num_blades: definition.num_blades,
                    // EJK CG_StaffHolsteredOnBack (cg_players.c:10812): a staff type
                    // or any single hilt with more than one blade goes on the back.
                    is_staff: definition.saber_type == sjk_client::LegacySaberType::Staff
                        || definition.num_blades > 1,
                    blade_lengths: definition.blade_lengths,
                    blade_radii: definition.blade_radii,
                    blade_style2_start: definition.blade_style2_start,
                    trail_style: definition.trail_style,
                    trail_style2: definition.trail_style2,
                    no_dlight: definition.no_dlight,
                    no_wall_marks: definition.no_wall_marks,
                    default_color: definition.default_color,
                },
            )
        })
        .collect())
}
