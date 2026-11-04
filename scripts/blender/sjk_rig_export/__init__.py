"""SJK Rig Model Export: a rigged character into an SJK rig model, with its emotes.

The 3D view's sidebar gets an SJK tab. Pick an armature, name the model, tick the
actions to bring as emotes and press Export: the armature and the meshes it deforms
are written as glTF (+Y up, rest pose, skins, no shape keys), each material reduced
to its base colour image, with an `sjk_rig.cfg` beside them; each ticked action
becomes an animation of that glTF and an `emotes/<id>.emote` file that plays it,
with the sound file and picture picked for it. A character icon puts the model in
the game's character grid.
Everything goes into one `<GameData>/base/rig_<name>.pk3` (or loose folders), so
removing the model is deleting one file; several models may share one pack (a
pack name of your own), each export replacing only its model's files. In the game,
`/model <name>` wears it and `sjkemote <id>` (or the quick wheel) plays an emote;
the other rig models with the same skeleton (joint names, as Unreal mannequin
characters share) play it too.

Meshes rigged to another armature whose bones follow this one (a Child Of or Copy
Transforms constraint on its bone, as a tail or a cape is set up) go with the
model, that armature's bones joined under the bone they follow; the manifest's
`dynamic` lines make them move on their own in the game (spring and wag).

The blend file is left as it was: the export works on temporary copies of the meshes
and materials, and puts the armature's action, NLA state and frame back afterwards.

See docs/rig-models.md in the SJK repository for the format and its limits.
"""

import json
import os
import re
import shutil
import struct
import tempfile
import zipfile

import bpy
from bpy.props import (
    BoolProperty,
    EnumProperty,
    FloatProperty,
    IntProperty,
    PointerProperty,
    StringProperty,
)
from bpy.types import AddonPreferences, Operator, Panel, PropertyGroup

# The joints each bone map preset drives, as `rig_model.rs` `PRESETS` names them
# (keep the two in step). Only used to tell the user whether the skeleton fits.
PRESET_JOINTS = {
    "mannequin": [
        "pelvis", "spine_01", "spine_02", "spine_03", "spine_04", "spine_05",
        "neck_01", "neck_02", "head",
        *(
            f"{part}_{side}"
            for side in ("l", "r")
            for part in ("upperarm", "lowerarm", "hand", "thigh", "calf", "foot")
        ),
    ],
}

PRESET_ITEMS = [
    ("mannequin", "Unreal mannequin", "pelvis, spine_01, upperarm_l, hand_r, thumb_01_l..."),
]

# Joints the game cannot do without: `scale auto` fits the pelvis, the saber hand.
REQUIRED = ("pelvis", "hand_r")

# Where a Steam or retail install usually keeps GameData, tried when none is set.
GAME_DATA_GUESSES = (
    r"C:\Program Files (x86)\Steam\steamapps\common\Jedi Academy\GameData",
    r"C:\Program Files\Steam\steamapps\common\Jedi Academy\GameData",
    r"C:\Program Files (x86)\LucasArts\Star Wars Jedi Knight Jedi Academy\GameData",
    os.path.expanduser("~/.steam/steam/steamapps/common/Jedi Academy/GameData"),
)

# Image nodes or sockets that hold a material's base colour, as exporters name them.
COLOR_WORDS = ("base color", "basecolor", "diffuse", "albedo")
COLOR_SUFFIX = re.compile(r"(_d|_diffuse|_basecolor|_base_color|_albedo|_bc|_col|_color)$")

MANIFEST = "sjk_rig.cfg"
BONE_PATH = re.compile(r'pose\.bones\["(.+?)"\]')
# The game's limits on an emote (`emotes.rs`): id length, play time in milliseconds.
EMOTE_ID_CHARS = 32
EMOTE_MS = (100, 60_000)
# Sound files the game decodes.
SOUND_EXTENSIONS = (".wav", ".mp3")
# Pictures the game decodes for icons, and the extension each is packed under.
PICTURE_EXTENSIONS = {".png": ".png", ".jpg": ".jpg", ".jpeg": ".jpg", ".tga": ".tga"}
# Bone constraints by which another armature's bone follows one of this armature's.
FOLLOW_CONSTRAINTS = ("CHILD_OF", "COPY_TRANSFORMS")


def preferences(context):
    """The add-on's preferences, or None when it runs without them (a script)."""
    addon = context.preferences.addons.get(__package__)
    return addon.preferences if addon else None


def game_data(context):
    """The GameData folder from the preferences, else the first usual one found."""
    settings = preferences(context)
    chosen = bpy.path.abspath(settings.game_data).strip() if settings else ""
    if chosen:
        return chosen
    return next((path for path in GAME_DATA_GUESSES if os.path.isdir(path)), "")


def clean_name(text):
    """Lower case letters, digits and underscores, as model folders and emote ids are."""
    return re.sub(r"[^a-z0-9_]+", "_", text.lower()).strip("_")


def model_name(settings):
    """The folder name, from the field or the armature's name."""
    return clean_name(
        settings.model_name.strip() or (settings.armature.name if settings.armature else "")
    )


def emote_id(model, action):
    """The emote id and glTF animation name of `action` on `model`: its emote name
    (`sjkemote comrade`), else `<model>_<action>`. Every rig model with the same
    skeleton plays it, so the name alone says which emote it is; `stop` is the
    game's own."""
    named = clean_name(action.sjk_emote_name)[:EMOTE_ID_CHARS].strip("_")
    if named and named != "stop":
        return named
    return clean_name(f"{model}_{action.name}")[:EMOTE_ID_CHARS].strip("_")


