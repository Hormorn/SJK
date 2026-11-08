// Korriban Ragnos: the instant a strike lands perfectly, held in a blade forged in the Sith
// tombs of Korriban. Its own body is obsidian black with a crimson rim, fine red cracks of light
// breathing through it; cursed flames of black and red lick up its edges; the air round it ripples
// as if space bends near it, and its image splits for a heartbeat into red and blue ghosts each
// time black sparks crackle off its edges. Red-black cinders drift slowly round it. Every 18
// seconds the black flash itself: the space round the blade darkens and draws in, the cracks
// glow hotter and turn from red to violet, then it strikes: black lightning wreathed in red
// bursts out of the blade's whole body all round, the blade blazes white-violet for a moment and
// the cursed flames flare tall, and it settles back to black. It opens like a crack of red light
// splitting up from the emitter, and goes out as its black falls apart into drifting cinders. Swung,
// the air ripples harder behind it. A clash is a small black flash. Its trail is the black flash
// (`@trail ragnos`). No figures, no rings.
// @part Obsidian body
// @part Dark heat
// @part Black lightning
// @part The black flash
// @part Cursed flames
// @part Bleeding crystal
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail ragnos
// @palette {"corona.rim_cool":[0.35,0.0,0.03],"corona.rim_hot":[0.95,0.08,0.1],"corona.inner":[1.0,0.45,0.42],"core.white":[1.0,0.85,0.85],"core.fringe_cool":[0.6,0.0,0.05],"core.fringe_hot":[1.0,0.3,0.3],"light.color":[0.9,0.06,0.08],"trail":[0.9,0.06,0.1]}
// @speed 2.6

const vec3 BF_RED = vec3(0.95, 0.06, 0.08);
const vec3 BF_DEEP = vec3(0.35, 0.0, 0.03);
const vec3 BF_BLACK = vec3(0.02, 0.0, 0.01);
const vec3 BF_HOT = vec3(1.0, 0.55, 0.5);
const vec3 BF_WHITE = vec3(1.0, 0.93, 0.92);

