// The mystery box's chest and lid (SJK; see README.md): wood and iron lit by the
// world's light as a player model is, the question marks' glow added on top, pulsing.
models/sjk/mystery_box
{
	{
		map models/sjk/mystery_box
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/mystery_box_glow
		blendFunc add
		rgbGen wave sin 0.75 0.25 0 0.5
	}
}

// The faint halo of light rising over the chest, so a box is found from afar (SJK; see
// README.md): added, seen from inside and out, its rays drifting slowly round, breathing.
models/sjk/mystery_box_halo
{
	cull none
	nopicmip
	{
		map models/sjk/mystery_box_halo
		blendFunc add
		rgbGen wave sin 0.85 0.15 0 0.35
		tcMod scroll 0.02 0
	}
}

// The teddy bear a box stops on when it blows up (SJK; see README.md): plush, lit as a
// player model is.
models/sjk/teddy_bear
{
	{
		map models/sjk/teddy_bear
		rgbGen lightingDiffuse
	}
}

// The box's loot of 11/11/2026 (SJK; see README.md, scripts/box_loot_model.py): the Ray
// Gun and the piles of gold coins, lit as a player model is, with a glint that slides
// over the metal as the eye moves.
models/sjk/box_loot
{
	{
		map models/sjk/box_loot
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/box_loot
		blendFunc GL_DST_COLOR GL_ONE
		tcGen environment
		rgbGen const ( 0.35 0.35 0.35 )
	}
}

// The Ray Gun's glowing tube and its bolt: green light, added, pulsing a little.
models/sjk/box_loot_glow
{
	nopicmip
	{
		map models/sjk/box_loot_glow
		blendFunc add
		rgbGen wave sin 0.85 0.15 0 2
	}
}