def deformed_meshes(context, armature, visible_only):
    """The meshes `armature` deforms (an Armature modifier on it, or parented to it)."""
    meshes = []
    for obj in context.scene.objects:
        if obj.type != "MESH":
            continue
        rigged = any(m.type == "ARMATURE" and m.object == armature for m in obj.modifiers)
        if not rigged and obj.parent != armature:
            continue
        if visible_only and not obj.visible_get():
            continue
        meshes.append(obj)
    return meshes


def followed_bone(armature, other):
    """The bone of `armature` that a bone of `other` follows, or None."""
    for pose_bone in other.pose.bones:
        for constraint in pose_bone.constraints:
            if (
                constraint.type in FOLLOW_CONSTRAINTS
                and not constraint.mute
                and constraint.target == armature
                and constraint.subtarget in armature.data.bones
            ):
                return constraint.subtarget
    return None


def following_meshes(context, armature, visible_only):
    """(mesh, bone, other armature) for the meshes other armatures deform whose
    bones follow `armature`'s bone; they go with the model, their armature's bones
    joined under that bone."""
    found = []
    for other in context.scene.objects:
        if other.type != "ARMATURE" or other == armature:
            continue
        bone = followed_bone(armature, other)
        if bone is None:
            continue
        found.extend(
            (mesh, bone, other) for mesh in deformed_meshes(context, other, visible_only)
        )
    return found


def followers(following):
    """Each following armature once, with the bone it follows."""
    seen = {}
    for _, bone, other in following:
        seen.setdefault(other, bone)
    return list(seen.items())


def dynamic_roots(following):
    """The root bones of the following armatures: the game moves the bones under
    them on their own (the manifest's `dynamic` lines)."""
    return sorted(
        bone.name
        for other, _ in followers(following)
        for bone in other.data.bones
        if bone.parent is None
    )


def picture(path, what):
    """(path, packed extension) of a picture the game reads, or None for no path."""
    path = bpy.path.abspath(path).strip()
    if not path:
        return None
    extension = PICTURE_EXTENSIONS.get(os.path.splitext(path)[1].lower())
    if extension is None:
        raise ValueError(f"{what}: the picture must be .png, .jpg or .tga")
    if not os.path.isfile(path):
        raise ValueError(f"{what}: no picture at {path}")
    return path, extension


def pack_name(settings, name):
    """The pack's file name without .pk3: the field, else rig_<name>."""
    return clean_name(settings.pack_name.strip()) or f"rig_{name}"


def model_files(archive, name):
    """The files of model `name` in an open pack: its folder, and the emote files,
    sounds and pictures of the animations its glTF holds (each is an emote id)."""
    folder = f"models/players/{name}/"
    names = archive.namelist()
    owned = {entry for entry in names if entry.startswith(folder)}
    ids = set()
    if f"{folder}model.gltf" in names:
        try:
            document = json.loads(archive.read(f"{folder}model.gltf"))
            ids = {animation.get("name") for animation in document.get("animations", [])}
        except ValueError:
            pass
    for entry in names:
        directory, _, file = entry.rpartition("/")
        stem = os.path.splitext(file)[0]
        if directory in ("emotes", "sound/emotes", "gfx/emotes") and stem in ids:
            owned.add(entry)
    return owned


def joint_report(armature, preset):
    """(found, expected, missing required) for the preset's joints on `armature`."""
    names = {bone.name for bone in armature.data.bones}
    expected = PRESET_JOINTS.get(preset, [])
    found = sum(1 for joint in expected if joint in names)
    missing = [joint for joint in REQUIRED if joint not in names]
    return found, len(expected), missing


def action_fcurves(action):
    """Every F-curve of `action`, in layered (Blender 4.4+) or legacy actions."""
    layers = getattr(action, "layers", None)
    if layers:
        return [
            curve
            for layer in layers
            for strip in layer.strips
            for bag in getattr(strip, "channelbags", ())
            for curve in bag.fcurves
        ]
    return list(getattr(action, "fcurves", ()))


def animated_bones(action, armature):
    """The bones of `armature` that `action` animates, in the armature's order."""
    named = {
        match.group(1)
        for curve in action_fcurves(action)
        if (match := BONE_PATH.match(curve.data_path))
    }
    return [bone.name for bone in armature.data.bones if bone.name in named]


def emote_actions(armature):
    """The actions that animate `armature`'s bones, by name."""
    return sorted(
        (action for action in bpy.data.actions if animated_bones(action, armature)),
        key=lambda action: action.name.lower(),
    )


def upstream_image(material):
    """The nearest image feeding a Principled BSDF's Base Color, through the nodes
    between (Ambient Occlusion, Mix, Hue/Saturation...), or None."""
    for node in material.node_tree.nodes:
        if node.type != "BSDF_PRINCIPLED":
            continue
        queue = [link.from_node for link in node.inputs["Base Color"].links]
        seen = set()
        while queue:
            found = queue.pop(0)
            if found.name in seen:
                continue
            seen.add(found.name)
            if found.type == "TEX_IMAGE" and found.image is not None:
                return found.image
            queue.extend(
                link.from_node for socket in found.inputs for link in socket.links
            )
    return None


