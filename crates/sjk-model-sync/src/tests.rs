use super::*;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// A one-triangle GLM (the layout of sjk-model's `TestGlm`).
fn glm() -> Vec<u8> {
    fn put(data: &mut [u8], at: usize, value: i32) {
        data[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    const HEADER: usize = 164;
    const HIERARCHY: usize = HEADER + 4;
    const LOD: usize = HIERARCHY + 144;
    const SURFACE: usize = LOD + 8;
    const VERTICES: usize = 40;
    let triangles = VERTICES + 3 * 32 + 3 * 8;
    let references = triangles + 12;
    let surface_end = references + 4;
    let end = SURFACE + surface_end;
    let mut data = vec![0; end];
    data[..4].copy_from_slice(b"2LGM");
    put(&mut data, 4, 6);
    data[8..13].copy_from_slice(b"model");
    put(&mut data, 140, 1);
    put(&mut data, 144, 1);
    put(&mut data, 148, LOD as i32);
    put(&mut data, 152, 1);
    put(&mut data, 156, HIERARCHY as i32);
    put(&mut data, 160, end as i32);
    put(&mut data, HEADER, (HIERARCHY - HEADER) as i32);
    data[HIERARCHY..HIERARCHY + 5].copy_from_slice(b"torso");
    put(&mut data, HIERARCHY + 136, -1);
    put(&mut data, LOD, (end - LOD) as i32);
    put(&mut data, LOD + 4, (SURFACE - (LOD + 4)) as i32);
    put(&mut data, SURFACE + 12, 3);
    put(&mut data, SURFACE + 16, VERTICES as i32);
    put(&mut data, SURFACE + 20, 1);
    put(&mut data, SURFACE + 24, triangles as i32);
    put(&mut data, SURFACE + 28, 1);
    put(&mut data, SURFACE + 32, references as i32);
    put(&mut data, SURFACE + 36, surface_end as i32);
    for corner in 0..3 {
        put(&mut data, SURFACE + triangles + corner * 4, corner as i32);
    }
    data
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut data = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    data.extend(width.to_be_bytes());
    data.extend(height.to_be_bytes());
    data.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    data
}

fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0, 0];
    data.extend([0xFF, 0xC0, 0x00, 0x0B, 0x08]);
    data.extend(height.to_be_bytes());
    data.extend(width.to_be_bytes());
    data.extend([0x01, 0x01, 0x11, 0x00, 0xFF, 0xD9]);
    data
}

fn tga(width: u16, height: u16) -> Vec<u8> {
    let mut data = vec![0u8; 18];
    data[2] = 2;
    data[12..14].copy_from_slice(&width.to_le_bytes());
    data[14..16].copy_from_slice(&height.to_le_bytes());
    data[16] = 32;
    data.resize(18 + usize::from(width) * usize::from(height) * 4, 0);
    data
}

fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (path, bytes) in files {
        zip.start_file(*path, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn good() -> Vec<(String, Vec<u8>)> {
    vec![
        ("models/players/solfox/model.glm".into(), glm()),
        (
            "models/players/solfox/model_default.skin".into(),
            b"torso,models/players/solfox/body.tga\n".to_vec(),
        ),
        ("models/players/solfox/body.tga".into(), tga(4, 4)),
        ("models/players/solfox/icon_default.jpg".into(), jpeg(128, 128)),
        ("models/players/solfox/glow.png".into(), png(64, 64)),
        (
            "shaders/solfox.shader".into(),
            b"// glow\nmodels/players/solfox/body\n{\n\t{\n\t\tmap models/players/solfox/glow\n\t}\n}\n"
                .to_vec(),
        ),
    ]
}

fn check_list(files: Vec<(String, Vec<u8>)>) -> Result<Checked, Rejection> {
    check_files(files, &Limits::DEFAULT)
}

fn with(path: &str, bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut files = good();
    files.push((path.into(), bytes.to_vec()));
    files
}

#[test]
fn a_whole_model_passes_through_the_pk3() {
    let files = good();
    let entries: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
        .collect();
    let checked = check(&zip(&entries), &Limits::DEFAULT).expect("a good pack");
    assert_eq!(checked.name, "solfox");
    assert_eq!(checked.files.len(), 6);
    assert_eq!(checked.content_md5.len(), 32);
}

#[test]
fn the_content_md5_ignores_order_and_packing() {
    let mut reversed = good();
    reversed.reverse();
    assert_eq!(
        check_list(good()).unwrap().content_md5,
        check_list(reversed).unwrap().content_md5
    );
    let mut changed = good();
    changed[1].1.push(b'\n');
    assert_ne!(
        check_list(good()).unwrap().content_md5,
        check_list(changed).unwrap().content_md5
    );
}

#[test]
fn files_outside_the_model_are_refused() {
    for path in [
        "models/players/kyle/model.glm",
        "maps/ffa_bespin.bsp",
        "ui/main.menu",
        "shaders/sabers.shader",
        "autoexec.cfg",
        "models/players/solfox.glm",
    ] {
        assert!(check_list(with(path, &glm())).is_err(), "{path}");
    }
    assert!(matches!(
        check_list(with("models/players/kyle/model.glm", &glm())),
        Err(Rejection::TwoModels(..))
    ));
}

#[test]
fn other_file_types_are_refused_inside_the_folder() {
    for path in [
        "models/players/solfox/animation.cfg",
        "models/players/solfox/evil.dll",
        "models/players/solfox/vm.qvm",
        "models/players/solfox/sound.mp3",
        "models/players/solfox/noextension",
    ] {
        assert_eq!(
            check_list(with(path, b"x")),
            Err(Rejection::FileType(path.into())),
            "{path}"
        );
    }
}

#[test]
fn bad_paths_are_refused() {
    for path in [
        "models/players/solfox/../../../autoexec.cfg",
        "/models/players/solfox/a.tga",
        "models\\players\\solfox\\a.tga",
        "models/players/solfox//a.tga",
        "models/players/solfox/./a.tga",
        "models/players/SolFox/a.tga",
        "c:/models/players/solfox/a.tga",
    ] {
        assert_eq!(
            check_list(with(path, &tga(1, 1))),
            Err(Rejection::BadPath(path.into())),
            "{path}"
        );
    }
}

#[test]
fn names_are_checked() {
    assert!(valid_name("solfox_v2-blue"));
    assert!(!valid_name(""));
    assert!(!valid_name("a".repeat(MAX_NAME_LEN + 1).as_str()));
    assert!(!valid_name("sol fox"));
    let files = vec![(
        format!("models/players/{}/model.glm", "a".repeat(40)),
        glm(),
    )];
    assert!(matches!(check_list(files), Err(Rejection::BadName(_))));
}

#[test]
fn a_shader_may_only_define_the_models_own_shaders() {
    let mut files = good();
    files[5].1 = b"models/players/solfox/body { }\ngfx/effects/sabers/blue_glow { }".to_vec();
    assert_eq!(
        check_list(files),
        Err(Rejection::ForeignShader(
            "gfx/effects/sabers/blue_glow".into()
        ))
    );
    // A comment hiding a brace is still read as the game reads it.
    let mut files = good();
    files[5].1 = b"models/players/solfox/body { // }\n}\ntextures/hidden { }".to_vec();
    assert!(matches!(
        check_list(files),
        Err(Rejection::ForeignShader(_))
    ));
    let mut files = good();
    files[5].1 = b"models/players/solfox/body {".to_vec();
    assert!(matches!(check_list(files), Err(Rejection::Broken { .. })));
}

#[test]
fn images_must_be_real_and_small_enough() {
    for (path, bytes) in [
        ("models/players/solfox/a.png", b"not a png".to_vec()),
        ("models/players/solfox/a.jpg", png(4, 4)),
        ("models/players/solfox/a.tga", tga(4, 4)[..40].to_vec()),
        ("models/players/solfox/a.png", png(0, 4)),
    ] {
        assert!(
            matches!(
                check_list(with(path, &bytes)),
                Err(Rejection::Broken { .. })
            ),
            "{path}"
        );
    }
    assert!(matches!(
        check_list(with("models/players/solfox/a.png", &png(8192, 16))),
        Err(Rejection::TooLarge { .. })
    ));
    let mut files = good();
    for index in 0..5 {
        files.push((
            format!("models/players/solfox/big{index}.png"),
            png(4096, 4096),
        ));
    }
    assert!(matches!(
        check_list(files),
        Err(Rejection::TooLarge {
            what: "the textures' memory",
            ..
        })
    ));
}

#[test]
fn a_model_needs_a_valid_glm() {
    let files: Vec<_> = good()
        .into_iter()
        .filter(|(path, _)| !path.ends_with(".glm"))
        .collect();
    assert_eq!(check_list(files), Err(Rejection::NoModel));
    let mut files = good();
    files[0].1.truncate(100);
    assert!(matches!(check_list(files), Err(Rejection::Broken { .. })));
    let limits = Limits {
        glm_vertices: 2,
        ..Limits::DEFAULT
    };
    assert!(matches!(
        check_files(good(), &limits),
        Err(Rejection::TooLarge { .. })
    ));
}

#[test]
fn text_files_must_be_plain_and_short() {
    assert!(matches!(
        check_list(with("models/players/solfox/a.skin", b"torso,\0x")),
        Err(Rejection::Broken { .. })
    ));
    let long = vec![b'a'; 70 * 1024];
    assert!(matches!(
        check_list(with("models/players/solfox/a.skin", &long)),
        Err(Rejection::TooLarge { .. })
    ));
}

#[test]
fn zip_bombs_and_oversized_packs_are_refused() {
    let glm = glm();
    let bomb = vec![0u8; 8 * 1024 * 1024];
    let pk3 = zip(&[
        ("models/players/solfox/model.glm", &glm),
        ("models/players/solfox/bomb.tga", &bomb),
    ]);
    assert!(matches!(
        check(&pk3, &Limits::DEFAULT),
        Err(Rejection::Broken { .. })
    ));
    let small = Limits {
        unpacked_bytes: 1024,
        ..Limits::DEFAULT
    };
    assert!(matches!(
        check(
            &zip(&[("models/players/solfox/model.glm", &tga(32, 32))]),
            &small
        ),
        Err(Rejection::TooLarge { .. })
    ));
    let tiny = Limits {
        pk3_bytes: 10,
        ..Limits::DEFAULT
    };
    assert!(matches!(
        check(&zip(&[("models/players/solfox/model.glm", &glm)]), &tiny),
        Err(Rejection::TooLarge { .. })
    ));
    assert!(matches!(
        check(b"PK not really", &Limits::DEFAULT),
        Err(Rejection::NotAZip(_))
    ));
}

#[test]
fn the_file_count_is_capped() {
    let limits = Limits {
        files: 3,
        ..Limits::DEFAULT
    };
    let files = good();
    let entries: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
        .collect();
    assert!(matches!(
        check(&zip(&entries), &limits),
        Err(Rejection::TooLarge { .. })
    ));
}

#[test]
fn pack_keeps_only_the_model_and_its_shader() {
    let mut files: Vec<(String, Vec<u8>)> = good()
        .into_iter()
        .map(|(path, bytes)| (path.replace("solfox", "SolFox"), bytes))
        .collect();
    files.push(("models/players/kyle/model.glm".into(), glm()));
    files.push(("maps/mp/ffa_bespin.bsp".into(), b"IBSP".to_vec()));
    files.push(("shaders/sabers.shader".into(), b"x { }".to_vec()));
    let (pk3, checked) = pack::pack("SolFox", files, &Limits::DEFAULT).expect("packs");
    assert_eq!(checked.name, "solfox");
    assert_eq!(checked.files.len(), 6);
    assert_eq!(check(&pk3, &Limits::DEFAULT).unwrap(), checked);
    assert_eq!(checked.content_md5, check_list(good()).unwrap().content_md5);
    let (again, _) = pack::pack("solfox", good(), &Limits::DEFAULT).unwrap();
    assert_eq!(pk3, again, "same files, same PK3");
}
