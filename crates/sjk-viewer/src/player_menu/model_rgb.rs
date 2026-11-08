//! The model's colour sliders on the SJK UI's Character page (Creyon, 29/10/2026, as
//! JoF EJK's `ingame_player.menu` R, G and B sliders over `ui_char_color_*`): a model
//! worn with its `rgb` skin takes a colour of the player's choosing, the
//! `char_color_red`, `_green` and `_blue` userinfo cvars (0 to 255) that the server
//! hands everyone as the player's `customRGBA`. The colour shows only on shader stages
//! that read the entity colour (`rgbGen entity`, `oneMinusEntity`,
//! `lightingDiffuseEntity`), so SJK shows the sliders, on the Model category under the
//! grid, for a model with such a shader, whatever its skin is called (Creyon: many RGB
//! models have no `rgb` skin), and for an `rgb` skin, the ones JoF EJK's RGB team colour
//! lists (`UI_HeadCountByColor`). The colour shows on the stage at once and is worn
//! with the model on Apply; the grid's pictures keep their own colours.
use super::*;

/// Whether `model` (`name/skin`) is worn with an RGB skin, which the sliders colour.
pub(super) fn takes_rgb(model: &str) -> bool {
    model
        .rsplit_once('/')
        .is_some_and(|(_, skin)| skin.eq_ignore_ascii_case("rgb"))
}

/// Whether `shader`'s definition has a stage the entity colour colours.
pub(crate) fn takes_entity_colour(shaders: &sjk_shader::ShaderCatalog, shader: &str) -> bool {
    use sjk_shader::RgbGen;
    shaders.get(shader).is_some_and(|definition| {
        definition.stages.iter().any(|stage| {
            matches!(
                stage.resolved_colour.rgb,
                RgbGen::Entity | RgbGen::OneMinusEntity | RgbGen::LightingDiffuseEntity
            )
        })
    })
}

/// A slider's step for the arrow keys, as the saber's colour sliders step.
const STEP: isize = 5;

impl PlayerMenu {
    /// Whether the model on show takes the colour sliders: an `rgb` skin, or a model the
    /// stage found a shader of that the colour shows on.
    pub(super) fn model_takes_rgb(&self) -> bool {
        takes_rgb(&self.draft.model)
            || self
                .stage_tintable
                .as_ref()
                .is_some_and(|(model, tintable)| {
                    *tintable && model.eq_ignore_ascii_case(&self.draft.model)
                })
    }

    /// What the stage found of model `model` ([`takes_entity_colour`]).
    pub(crate) fn set_stage_tintable(&mut self, model: &str, tintable: bool) {
        if self
            .stage_tintable
            .as_ref()
            .is_some_and(|(known, was)| *was == tintable && known == model)
        {
            return;
        }
        self.stage_tintable = Some((model.to_owned(), tintable));
    }

    /// The colour channel (0 red, 1 green, 2 blue) of Character row `index`, if it is
    /// one of the model's sliders.
    pub(super) fn model_channel(&self, index: usize) -> Option<usize> {
        if self.page != ProfilePage::Character {
            return None;
        }
        self.character_rows().get(index)?.channel()
    }

    /// The model's colour on channel `channel`.
    pub(super) fn model_channel_value(&self, channel: usize) -> u8 {
        self.draft.rgb[channel]
    }

    /// Set the model's colour on `channel`: shown on the stage, worn on Apply.
    pub(super) fn set_model_channel(&mut self, channel: usize, value: u8) {
        self.draft.rgb[channel] = value;
    }

    /// Step the model's colour on `channel` by `direction`.
    pub(super) fn step_model_channel(&mut self, channel: usize, direction: isize) {
        let value = (isize::from(self.draft.rgb[channel]) + direction * STEP).clamp(0, 255);
        self.set_model_channel(channel, value as u8);
    }