def base_color_image(material):
    """The image a material's base colour most likely comes from, or None."""
    if material is None or material.node_tree is None:
        return None
    for node in material.node_tree.nodes:  # a picked (often baked) image wins
        if node.type == "TEX_IMAGE" and node.image is not None and node.label == "SJK":
            return node.image
    image = upstream_image(material)
    if image is not None:
        return image
    best = None
    for node in material.node_tree.nodes:
        if node.type != "TEX_IMAGE" or node.image is None:
            continue
        sockets = [link.to_socket.name.lower() for link in node.outputs["Color"].links]
        label = (node.label or node.name).lower()
        stem = os.path.splitext(node.image.name.lower())[0]
        if any(socket in COLOR_WORDS for socket in sockets):
            rank = 0
        elif label in COLOR_WORDS:
            rank = 1
        elif COLOR_SUFFIX.search(stem):
            rank = 2
        elif node.image.colorspace_settings.name == "sRGB":
            rank = 3
        else:
            continue
        if best is None or rank < best[0]:
            best = (rank, node.image)
    return best[1] if best else None


def keep_shape_mix(obj):
    """Drop `obj`'s shape keys, keeping the shape they mix to now (a key switched on,
    such as a top fitted over the chest, stays in the mesh)."""
    if not obj.data.shape_keys:
        return
    mix = obj.shape_key_add(name="SJK_mix", from_mix=True)
    co = [0.0] * (len(obj.data.vertices) * 3)
    mix.data.foreach_get("co", co)
    obj.shape_key_clear()
    obj.data.vertices.foreach_set("co", co)
    obj.data.update()


def apply_masks(context, obj):
    """Apply `obj`'s Mask modifiers shown in the viewport, so hidden geometry is not
    exported (the export applies no other modifier)."""
    for modifier in list(obj.modifiers):
        if modifier.type == "MASK" and modifier.show_viewport:
            with context.temp_override(object=obj, active_object=obj, selected_objects=[obj]):
                bpy.ops.object.modifier_apply(modifier=modifier.name)


def plain_material(source):
    """A temporary Principled material with `source`'s base colour image."""
    material = bpy.data.materials.new(f"SJK_export_{source.name if source else 'none'}")
    if material.node_tree is None:  # before Blender 5, materials start without nodes
        material.use_nodes = True
    tree = material.node_tree
    bsdf = next(node for node in tree.nodes if node.type == "BSDF_PRINCIPLED")
    image = base_color_image(source)
    if image is None and source is not None:
        image = color_image(source)
    if image is not None:
        texture = tree.nodes.new("ShaderNodeTexImage")
        texture.image = image
        tree.links.new(texture.outputs["Color"], bsdf.inputs["Base Color"])
    return material


def color_image(material):
    """A temporary 4x4 image of an untextured material's colour (its Principled Base
    Color, else its viewport colour, alpha included), so the game draws that colour
    rather than white. Named `<material>_color`, which a shader script can override."""
    color = list(material.diffuse_color)
    tree = material.node_tree
    bsdf = tree and next((n for n in tree.nodes if n.type == "BSDF_PRINCIPLED"), None)
    if bsdf is not None and not bsdf.inputs["Base Color"].is_linked:
        color = list(bsdf.inputs["Base Color"].default_value)[:3] + [color[3]]
    image = bpy.data.images.new(f"{material.name}_color", 4, 4, alpha=True)
    image.pixels.foreach_set(color * 16)
    image.file_format = "PNG"
    image["sjk_export"] = True
    return image


def with_lift(text, lift):
    """`text` (a manifest) with its `lift` line set to `lift`, or removed at 0."""
    lines = [line for line in text.splitlines() if line.split()[:1] != ["lift"]]
    if lift:
        lines.append(f"lift {lift:g}")
    return "\n".join(lines) + "\n"


# The game's gravity on a dynamic chain when its line gives none (`rig_dynamics.rs`).
DEFAULT_GRAVITY = 1600.0


def dynamic_options(words):
    """The words of a `dynamic` line after its joint and bare gravity (`wag 0`,
    `stiffness 120`, `nocollide`...), written by hand and kept on re-export."""
    rest = words[2:]
    if rest:
        try:
            float(rest[0])
            rest = rest[1:]
        except ValueError:
            pass
    return rest


def with_dynamic(text, joints, gravity=DEFAULT_GRAVITY, own=()):
    """`text` (a manifest) with one `dynamic` line per joint of `joints`, the ones it
    had replaced (keeping their hand-written options, such as `wag 0`); `gravity` is
    written when it is not the game's default. Lines naming a bone of `own` (the
    armature's own bones, such as a tail added by hand) stay as they are."""
    options = {}
    lines = []
    for line in text.splitlines():
        words = line.split()
        if words[:1] != ["dynamic"]:
            lines.append(line)
        elif len(words) > 1 and words[1] in own and words[1] not in joints:
            lines.append(line)
        elif len(words) > 1:
            options[words[1]] = dynamic_options(words)
    suffix = "" if gravity == DEFAULT_GRAVITY else f" {gravity:g}"
    for joint in joints:
        extra = "".join(f" {word}" for word in options.get(joint, []))
        lines.append(f"dynamic {joint}{suffix}{extra}")
    return "\n".join(lines) + "\n"


def manifest_text(preset):
    return "\n".join([
        "// SJK rig model, written by the SJK Rig Model Export add-on.",
        "// Format: docs/rig-models.md in the SJK repository.",
        "model model.gltf",
        f"preset {preset}",
        "scale auto",
        "",
    ])


def frame_rate(context):
    """The actions' frames per second: the setting, else the scene's."""
    settings = context.scene.sjk_rig_export
    scene = context.scene
    return settings.frame_rate or scene.render.fps / scene.render.fps_base