float bf_life(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, 0.9, uAge - 2000.0);
  return smoothstep(0.0, 0.45, uAge);
}
float bf_after(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, 0.4, uAge - 2000.0);
  return smoothstep(0.4, 1.3, uAge);
}
float bf_top(){ return (uLength + 0.4)*bf_life(); }
float bf_flow(vec2 p){
  float v = sin(p.x + 1.5*sin(p.y*0.9 + 0.4));
  v += 0.8*sin(p.y*1.3 + 1.2*sin(p.x*1.2 + 1.9));
  v += 0.5*sin((p.x - p.y)*2.0 + 1.0*sin(p.y*2.1 + 0.7));
  return 0.5 + v/4.6;
}
// The black flash on its 18-second cycle: `draw` the space darkening and drawing in before it,
// `strike` (0 to 1 over its burst, below 0 when none), smooth both ways.
float bf_flash(out float draw){
  draw = 0.0;
  if (part(3.0) < 0.5) return -1.0;
  float c = mod(uClock + uSeed*8.0, 18.0);
  float on = bf_after();
  draw = smoothstep(11.0, 13.5, c)*(1.0 - smoothstep(13.4, 14.3, c))*on;
  return (c >= 13.5 && c < 15.5 && on > 0.5) ? (c - 13.5)/2.0 : -1.0;
}
float bf_half(float y, float top){
  float s = clamp(y/max(uLength, 1.0), 0.0, 1.0);
  float w = 0.58 + 0.1*smoothstep(0.0, 0.12, s);
  float tip = clamp((top - y)/2.8, 0.0, 1.0);
  return w*sqrt(tip);
}
float bf_out(vec2 b, float top, out float half){
  float y = clamp(b.y, 0.0, max(top, 0.0));
  half = bf_half(y, top);
  return length(vec2(max(abs(b.x) - half, 0.0), b.y - y));
}

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = bf_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half;
    float d = bf_out(b, top, half);
    float draw;
    float strike = bf_flash(draw);
    col = BF_RED*halo(d, 0.45)*0.9 + BF_DEEP*halo(d, 1.3)*0.7 + BF_HOT*halo(d, 0.12)*0.4;
    // effect 2: dark heat (1/11/2026, replacing the rippling rings Creyon disliked): dark red
    // heat rising off the blade in slow wavering columns, stronger as it is swung and as the
    // flash draws in
    if (part(1.0) > 0.5 && uFx > 0.0){
      float sw = smoothstep(0.1, 0.8, clamp(uSwing, 0.0, 1.0));
      float col1 = bf_flow(vec2(b.x*0.35 + clamp(b.x*2.0, -1.0, 1.0)*3.0, b.y*0.12 - tCalm()*0.45));
      // (update 16) torn, flame-like columns instead of soft round glows
      float heat = smoothstep(0.55, 0.72, col1)*(1.0 - smoothstep(0.4, 2.4 + 1.0*sw + 1.0*draw, d))*smoothstep(0.15, 0.6, d);
      col += BF_DEEP*heat*(0.6 + 0.6*sw + 0.8*draw)*smoothstep(-0.5, 1.5, b.y)*(1.0 - smoothstep(top - 2.0, top + 0.5, b.y))*uFx;
    }
    // effect 6 (with Black sparks): the split, each time sparks crackle its image splits for a
    // heartbeat into a red ghost on one side and a blue one on the other
    if (part(2.0) > 0.5 && uFx > 0.0){
      float beat = pow(0.5 + 0.5*sin((uTime*0.5)*0.9*6.2832 + uSeed*3.0), 12.0) + 0.8*draw;
      if (beat > 0.01){
        float off = 0.35 + 0.25*beat;
        float yy = clamp(b.y, 0.0, top);
        float dl = length(vec2(b.x + off, b.y - yy));
        float dr2 = length(vec2(b.x - off, b.y - yy));
        col += (vec3(1.0, 0.05, 0.1)*halo(dl, 0.35) + vec3(0.15, 0.3, 1.0)*halo(dr2, 0.35))*beat*0.6*uFx;
      }
    }
    // the space drawing in before the flash: the light round it dims
    col *= 1.0 - 0.5*draw*(1.0 - halo(d, 0.6));
    if (strike >= 0.0) col += (BF_HOT*halo(d, 1.5 + 3.0*strike) + BF_RED*halo(d, 4.0)*0.6)*(1.0 - strike)*(1.0 - strike)*1.2;
  }
  // effect 9: the crimson ember, a red glow smouldering round the emitter, breathing slowly
  {
    float br = 0.75 + 0.25*sin((uTime*0.5)*0.9);
    float de = length(vec2(b.x*0.8, max(b.y, 0.0)));
    col += BF_RED*halo(de, 1.4)*0.5*br*(1.0 - smoothstep(-1.0, 3.0, b.y)*0.6);
  }
  col += lvl(g)*BF_RED*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.5;
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core_d(vec3 c, vec2 b, float px){
  return c*vec3(1.0, 0.4, 0.4)*0.25*(1.0 - smoothstep(0.3, 2.0, b.y));
}

