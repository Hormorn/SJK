// The holocron's loot-box tiers (SJK; see README.md): each tier's face (the Jedi
// cube, the Great Holocron's pentagons, the Sith pyramid, the engram). Lit (through
// its baked normal map when sculpted), also glowing faintly of itself, and its lit
// parts kept bright and in the dynamic glow, breathing; the tiers climb in how
// bright. The engram's plates also breathe out and back (deformVertexes) and flash
// with a sheen read by the view as they turn.
// Made by scripts/holocron_assets.py from its TIERS table: edit that, not this.
models/sjk/holocron_uncommon
{
	q3map_nolightmap
	{
		map models/sjk/holocron_uncommon
		normalMap models/sjk/holocron_uncommon_n
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_uncommon
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.35 0 0 0
	}
	{
		map models/sjk/holocron_uncommon_glow
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.4 0.1 0 0.3
		glow
	}
}
models/sjk/holocron_rare
{
	q3map_nolightmap
	{
		map models/sjk/holocron_rare
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_rare
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.4 0 0 0
	}
	{
		map models/sjk/holocron_rare_glow
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.5 0.15 0 0.35
		glow
	}
}
models/sjk/holocron_legendary
{
	q3map_nolightmap
	{
		map models/sjk/holocron_legendary
		normalMap models/sjk/holocron_legendary_n
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_legendary
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.45 0 0 0
	}
	{
		map models/sjk/holocron_legendary_glow
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.6 0.2 0 0.4
		glow
	}
}
models/sjk/holocron_mythical
{
	q3map_nolightmap
	deformVertexes wave 1000 sin 0.2 0.2 0 0.35
	{
		map models/sjk/holocron_mythical
		normalMap models/sjk/holocron_mythical_n
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_mythical
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.35 0.08 0.25 0.35
	}
	{
		map models/sjk/holocron_mythical_glow
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.85 0.35 0 0.55
		glow
	}
	{
		map models/sjk/holocron_mythical_env
		blendFunc GL_ONE GL_ONE
		tcGen environment
		tcMod scale 1.5 1.5
		tcMod scroll 0.05 0
		rgbGen wave sin 0.8 0.2 0 0.3
	}
}
