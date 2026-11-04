// The Mining toy's pickaxe, vein, glint and what a vein gives (SJK; see README.md).
// Flat colours from one small atlas, lit by the world's light as a player model is.
models/sjk/mining
{
	cull none
	{
		map models/sjk/mining
		rgbGen lightingDiffuse
	}
}

// The vein's crystals and the glowing ores: the same atlas, full bright, so they shine
// in a dark cave.
models/sjk/mining_glow
{
	cull none
	{
		map models/sjk/mining
		rgbGen identity
	}
}

// The rock of the vein and of every chunk: Yavin's rock from the installed game (only its
// name is here, as the Fishing pond's stones).
models/sjk/mining_rock
{
	{
		map textures/yavin/s_rock4
		rgbGen lightingDiffuse
	}
}

// The glint on the vein, the moment to strike: a star added over the world.
models/sjk/mining_glint
{
	cull none
	nopicmip
	{
		map models/sjk/mining_glint
		blendFunc GL_ONE GL_ONE
		rgbGen identity
	}
}