// effect 1: the obsidian body, black glass with a crimson rim and fine red cracks of light
// breathing through it (warped sines, so no grid); it opens as a red crack splitting up from the
// emitter; at the flash it blazes white-red
// effect 5: cursed flames, black flames with red edges licking up off both its sides in smooth
// tongues (warped sines), flaring tall at the black flash
vec4 bf_flames(vec2 b, float d, float top, float strike){
  // (update 18, Creyon: not round; they must come and fade, and not look childish) real fire: a
  // field of warped currents rising along the blade eats away at a body of flame that thins with
  // distance, so tongues form, tear free and burn out by themselves; black where the fire is deep,
  // through crimson to a last red light at its thin edges
  if (part(4.0) < 0.5 || uFx <= 0.0 || d > 6.0 || b.y < 0.8) return vec4(0.0);
  float side = b.x < 0.0 ? 2.7 : 0.0;
  float flare = strike >= 0.0 ? sin(3.1416*min(strike*1.3, 1.0)) : 0.0;
  float tm = uClock*0.85;
  vec2 p = vec2(side + d*0.62, b.y*0.24 - tm*1.05 - d*0.2);
  vec2 w = vec2(rxFlow(p*1.3 + 11.0), rxFlow(p*1.7 + 29.0)) - 0.5;
  float v = 0.68*rxFlow(p + w*1.5) + 0.36*rxFlow(p*2.5 + w*2.4 + 7.0);
  // the fire comes and goes along the blade in slow waves
  float come = smoothstep(0.28, 0.66, rxFlow(vec2(b.y*0.1 - tm*0.32, side*1.3 + tm*0.11)));
  float reach = (1.1 + 3.4*come)*(1.0 + 0.7*flare)*bf_after();
  float f = v - 0.2 - d/max(reach, 0.05)*0.72;
  float a = smoothstep(0.0, 0.26, f);
  float span = smoothstep(0.8, 4.5, b.y)*(1.0 - smoothstep(top - 3.5, top - 0.1, b.y));
  // hot at its thin edges, as fire round a black coal
  vec3 col = mix(vec3(1.0, 0.45, 0.2), BF_RED*1.05, smoothstep(0.0, 0.05, f));
  col = mix(col, BF_DEEP*1.1, smoothstep(0.04, 0.16, f));
  col = mix(col, BF_BLACK, smoothstep(0.08, 0.3, f));
  return vec4(col, a*0.9*span);
}

vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = bf_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.5) return o;
  float half;
  float dOut = bf_out(b, top, half);
  {
    float dr0;
    float st0 = bf_flash(dr0);
    vec4 fl = bf_flames(b, dOut, top, st0);
    o = vec4(mix(o.rgb, fl.rgb, fl.a), max(o.a, fl.a));
  }
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float draw;
  float strike = bf_flash(draw);
  vec3 col = mix(BF_BLACK, BF_DEEP*0.6, smoothstep(0.5, 0.95, u));
  col += BF_RED*smoothstep(0.78, 1.0, u)*1.1;
  if (part(0.0) > 0.5){
    float c1 = abs(bf_flow(vec2(b.x*3.5 + 2.0, b.y*0.35 + (uTime*0.5)*0.15)) - 0.5);
    float c2 = abs(bf_flow(vec2(b.x*5.0 - 1.0, b.y*0.6 - (uTime*0.5)*0.1 + 4.0)) - 0.5);
    float crack = (1.0 - smoothstep(0.0, 0.035, c1)) + (1.0 - smoothstep(0.0, 0.025, c2))*0.6;
    float breathe = 0.55 + 0.45*sin((uTime*0.5)*0.8 + b.y*0.1);
    col += mix(BF_RED, vec3(0.7, 0.2, 1.0), draw)*crack*breathe*(0.8 + 1.5*draw);
    col += BF_HOT*crack*crack*0.3*breathe;
  }
  // opening: a red crack splitting up it before the black fills in
  float opening = uAge < 2000.0 ? 1.0 - smoothstep(0.3, 1.0, uAge) : 0.0;
  col = mix(col, BF_RED*1.2 + BF_HOT*halo(b.x, 0.15), opening*0.85);
  // going out: its black breaking up into cinders from the edges in
  if (uAge >= 2000.0) col += BF_RED*smoothstep(0.4, 1.0, u)*clamp((uAge - 2000.0)/0.6, 0.0, 1.0);
  if (strike >= 0.0) col = mix(col, mix(BF_WHITE, vec3(0.85, 0.7, 1.0), 0.5), smoothstep(0.0, 0.06, strike)*(1.0 - smoothstep(0.05, 0.7, strike))*0.85);
  col += BF_HOT*halo(top - b.y, 0.8)*0.4 + BF_HOT*halo(max(b.y, 0.0), 0.8)*0.6;
  col = mix(col, BF_WHITE, 0.5*clamp(uClash, 0.0, 1.0));
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// A jagged bolt from a to c through the ray (12 kinks, sideways and in depth, widest at its
// middle); f returns where along it the nearest point is.
float bf_bolt(vec3 ro, vec3 rd, vec3 a, vec3 c, float seed, float jag, out float f){
  vec3 ab = c - a;
  float len = length(ab);
  vec3 side = normalize(cross(ab, vec3(0.31, 0.12, 0.94)) + vec3(1e-4));
  vec3 up = normalize(cross(side, ab) + vec3(1e-4));
  float bWs = 0.0;
  float best = 1e3, s;
  f = 0.0;
  vec3 prev = a;
  for (int i = 1; i <= 12; i++){
    float k = float(i)/12.0;
    float env = sin(3.1416*k);
    vec3 p = a + ab*k;
    if (i < 12) p += (side*(h11(seed*7.3 + k*29.0) - 0.5) + up*(h11(seed*2.9 + k*13.0) - 0.5))*jag*len*env;
    float d = raySeg(ro, rd, prev, p, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); f += ((float(i) - 1.0 + s)/12.0 - f)*wN/bWs; }
    prev = p;
  }
  return best;
}
// black lightning: a black thread (drawn as missing light: the red glow round it, its heart
// dark) wrapped in crimson
vec3 bf_black_bolt(float d, float px, float w){
  return BF_RED*(halo(d, w*5.0 + 0.25)*0.9 - halo(d, max(w, px))*0.6) + BF_HOT*halo(d, w*2.0)*0.15;
}