def sample_clips(context, armature, actions, model):
    """Each action's animated bones sampled at every frame, as pose-bone bases.

    Returns a list of (action, id, seconds per frame, bone names, frames), a frame
    being one `matrix_basis` per bone. The armature plays each action alone (its NLA
    off) for the sampling; its action, NLA switch and the scene frame come back.
    """
    scene = context.scene
    data = armature.animation_data_create()
    saved = (data.action, data.use_nla, scene.frame_current)
    saved_slot = getattr(data, "action_slot", None)
    fps = frame_rate(context)
    clips = []
    try:
        data.use_nla = False
        for action in actions:
            data.action = action
            if hasattr(data, "action_slot") and data.action_slot is None:
                slots = getattr(data, "action_suitable_slots", ())
                if slots:
                    data.action_slot = slots[0]
            bones = animated_bones(action, armature)
            start, end = (int(round(frame)) for frame in action.frame_range)
            frames = []
            for frame in range(start, max(end, start + 1) + 1):
                scene.frame_set(frame)
                frames.append([armature.pose.bones[name].matrix_basis.copy() for name in bones])
            clips.append((action, emote_id(model, action), 1.0 / fps, bones, frames))
    finally:
        data.action = saved[0]
        if saved_slot is not None and hasattr(data, "action_slot"):
            data.action_slot = saved_slot
        data.use_nla = saved[1]
        scene.frame_set(saved[2])
    return clips


def append_clips(gltf_path, clips):
    """Write `clips` into the glTF at `gltf_path` as animations of its joint nodes.

    A joint node's transform at a frame is its rest transform (as the exporter wrote
    it) times the pose bone's basis, the way Blender poses a bone under its parent.
    """
    from mathutils import Matrix, Quaternion, Vector

    with open(gltf_path, encoding="utf-8") as file:
        document = json.load(file)
    buffer = document["buffers"][0]
    bin_path = os.path.join(os.path.dirname(gltf_path), buffer["uri"])
    with open(bin_path, "rb") as file:
        binary = bytearray(file.read())
    nodes = document["nodes"]
    by_name = {node.get("name"): index for index, node in enumerate(nodes)}
    views = document.setdefault("bufferViews", [])
    accessors = document.setdefault("accessors", [])
    animations = document.setdefault("animations", [])

    def rest(node):
        if "matrix" in node:
            return Matrix([node["matrix"][column::4] for column in range(4)])
        t = Vector(node.get("translation", (0.0, 0.0, 0.0)))
        x, y, z, w = node.get("rotation", (0.0, 0.0, 0.0, 1.0))
        s = node.get("scale", (1.0, 1.0, 1.0))
        return Matrix.LocRotScale(t, Quaternion((w, x, y, z)), Vector(s))

    def accessor(values, kind, width, bounds=False):
        while len(binary) % 4:
            binary.append(0)
        offset = len(binary)
        for value in values:
            binary.extend(struct.pack(f"<{width}f", *value))
        views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(binary) - offset})
        entry = {
            "bufferView": len(views) - 1,
            "componentType": 5126,
            "count": len(values),
            "type": kind,
        }
        if bounds:
            entry["min"] = [min(v[i] for v in values) for i in range(width)]
            entry["max"] = [max(v[i] for v in values) for i in range(width)]
        accessors.append(entry)
        return len(accessors) - 1

    written = []
    for action, name, step, bones, frames in clips:
        times = accessor([(index * step,) for index in range(len(frames))], "SCALAR", 1, True)
        samplers, channels = [], []
        for slot, bone in enumerate(bones):
            node = by_name.get(bone)
            if node is None:
                continue
            base = rest(nodes[node])
            translations, rotations, scales = [], [], []
            previous = None
            for frame in frames:
                t, q, s = (base @ frame[slot]).decompose()
                if previous is not None and previous.dot(q) < 0.0:
                    q.negate()
                previous = q
                translations.append(tuple(t))
                rotations.append((q.x, q.y, q.z, q.w))
                scales.append(tuple(s))
            for path, values, kind, width in (
                ("translation", translations, "VEC3", 3),
                ("rotation", rotations, "VEC4", 4),
                ("scale", scales, "VEC3", 3),
            ):
                samplers.append({"input": times, "output": accessor(values, kind, width)})
                channels.append({
                    "sampler": len(samplers) - 1,
                    "target": {"node": node, "path": path},
                })
        if channels:
            animations[:] = [a for a in animations if a.get("name") != name]
            animations.append({"name": name, "samplers": samplers, "channels": channels})
            written.append((action, name, (len(frames) - 1) * step))
    buffer["byteLength"] = len(binary)
    with open(bin_path, "wb") as file:
        file.write(binary)
    with open(gltf_path, "w", encoding="utf-8") as file:
        json.dump(document, file)
    return written


def sound_milliseconds(path):
    """How long the sound at `path` lasts, in milliseconds (0 when unknown)."""
    try:
        import aud

        sound = aud.Sound(path)
        rate = sound.specs[0]
        return int(round(sound.length / rate * 1000)) if rate else 0
    except Exception:
        return 0


def emote_text(action, name, seconds, sound, icon, settings, model, music=None):
    milliseconds = min(max(int(round(seconds * 1000)), EMOTE_MS[0]), EMOTE_MS[1])
    title = action.sjk_emote_name.strip() or action.name
    # `rig` names the model the clip is in: other models with the same skeleton
    # (joint names) borrow it.
    entry = {"name": title[:32], "clip": name, "length_ms": milliseconds, "rig": model}
    if action.sjk_emote_loop:
        entry["loop"] = True
    if sound:
        entry["sound"] = sound
    if settings.emotes_move:
        entry["moving"] = True
    if settings.local_emotes:
        entry["local"] = True
    if icon:
        entry["icon"] = icon
    # Lengths and the loop, so the game repeats the music: (sound_ms, loop path,
    # loop_ms).
    if music:
        sound_ms, loop, loop_ms = music
        if sound_ms:
            entry["sound_ms"] = sound_ms
        if loop:
            entry["sound_loop"] = loop
            entry["sound_loop_ms"] = loop_ms
    return json.dumps(entry, indent=1) + "\n"