    /// The colour the stage model wears: the draft's, as `CG_Player` hands the player's
    /// `customRGBA` to the shader stages that read the entity colour, so only a model
    /// made for it changes.
    pub(crate) fn stage_rgb(&self) -> [u8; 3] {
        self.draft.rgb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::sjk::TextTarget;

    #[test]
    fn only_an_rgb_skin_takes_the_sliders() {
        assert!(takes_rgb("jedi_hm/rgb"));
        assert!(takes_rgb("Figo/RGB"));
        assert!(!takes_rgb("kyle/default"));
        assert!(!takes_rgb("jedi_hm/red"));
        assert!(!takes_rgb("rgb"));
    }

    fn draw(menu: &mut PlayerMenu) {
        let font = crate::text::load_modern(1.0, None).unwrap().font;
        let mut vertices = Vec::new();
        menu.append_sjk(
            TextTarget::Inter(&mut vertices, &font),
            [1920.0, 1080.0],
            1.0,
        );
    }

    fn page(model: &str) -> (PlayerMenu, ViewerConsole, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut menu = PlayerMenu::new();
        menu.set_sjk(true);
        menu.return_target = ReturnTarget::InGame;
        menu.set_page(ProfilePage::Character);
        menu.draft.model = model.to_owned();
        draw(&mut menu);
        (menu, console, directory)
    }

    fn slider_rows(menu: &PlayerMenu) -> Vec<usize> {
        menu.character_rows()
            .iter()
            .enumerate()
            .filter(|(_, row)| row.channel().is_some())
            .map(|(index, _)| index)
            .collect()
    }

    #[test]
    fn the_sliders_show_only_for_an_rgb_model_above_apply() {
        let (menu, _console, _directory) = page("kyle/default");
        assert!(slider_rows(&menu).is_empty());
        let (menu, _console, _directory) = page("jedi_hm/rgb");
        let rows = slider_rows(&menu);
        assert_eq!(rows.len(), 3);
        // Then the Toon shading switch, then Apply.
        assert_eq!(
            rows[2] + 2,
            menu.apply_row(),
            "just above the toon switch and Apply"
        );
        assert!(!menu.canvas.overflowed());
        for row in rows {
            assert!(
                menu.canvas.rect_for(row as u16).is_some(),
                "row {row} drawn"
            );
        }
    }

    #[test]
    fn the_sliders_colour_the_stage_and_apply_wears_the_colour() {
        let (mut menu, mut console, _directory) = page("jedi_hm/rgb");
        let rows = slider_rows(&menu);
        // The arrows step a slider.
        menu.selected = rows[0];
        menu.adjust(&mut console, -1);
        assert_eq!(menu.stage_rgb(), [250, 255, 255]);
        // A press on the track sets it from the pointer: the left end is 0.
        let track = menu
            .canvas
            .rect_for(rows[1] as u16)
            .expect("the green track");
        let left = sjk_ui::Vec2::new(track.x + 2.0, track.y + track.height * 0.5);
        let button = sjk_ui::PointerButton::Primary;
        for event in [
            sjk_ui::InputEvent::PointerMove(left),
            sjk_ui::InputEvent::PointerPress {
                position: left,
                button,
            },
            sjk_ui::InputEvent::PointerRelease {
                position: left,
                button,
            },
        ] {
            let _ = menu.handle_pointer(event, &mut console);
        }
        assert_eq!(menu.stage_rgb(), [250, 0, 255]);
        // Typed: 128 on blue.
        assert!(menu.begin_typed(rows[2], "1"));
        for key in [
            winit::keyboard::KeyCode::Digit2,
            winit::keyboard::KeyCode::Digit8,
        ] {
            let text = if key == winit::keyboard::KeyCode::Digit2 {
                "2"
            } else {
                "8"
            };
            menu.edit_numeric(key, Some(text), false, &mut console);
        }
        menu.edit_numeric(winit::keyboard::KeyCode::Enter, None, false, &mut console);
        assert_eq!(menu.stage_rgb(), [250, 0, 128]);
        // Worn only on Apply, as the model is.
        assert!(menu.look_pending());
        menu.apply_look(&mut console);
        assert!(!menu.look_pending());
        assert_eq!(menu.applied_rgb, [250, 0, 128]);
    }

    #[test]
    fn a_shader_that_reads_the_entity_colour_takes_it() {
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        vfs.mount_memory(
            "test",
            [(
                "shaders/test.shader",
                b"models/players/wolf/fur\n{\n{\nmap models/players/wolf/fur\nrgbGen lightingDiffuseEntity\n}\n}\nmodels/players/wolf/eyes\n{\n{\nmap models/players/wolf/eyes\nrgbGen lightingDiffuse\n}\n}\n"
                    .to_vec(),
            )],
        )
        .unwrap();
        let shaders = sjk_shader::ShaderCatalog::load(&vfs).unwrap();
        assert!(takes_entity_colour(&shaders, "models/players/wolf/fur"));
        assert!(!takes_entity_colour(&shaders, "models/players/wolf/eyes"));
        assert!(!takes_entity_colour(&shaders, "models/players/wolf/none"));
    }

    #[test]
    fn a_model_the_stage_finds_colourable_gets_the_sliders_whatever_its_skin() {
        let (mut menu, _console, _directory) = page("wolf/default");
        assert!(!menu.model_takes_rgb());
        menu.set_stage_tintable("wolf/default", true);
        assert!(menu.model_takes_rgb());
        draw(&mut menu);
        assert_eq!(slider_rows(&menu).len(), 3);
        assert!(!menu.canvas.overflowed());
        // What the stage found of another model says nothing of this one.
        menu.draft.model = "kyle/default".to_owned();
        assert!(!menu.model_takes_rgb());
    }
}