// effect 3: black sparks, short forks of black lightning with red halos crackling off the edges
vec3 bf_sparks(vec3 ro, vec3 rd, float px, float draw){
  vec3 acc = vec3(0.0);
  float L = uLength, f;
  if (fxNear(ro, rd, 0.0, L) > 16.0) return acc;
  for (int i = 0; i < 5; i++){
    float k = float(i);
    float cyc = (uTime*0.5)*(0.9 + 0.3*h11(k*3.1)) + h11(k*5.7);
    float n = floor(cyc), q = fract(cyc);
    if (q > 0.4) continue;
    float life = smoothstep(0.0, 0.04, q)*(1.0 - smoothstep(0.12, 0.4, q))*(1.0 + draw);
    float a = h11(n*4.3 + k*1.9);
    float y = L*(0.1 + 0.85*h11(n*1.7 + k));
    vec3 pa = around(0.6, a, y);
    vec3 pc = around(1.8 + 1.5*h11(n*8.3 + k), a + 0.06*(h11(n*6.1 + k) - 0.5), y + (h11(n*2.9 + k) - 0.5)*3.0);
    float d = bf_bolt(ro, rd, pa, pc, n*13.7 + k*3.0, 0.3, f);
    acc += bf_black_bolt(d, px, 0.05)*life*(1.0 - 0.5*f);
  }
  return acc;
}

// effect 4: the black flash's burst, black lightning wreathed in red thrown out of the blade's
// body all round, and a ring of red pressure rolling out
vec3 bf_burst(vec3 ro, vec3 rd, float px, float strike){
  if (strike < 0.0) return vec3(0.0);
  vec3 acc = vec3(0.0);
  float L = uLength, f;
  float fade = (1.0 - strike)*(1.0 - strike)*smoothstep(0.0, 0.04, strike);
  for (int i = 0; i < 9; i++){
    float k = float(i);
    float a = (k + 0.5)/9.0 + 0.05*h11(k*3.3);
    float y = L*(0.25 + 0.6*h11(k*5.9));
    float reach = (3.0 + 5.0*sqrt(strike))*(0.7 + 0.3*h11(k*1.3));
    vec3 pa = around(0.6, a, y);
    vec3 pc = around(reach, a + 0.04, y + (h11(k*7.7) - 0.5)*6.0);
    float d = bf_bolt(ro, rd, pa, pc, k*11.0 + floor(strike*5.0), 0.3, f);
    acc += bf_black_bolt(d, px, 0.07)*fade*(1.0 - 0.4*f)*roomFade(reach)*1.4;
  }
  return acc;
}