def emote_sound(action, key="sjk_emote_sound"):
    """The sound file picked for `action` (its `key` property), if it is one the
    game reads."""
    path = bpy.path.abspath(getattr(action, key)).strip()
    if not path:
        return None
    if os.path.splitext(path)[1].lower() not in SOUND_EXTENSIONS:
        raise ValueError(f"{action.name}: the sound must be .wav or .mp3")
    if not os.path.isfile(path):
        raise ValueError(f"{action.name}: no sound file at {path}")
    return path


class SJKRigExportPreferences(AddonPreferences):
    bl_idname = __package__

    game_data: StringProperty(
        name="GameData folder",
        description="Jedi Academy's GameData folder (empty: found automatically)",
        subtype="DIR_PATH",
        default="",
    )

    def draw(self, context):
        self.layout.prop(self, "game_data")
        self.layout.label(text=f"In use: {game_data(context) or 'not found'}")


class SJKRigExportSettings(PropertyGroup):
    armature: PointerProperty(
        name="Armature",
        type=bpy.types.Object,
        poll=lambda _self, obj: obj.type == "ARMATURE",
    )
    model_name: StringProperty(
        name="Model name",
        description="The folder under models/players and the name /model takes "
        "(empty: from the armature's name)",
        default="",
    )
    preset: EnumProperty(name="Bone map", items=PRESET_ITEMS, default="mannequin")
    visible_only: BoolProperty(
        name="Visible meshes only",
        description="Leave out hidden meshes (other LODs, spare parts)",
        default=True,
    )
    lift: FloatProperty(
        name="Height adjustment",
        description="Game units the model is raised by (negative lowers it). The game "
        "already plants the feet on the ground; use this only if the soles still sink "
        "or float. Written as the manifest's lift line",
        default=0.0,
        soft_min=-10.0,
        soft_max=10.0,
        step=10,
        precision=1,
    )
    tail_gravity: FloatProperty(
        name="Tail gravity",
        description="How hard the game pulls a tail, cape or other following part "
        "down, game units per second squared: higher hangs it lower, 0 not at all",
        default=DEFAULT_GRAVITY,
        min=0.0,
        soft_max=4000.0,
        step=10000,
        precision=0,
    )
    overwrite_manifest: BoolProperty(
        name="Rewrite sjk_rig.cfg",
        description="Replace an existing manifest (and its map lines) with a fresh one",
        default=False,
    )
    pack: BoolProperty(
        name="One .pk3 file",
        description="Write everything into base/rig_<name>.pk3, so deleting that one "
        "file removes the model and its emotes; off writes loose folders",
        default=True,
    )
    pack_name: StringProperty(
        name="Pack name",
        description="The .pk3 file's name (empty: rig_<model name>). Give several "
        "models the same one to keep them in one pack; each export replaces only "
        "its own model's files in it",
        default="",
    )
    icon: StringProperty(
        name="Character icon",
        description="A picture (.png, .jpg or .tga) that puts the model in the "
        "game's character grid",
        default="",
        subtype="FILE_PATH",
    )
    frame_rate: IntProperty(
        name="Animation frame rate",
        description="Frames per second the actions were made at; 0 uses the scene's. "
        "Animations from Unreal games are usually 30",
        default=0,
        min=0,
        max=240,
    )
    emotes_move: BoolProperty(
        name="Keep playing while moving",
        description="The emotes go on while the player runs and jumps",
        default=True,
    )
    local_emotes: BoolProperty(
        name="Emotes stay on this computer",
        description="The emotes play for you only and are never sent to other "
        "players; off, the other SJK players on the server see them when they "
        "have the same model and emotes",
        default=False,
    )


