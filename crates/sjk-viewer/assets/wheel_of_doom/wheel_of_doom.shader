// The WHEEL OF DOOM's wheel and stand (SJK; see README.md): lit by the world's light as
// a player model is, the jackpot's gold, the words and the pointer glowing faintly on
// top so the dark wheel reads in a dark map. Both sides drawn.
models/sjk/wheel_of_doom
{
	cull none
	{
		map models/sjk/wheel_of_doom
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/wheel_of_doom_glow
		blendFunc add
		rgbGen wave sin 0.8 0.2 0 0.6
	}
}