// effects 5 and 6: opening, red light spilling up out of the crack as it splits; going out,
// cinders of its black, red at their edges, drifting off and falling away
vec3 bf_moments(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float t;
  if (uAge < 0.9){
    float on = smoothstep(0.0, 0.1, uAge)*(1.0 - smoothstep(0.4, 0.9, uAge));
    float y = bf_top();
    float d = rayPt(ro, rd, vec3(0.0, y, 0.0), t);
    acc += (BF_HOT*halo(d, 0.4) + BF_RED*halo(d, 1.5)*0.6)*on;
  } else if (uAge >= 2000.0 && uAge < 2001.6){
    float q = clamp((uAge - 2000.0)/1.5, 0.0, 1.0);
    for (int i = 0; i < 10; i++){
      float k = float(i);
      vec3 p = around(0.6 + 1.8*q*h11(k*4.1), h11(k*6.3) + 0.1*q, uLength*h11(k*2.9) + 1.0*q - 3.0*q*q);
      float d = rayPt(ro, rd, p, t);
      float s = 0.1 + 0.08*h11(k*1.7);
      acc += (BF_RED*halo(d, s*0.8)*1.2 + BF_DEEP*halo(d, s*2.5)*0.4)*(1.0 - q)*roomAt(p);
    }
  }
  return acc;
}

// effect 9: red-black cinders drifting slowly round the blade, glowing at their edges
vec3 bf_cinders(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float L = uLength, t;
  if (fxNear(ro, rd, 0.0, L) > 16.0) return acc;
  for (int i = 0; i < 6; i++){
    float k = float(i);
    float a = h11(k*3.1) + (uTime*0.5)*(0.025 + 0.02*h11(k*1.7));
    float y = L*(0.1 + 0.85*fract(h11(k*7.7) + (uTime*0.5)*0.012));
    vec3 p = around(1.4 + 1.6*h11(k*5.5), a, y);
    float d = rayPt(ro, rd, p, t);
    float s = 0.09 + 0.06*h11(k*2.2);
    acc += (BF_RED*halo(d, s*0.8)*1.0 + BF_DEEP*halo(d, s*3.0)*0.3)*(0.6 + 0.4*sin((uTime*0.5)*1.3 + k))*roomAt(p);
  }
  return acc;
}

// effect 7: the clash, a small black flash: a red flare and three forks of black lightning
vec3 bf_clash(vec3 ro, vec3 rd, float px){
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl <= 0.01) return vec3(0.0);
  vec3 at = vec3(0.0, uLength*0.6, 0.0);
  float t, f;
  if (rayPtRaw(ro, rd, at, t) > 7.0) return vec3(0.0);
  vec3 acc = BF_HOT*halo(rayPt(ro, rd, at, t), 0.9)*cl*1.3;
  for (int i = 0; i < 4; i++){
    float k = float(i);
    vec3 pc = at + around(2.5 + 2.0*(1.0 - cl), k/4.0 + 0.1, (h11(k*3.3) - 0.5)*3.0);
    float d = bf_bolt(ro, rd, at, pc, k*7.0 + floor(cl*6.0), 0.3, f);
    acc += bf_black_bolt(d, px, 0.06)*cl;
  }
  return acc;
}