class SJK_OT_export_rig_model(Operator):
    bl_idname = "sjk.export_rig_model"
    bl_label = "Export to SJK"
    bl_description = "Write the armature, its meshes and the ticked emotes for SJK"
    bl_options = {"REGISTER"}

    @classmethod
    def poll(cls, context):
        settings = context.scene.sjk_rig_export
        return settings.armature is not None and settings.armature.type == "ARMATURE"

    def invoke(self, context, event):
        target = self.target(context)
        if target and os.path.exists(target):
            return context.window_manager.invoke_confirm(
                self, event, message=f"Replace {target}?"
            )
        return self.execute(context)

    def target(self, context):
        """The .pk3 or the model folder the export writes."""
        root = game_data(context)
        settings = context.scene.sjk_rig_export
        name = model_name(settings)
        if not root or not name:
            return ""
        if settings.pack:
            return os.path.join(root, "base", f"{pack_name(settings, name)}.pk3")
        return os.path.join(root, "base", "models", "players", name, "model.gltf")

    def execute(self, context):
        settings = context.scene.sjk_rig_export
        armature = settings.armature
        name = model_name(settings)
        root = game_data(context)
        if not root or not os.path.isdir(root):
            self.report({"ERROR"}, "Set the GameData folder in the add-on's preferences")
            return {"CANCELLED"}
        if not name:
            self.report({"ERROR"}, "Give the model a name")
            return {"CANCELLED"}
        _, _, missing = joint_report(armature, settings.preset)
        if missing:
            self.report({"ERROR"}, f"The armature has no {', '.join(missing)} joint")
            return {"CANCELLED"}
        meshes = deformed_meshes(context, armature, settings.visible_only)
        following = following_meshes(context, armature, settings.visible_only)
        if not meshes:
            self.report({"ERROR"}, "No mesh is deformed by this armature")
            return {"CANCELLED"}
        if not armature.visible_get():
            self.report({"ERROR"}, "Show the armature first (it must be selectable)")
            return {"CANCELLED"}
        actions = [a for a in emote_actions(armature) if a.sjk_emote]
        try:
            sounds = {action.name: emote_sound(action) for action in actions}
            loops = {
                action.name: emote_sound(action, "sjk_emote_sound_loop") for action in actions
            }
            pictures = {
                action.name: picture(action.sjk_emote_icon, action.name) for action in actions
            }
            icon = picture(settings.icon, "Character icon")
        except ValueError as error:
            self.report({"ERROR"}, str(error))
            return {"CANCELLED"}
        base = os.path.join(root, "base")
        pack_path = os.path.join(base, f"{pack_name(settings, name)}.pk3")
        stage = tempfile.mkdtemp(prefix="sjk_rig_") if settings.pack else base
        folder = os.path.join(stage, "models", "players", name)
        os.makedirs(folder, exist_ok=True)
        try:
            gltf = os.path.join(folder, "model.gltf")
            self.export(context, armature, meshes, following, gltf)
            if icon:
                shutil.copyfile(icon[0], os.path.join(folder, f"icon_default{icon[1]}"))
            clips = sample_clips(context, armature, actions, name)
            written = append_clips(gltf, clips) if clips else []
            if written:
                os.makedirs(os.path.join(stage, "emotes"), exist_ok=True)
            for action, clip, seconds in written:
                sound = None
                emote_icon = None
                if pictures.get(action.name):
                    source, extension = pictures[action.name]
                    emote_icon = f"gfx/emotes/{clip}{extension}"
                    target = os.path.join(stage, *emote_icon.split("/"))
                    os.makedirs(os.path.dirname(target), exist_ok=True)
                    shutil.copyfile(source, target)
                music = None
                if sounds.get(action.name):
                    source = sounds[action.name]
                    sound = f"sound/emotes/{clip}{os.path.splitext(source)[1].lower()}"
                    target = os.path.join(stage, *sound.split("/"))
                    os.makedirs(os.path.dirname(target), exist_ok=True)
                    shutil.copyfile(source, target)
                    loop = loops.get(action.name)
                    loop_path = None
                    if loop:
                        loop_path = (
                            f"sound/emotes/{clip}_loop{os.path.splitext(loop)[1].lower()}"
                        )
                        shutil.copyfile(loop, os.path.join(stage, *loop_path.split("/")))
                    music = (
                        sound_milliseconds(source),
                        loop_path,
                        sound_milliseconds(loop) if loop else 0,
                    )
                with open(os.path.join(stage, "emotes", f"{clip}.emote"), "w",
                          encoding="utf-8", newline="\n") as file:
                    file.write(
                        emote_text(
                            action, clip, seconds, sound, emote_icon, settings, name, music
                        )
                    )
            manifest = self.old_manifest(base, name, settings, pack_path)
            with open(os.path.join(folder, MANIFEST), "w", encoding="utf-8",
                      newline="\n") as file:
                text = with_lift(manifest or manifest_text(settings.preset), settings.lift)
                file.write(
                    with_dynamic(text, dynamic_roots(following), settings.tail_gravity,
                                 own=set(armature.data.bones.keys()))
                )
            if settings.pack:
                self.pack(stage, pack_path, name)
        except Exception as error:  # report it in Blender rather than a traceback
            self.report({"ERROR"}, f"Export failed: {error}")
            return {"CANCELLED"}
        finally:
            if settings.pack:
                shutil.rmtree(stage, ignore_errors=True)
        where = (
            f"base/{os.path.basename(pack_path)}" if settings.pack
            else f"base/models/players/{name}"
        )
        emotes = ", ".join(clip for _, clip, _ in written)
        self.report({"INFO"}, f"Exported {where}. In the game: /model {name}"
                    + (f"; emotes: {emotes}" if emotes else ""))
        loose = os.path.join(base, "models", "players", name)
        if settings.pack and os.path.isdir(loose):
            self.report({"WARNING"}, f"A loose folder {loose} has the same name: remove it")
        others = self.other_packs(base, name, pack_path if settings.pack else None)
        if others:
            self.report({"WARNING"}, f"{', '.join(others)} also holds {name}: remove it")
        return {"FINISHED"}

    def other_packs(self, base, name, kept):
        """The other packs in `base` that hold model `name`."""
        inside = f"models/players/{name}/{MANIFEST}"
        found = []
        for file in sorted(os.listdir(base)):
            path = os.path.join(base, file)
            if not file.lower().endswith(".pk3") or (kept and os.path.samefile(path, kept)):
                continue
            try:
                with zipfile.ZipFile(path) as archive:
                    if inside in archive.namelist():
                        found.append(file)
            except (OSError, zipfile.BadZipFile):
                continue
        return found

    def old_manifest(self, base, name, settings, path):
        """The manifest already there (kept unless rewriting is asked), or None."""
        if settings.overwrite_manifest:
            return None
        inside = f"models/players/{name}/{MANIFEST}"
        if settings.pack:
            if os.path.exists(path):
                with zipfile.ZipFile(path) as archive:
                    if inside in archive.namelist():
                        return archive.read(inside).decode("utf-8")
            return None
        path = os.path.join(base, *inside.split("/"))
        if os.path.exists(path):
            with open(path, encoding="utf-8") as file:
                return file.read()
        return None

    def pack(self, stage, path, name):
        """Write `stage` into the pack at `path`, keeping the files of its other models."""
        partial = path + ".part"
        try:
            with zipfile.ZipFile(partial, "w", zipfile.ZIP_DEFLATED) as archive:
                written = set()
                for directory, _, files in os.walk(stage):
                    for file in files:
                        full = os.path.join(directory, file)
                        inside = os.path.relpath(full, stage).replace(os.sep, "/")
                        archive.write(full, inside)
                        written.add(inside)
                if os.path.exists(path):
                    with zipfile.ZipFile(path) as old:
                        replaced = model_files(old, name) | written
                        for entry in old.infolist():
                            if not entry.is_dir() and entry.filename not in replaced:
                                archive.writestr(entry, old.read(entry))
            os.replace(partial, path)
        except PermissionError:
            raise RuntimeError(
                f"{os.path.basename(path)} is in use: close the game, then export again"
            ) from None
        finally:
            if os.path.exists(partial):
                os.remove(partial)

    def joined(self, context, armature, following):
        """A temporary copy of `armature` with the bones of each following armature
        added under the bone it follows, where they stand on `armature`'s rest pose
        (the armature is at rest when this runs)."""
        from mathutils import Matrix

        data = armature.data.copy()
        rig = bpy.data.objects.new(f"SJK_export_{armature.name}", data)
        rig.matrix_world = armature.matrix_world
        context.scene.collection.objects.link(rig)
        inverse = rig.matrix_world.inverted()
        placed = []
        for other, followed in followers(following):
            for pose_bone in other.pose.bones:
                location, rotation, _ = (
                    inverse @ other.matrix_world @ pose_bone.matrix
                ).decompose()
                parent = pose_bone.parent.name if pose_bone.parent else followed
                placed.append((
                    pose_bone.name,
                    Matrix.LocRotScale(location, rotation, None),
                    pose_bone.bone.length,
                    parent,
                ))
        for obj in context.view_layer.objects:
            obj.select_set(False)
        rig.select_set(True)
        context.view_layer.objects.active = rig
        bpy.ops.object.mode_set(mode="EDIT")
        try:
            bones = data.edit_bones
            added = []
            for name, matrix, length, _ in placed:
                if name in bones:  # a name the body has: its own bone stays
                    continue
                bone = bones.new(name)
                bone.head = (0.0, 0.0, 0.0)
                bone.tail = (0.0, max(length, 1e-3), 0.0)
                bone.matrix = matrix
                added.append(name)
            for name, _, _, parent in placed:
                if name in added and parent in bones:
                    bones[name].parent = bones[parent]
        finally:
            bpy.ops.object.mode_set(mode="OBJECT")
        data.pose_position = "REST"
        return rig

    def baked(self, context, rig, source):
        """A temporary copy of `source` (a mesh a following armature deforms) as it
        stands now, keeping its weights, rigged to `rig` (whose joined bones stand
        where its own do)."""
        depsgraph = context.evaluated_depsgraph_get()
        data = bpy.data.meshes.new_from_object(
            source.evaluated_get(depsgraph), depsgraph=depsgraph
        )
        data.transform(rig.matrix_world.inverted() @ source.matrix_world)
        copy = bpy.data.objects.new(f"SJK_export_{source.name}", data)
        context.scene.collection.objects.link(copy)
        copy.parent = rig
        # The groups in the source's order, so the weights the mesh kept find them.
        for group in sorted(source.vertex_groups, key=lambda group: group.index):
            copy.vertex_groups.new(name=group.name)
        modifier = copy.modifiers.new("Armature", "ARMATURE")
        modifier.object = rig
        return copy

    def export(self, context, armature, meshes, following, path):
        if context.mode != "OBJECT":
            bpy.ops.object.mode_set(mode="OBJECT")
        selected = [obj for obj in context.selected_objects]
        active = context.view_layer.objects.active
        pose_position = armature.data.pose_position
        copies, meshes_data, materials = [], [], {}
        rig = None
        try:
            armature.data.pose_position = "REST"
            context.view_layer.update()
            # With following armatures, a copy of the armature holding their bones
            # too is exported in its place.
            if following:
                rig = self.joined(context, armature, following)
                context.view_layer.update()
            exported = rig or armature
            for source in meshes:
                copy = source.copy()
                copy.data = source.data.copy()
                copy.name = f"SJK_export_{source.name}"
                context.scene.collection.objects.link(copy)
                copies.append(copy)
                meshes_data.append(copy.data)
                keep_shape_mix(copy)
                apply_masks(context, copy)
                if rig is not None:
                    copy.parent = rig
                    for modifier in copy.modifiers:
                        if modifier.type == "ARMATURE" and modifier.object == armature:
                            modifier.object = rig
            for source, _, _ in following:
                copy = self.baked(context, rig, source)
                copies.append(copy)
                meshes_data.append(copy.data)
            for copy in copies:
                for index, slot in enumerate(copy.material_slots):
                    key = slot.material.name if slot.material else ""
                    if key not in materials:
                        materials[key] = plain_material(slot.material)
                    if slot.link == "OBJECT":
                        slot.material = materials[key]
                    else:
                        copy.data.materials[index] = materials[key]
            for obj in context.view_layer.objects:
                obj.select_set(False)
            exported.select_set(True)
            for copy in copies:
                copy.select_set(True)
            context.view_layer.objects.active = exported
            bpy.ops.export_scene.gltf(
                filepath=path,
                export_format="GLTF_SEPARATE",
                use_selection=True,
                export_animations=False,
                export_morph=False,
                export_skins=True,
                export_yup=True,
                export_apply=False,
                export_rest_position_armature=True,
                export_materials="EXPORT",
                export_image_format="AUTO",
            )
        finally:
            for copy in copies:
                bpy.data.objects.remove(copy, do_unlink=True)
            for data in meshes_data:
                if data.users == 0:
                    bpy.data.meshes.remove(data)
            for material in materials.values():
                bpy.data.materials.remove(material)
            for image in [image for image in bpy.data.images if image.get("sjk_export")]:
                bpy.data.images.remove(image)
            if rig is not None:
                rig_data = rig.data
                bpy.data.objects.remove(rig, do_unlink=True)
                bpy.data.armatures.remove(rig_data)
            armature.data.pose_position = pose_position
            for obj in context.view_layer.objects:
                obj.select_set(obj in selected)
            context.view_layer.objects.active = active


