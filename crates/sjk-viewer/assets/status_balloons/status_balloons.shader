// The status balloons (SJK; status_balloons.rs, scripts/status_balloon_assets.py): the talk
// balloon over an SJK player, showing why it is up. Typing: the three dots lit in turn. A
// menu or the console: a pause sign. Away (alt-tabbed or minimised): a moon and rising z's.
// Drawn as stock's gfx/mp/chat_icon is (shaders/mp.shader).

gfx/sjk/status/typing
{
	nopicmip
	nomipmaps
	{
		animMap 3 gfx/sjk/status/typing_1 gfx/sjk/status/typing_2 gfx/sjk/status/typing_3
		blendFunc GL_SRC_ALPHA GL_ONE_MINUS_SRC_ALPHA
	}
}

gfx/sjk/status/busy
{
	nopicmip
	nomipmaps
	{
		clampmap gfx/sjk/status/busy
		blendFunc GL_SRC_ALPHA GL_ONE_MINUS_SRC_ALPHA
	}
}

gfx/sjk/status/away
{
	nopicmip
	nomipmaps
	{
		animMap 1.5 gfx/sjk/status/away_1 gfx/sjk/status/away_2 gfx/sjk/status/away_3
		blendFunc GL_SRC_ALPHA GL_ONE_MINUS_SRC_ALPHA
	}
}