// Sith lightning leaping off the blade (update 16): four forked bolts, each lit for a third of a
// second about once a second, thrown two to five units out from anywhere along the blade, a fork
// splitting off its middle; white-hot cores in a heavy crimson glow
vec3 bf_leap(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float L = uLength;
  if (fxNear(ro, rd, 0.0, L) > 16.0) return acc;
  for (int i = 0; i < 3; i++){
    float k = float(i);
    float cyc = uTime*(0.85 + 0.3*h11(k*3.1)) + h11(k*5.7);
    float n = floor(cyc), q = fract(cyc);
    if (q > 0.33) continue;
    float life = smoothstep(0.0, 0.03, q)*(1.0 - smoothstep(0.1, 0.33, q));
    float a = h11(n*4.3 + k*1.9);
    float y = L*(0.08 + 0.85*h11(n*1.7 + k));
    vec3 pa = around(0.55, a, y);
    vec3 pc = around(2.5 + 2.5*h11(n*8.3 + k), a + 0.05*(h11(n*6.1 + k) - 0.5), y + (h11(n*2.9 + k) - 0.4)*4.0);
    vec3 pm = mix(pa, pc, 0.5);
    vec3 pf = pm + around(1.2 + 1.2*h11(n*3.3 + k), a + 0.12, (h11(n*7.1 + k) - 0.5)*3.0);
    float d = min(bolt3(ro, rd, pa, pc, n*13.7 + k*3.0, 0.3, px), bolt3(ro, rd, pm, pf, n*5.9 + k, 0.35, px));
    acc += (vec3(1.0, 0.85, 0.85)*halo(d, max(0.05, px))*2.0 + BF_RED*halo(d, 0.35)*1.0 + BF_RED*halo(d, 1.3)*0.25)*life;
  }
  return acc;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float after = bf_after();
  float draw;
  float strike = bf_flash(draw);
  if (part(4.0) > 0.5 && after > 0.01) fx += bf_cinders(ro, rd, px)*after;
  fx += bf_burst(ro, rd, px, strike);
  fx += bf_moments(ro, rd, px) + bf_clash(ro, rd, px);
  // the Sith lightning (1/11/2026, lighter: crimson bolts on the lab's shells, one look-up a
  // pixel, where the old black bolts cost a loop of 3D segments twice over)
  if (part(2.0) > 0.5 && after > 0.01){
    float flare = strike >= 0.0 ? 1.0 - strike : 0.0;
    float y1 = uLength - 0.8;
    // (update 16, Creyon: "the lightning is really weak and sad") Sith lightning over the whole
    // blade to its point: white-hot forks in a strong crimson glow crawling over it at a steady
    // quick crackle (its rate no longer jumps with the flash; the flash brightens it), longer
    // bolts further out, and forked bolts leaping off it
    float boost = 1.0 + 1.5*flare + 1.5*clamp(uClash, 0.0, 1.0);
    float top = bf_top();
    // (update 31, Creyon) its lightning turns with the blade under an RGB
    uTurnFx += 2.4*boost*shBolts(ro, rd, px, 0.75, 0.4, top - 0.15, 4.0, 6.0, 2.5, 0.35, 3.0, mix(BF_HOT, vec3(1.0), 0.5), BF_RED*1.3)*after*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
    uTurnFx += (bf_leap(ro, rd, px)*boost*after)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
  }
  return fx*uFx;
}