class SJK_PT_rig_export(Panel):
    bl_label = "SJK Rig Model"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "SJK"

    def draw(self, context):
        layout = self.layout
        settings = context.scene.sjk_rig_export
        layout.prop(settings, "armature")
        armature = settings.armature
        if armature is not None:
            found, expected, missing = joint_report(armature, settings.preset)
            meshes = deformed_meshes(context, armature, settings.visible_only)
            icon = "ERROR" if missing or found < expected // 2 else "CHECKMARK"
            box = layout.box()
            box.label(text=f"Bone map joints found: {found} of {expected}", icon=icon)
            if missing:
                box.label(text=f"Missing: {', '.join(missing)}", icon="CANCEL")
            following = following_meshes(context, armature, settings.visible_only)
            box.label(text=f"Meshes: {len(meshes)}", icon="MESH_DATA")
            if following:
                bones = ", ".join(sorted({bone for _, bone, _ in following}))
                box.label(text=f"Following meshes: {len(following)} (on {bones}), "
                          "moving on their own", icon="CONSTRAINT_BONE")
                box.prop(settings, "tail_gravity")
        layout.prop(settings, "model_name")
        layout.prop(settings, "icon")
        layout.prop(settings, "preset")
        layout.prop(settings, "visible_only")
        layout.prop(settings, "lift")
        layout.prop(settings, "overwrite_manifest")
        layout.prop(settings, "pack")
        if settings.pack:
            layout.prop(settings, "pack_name")
        name = model_name(settings)
        if armature is not None:
            actions = emote_actions(armature)
            box = layout.box()
            box.label(text="Emotes (actions of this armature)", icon="ACTION")
            if not actions:
                box.label(text="No action animates this armature")
            for action in actions:
                row = box.row(align=True)
                row.prop(action, "sjk_emote", text=action.name)
                row.prop(action, "sjk_emote_loop", text="", icon="FILE_REFRESH", toggle=True)
                if action.sjk_emote:
                    column = box.column(align=True)
                    if name:
                        column.label(text=f"sjkemote {emote_id(name, action)}", icon="PLAY")
                    column.prop(action, "sjk_emote_name", text="Name")
                    column.prop(action, "sjk_emote_sound", text="Sound")
                    column.prop(action, "sjk_emote_sound_loop", text="Music loop")
                    column.prop(action, "sjk_emote_icon", text="Icon")
            if actions:
                fps = frame_rate(context)
                box.prop(settings, "frame_rate")
                box.label(text=f"Playing at {fps:g} frames a second")
                box.prop(settings, "emotes_move")
                box.prop(settings, "local_emotes")
        root = game_data(context)
        if root and name:
            where = (
                f"base/{pack_name(settings, name)}.pk3" if settings.pack
                else f"base/models/players/{name}"
            )
            layout.label(text=where, icon="FILE_FOLDER")
        elif not root:
            layout.label(text="GameData not found: set it in the preferences", icon="ERROR")
        layout.operator(SJK_OT_export_rig_model.bl_idname, icon="EXPORT")
        note = layout.column(align=True)
        note.scale_y = 0.8
        note.label(text="For your own files, on your own disk.")
        note.label(text="Never upload or share models you have no rights to.")


CLASSES = (
    SJKRigExportPreferences,
    SJKRigExportSettings,
    SJK_OT_export_rig_model,
    SJK_PT_rig_export,
)

# What each action keeps for the export, saved in the blend file.
ACTION_PROPERTIES = {
    "sjk_emote": lambda: BoolProperty(
        name="Emote", description="Bring this action to the game as an emote"
    ),
    "sjk_emote_loop": lambda: BoolProperty(
        name="Loop", description="The emote loops until stopped (sjkemote stop)"
    ),
    "sjk_emote_sound": lambda: StringProperty(
        name="Sound",
        description="A .wav or .mp3 played with the emote",
        subtype="FILE_PATH",
    ),
    "sjk_emote_sound_loop": lambda: StringProperty(
        name="Music loop",
        description="A .wav or .mp3 that follows the sound and repeats while the "
        "emote plays (music made of an intro and a loop)",
        subtype="FILE_PATH",
    ),
    "sjk_emote_name": lambda: StringProperty(
        name="Name",
        description="What the game calls the emote (empty: the action's name)",
        maxlen=32,
    ),
    "sjk_emote_icon": lambda: StringProperty(
        name="Icon",
        description="A picture (.png, .jpg or .tga) the quick wheel shows for the emote",
        subtype="FILE_PATH",
    ),
}


def register():
    for cls in CLASSES:
        bpy.utils.register_class(cls)
    bpy.types.Scene.sjk_rig_export = PointerProperty(type=SJKRigExportSettings)
    for key, prop in ACTION_PROPERTIES.items():
        setattr(bpy.types.Action, key, prop())


def unregister():
    for key in ACTION_PROPERTIES:
        delattr(bpy.types.Action, key)
    del bpy.types.Scene.sjk_rig_export
    for cls in reversed(CLASSES):
        bpy.utils.unregister_class(cls)