// dark smoke curling up off the cursed flames, from well above the hilt
vec4 sig_vol_solid_base(vec3 ro, vec3 rd, float px){
  if (uFx <= 0.0 || part(4.0) < 0.5 || bf_after() <= 0.01 || fxNear(ro, rd, 0.0, uLength) > 16.0) return vec4(0.0);
  // (update 16, Creyon: the effects round it looked round) the smoke shells are gone; the cursed
  // flames (drawn in the blade's picture, jagged) carry the dark
  return vec4(0.0);
}


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(5.0) > 0.5) fx += x14_korriban_ragnos(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  return fx*hiltFade(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = sig_vol_solid_base(ro, rd, px);
  return o*hiltFade(ro, rd);
}

// effect 6 (1/11/2026, update 14): the bleeding crystal, the blade bleeding as a Sith crystal does:
// beads of red light well up on its face, run slowly down it leaving a glowing trail, gather into
// a drop and fall away into the dark
vec3 x14_korriban_ragnos(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return acc;
  float t, s;
  for (int i = 0; i < 4; i++){
    float k = float(i);
    float cyc = uTime*(0.1 + 0.04*h11(k*3.1)) + h11(k*7.7);
    float n = floor(cyc), q = fract(cyc);
    float a = h11(n*4.3 + k*1.9);
    float y0 = uLength*(0.45 + 0.45*h11(n*2.7 + k));
    float run = smoothstep(0.1, 0.7, q);
    float yRun = y0 - run*uLength*0.25;
    vec3 head = around(0.6, a, yRun);
    vec3 tail = around(0.6, a, mix(y0, yRun, 0.35));
    float d = raySeg(ro, rd, tail, head, s);
    float swell = smoothstep(0.0, 0.1, q)*(1.0 - smoothstep(0.7, 0.76, q));
    acc += (vec3(1.0, 0.3, 0.22)*halo(d, max(0.07 + 0.09*s, px))*2.2 + vec3(0.7, 0.0, 0.03)*halo(d, 0.3)*0.45)*swell*(0.25 + 0.75*s);
    if (q > 0.7){
      float f = (q - 0.7)/0.3;
      vec3 drop = around(0.6 + 0.4*f, a, yRun - 9.0*f*f);
      float dd = rayPt(ro, rd, drop, t);
      acc += (vec3(1.0, 0.4, 0.32)*halo(dd, max(0.16, px))*2.4 + vec3(0.8, 0.0, 0.05)*halo(dd, 0.45)*0.4)*(1.0 - f);
    }
  }
  return acc;
}
// (update 26) the body starts softly in the emitter, no hard end above the hilt
vec4 sig_solid_l(vec4 o, vec2 b, float px){
  vec4 r = sig_solid_raw(o, b, px);
  float j = smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, r.rgb, j), mix(o.a, r.a, j));
}
// (update 29, Creyon: nothing past the tip) every effect ends at the blade's end
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_tc(ro, rd, px);
  float s;
  raySeg(ro, rd, vec3(0.0), vec3(0.0, uLength + 4.0, 0.0), s);
  return fx*tipFade(ro, rd);
}
vec3 sig_glow_34(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(0.950, 0.080, 0.100);
const vec3 SX_DEEP = vec3(0.350, 0.000, 0.030);
const vec3 SX_PALE = vec3(1.000, 0.450, 0.420);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += sxDrops(ro, rd, px, 10.0, 1.3, 0.09, 51.0, SX_HOT, SX_DEEP); a += 1.1*sxStrike(ro, rd, px, 2.0, 0.23, 6.0, 52.0, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_d(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.85*sxFlames(b, 0.85, 2.6, 1.7, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34

// >>> update 42: this blade's great effects in the air
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the tomb of Marka Ragnos: great slabs of obsidian lifting off the ground and turning slowly up past the blade, lit red along their edges,
  // and the old Sith's souls climbing among them as dark red lights
  a += 1.35*gxTumble(ro, rd, px, 9.0, 5.2, 2.0, 0.0, 0.72, 31.0, vec3(1.0, 0.42, 0.26), vec3(0.42, 0.015, 0.03));
  a += 1.2*gxLanterns(ro, rd, px, 5.0, 4.4, 32.0, vec3(1.0, 0.5, 0.34), vec3(0.62, 0.03, 0.05));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.2, 5.0, 0.012, 11.0, vec3(0.34, 0.02, 0.03), vec3(0.03, 0.002, 0.006));
  return base + a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<< update 42
// >>> dim: less glow, so its effects and body can be seen
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){ return sig_glow_d(g, b, r, px)*(0.82 + 0.38*sin(b.y*0.3 - uClock*2.4)); }
vec3 sig_core(vec3 c, vec2 b, float px){ return sig_core_d(c, b, px)*1.0; }
// <<< dim
// >>> life: waves of light running along its body
vec4 sig_solid(vec4 o, vec2 b, float px){
  vec4 r = sig_solid_l(o, b, px);
  if (r.a <= 0.0) return r;
  return vec4(r.rgb*(0.85 + 0.9*pow(0.5 + 0.5*sin(b.y*0.3 - uClock*2.4), 3.0)), r.a);
}
// <<< life
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
