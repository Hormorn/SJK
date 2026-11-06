// Bendu Equilibrium: the body moving on its own, beyond thought, the Force in perfect balance
// as the Bendu knew it. Its own body is silver-white light so clear it seems hollow, a cool blue
// heart running down it and a slow rainbow sheen sliding over its surface; round it rises a silver
// steam, smooth curls of pale light drifting up off its edges and fading; a calm cloud of tiny
// silver motes hangs round it, drifting; a heat-haze shimmer clings to it. Every
// 24 seconds it moves on its own: its afterimages step out to either side and back, the silver
// steam blazes up, the comets quicken and the sheen runs bright, then it is calm again. Lit, the
// silver steam rushes up it from the emitter; let go, the steam rises away and it fades. A clash is
// a soundless white flash. Its trail is the will (`@trail equilibrium`). No figures.
// @part Silver body
// @part Silver steam
// @part Floating light
// @part Haze shimmer
// @part Moving on its own
// @part Two currents
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail equilibrium
// @palette {"corona.rim_cool":[0.45,0.55,0.75],"corona.rim_hot":[0.8,0.9,1.0],"corona.inner":[0.96,0.98,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.6,0.7,0.9],"core.fringe_hot":[0.88,0.94,1.0],"light.color":[0.8,0.88,1.0],"trail":[0.85,0.9,1.0]}
// @speed 4.5

const vec3 BW_SILVER = vec3(0.86, 0.9, 0.98);
const vec3 BW_BLUE = vec3(0.45, 0.65, 1.0);
const vec3 BW_WHITE = vec3(1.0, 1.0, 1.0);

float bw_top(){ return (uLength + 0.4)*rxLife(0.0, 0.35, 0.6); }
float bw_after(){ return rxLife(0.2, 1.1, 0.7); }
float bw_moving(){ return part(4.0) > 0.5 ? rxCycle(24.0, 14.0, 16.0, 19.0, 22.0)*bw_after() : 0.0; }
// how many seconds it has moved on its own so far (update 16): what its quicker steam and sheen
// add, so they speed up smoothly instead of racing (Creyon's video)
float bw_movingI(){ return part(4.0) > 0.5 ? rxCycleInt(24.0, 14.0, 16.0, 19.0, 22.0) : 0.0; }

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = bw_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half = rxHalf(clamp(b.y, 0.0, top), top, 0.48, 1.0);
    float d = rxOut(b, top, half);
    float mv = bw_moving();
    float after = bw_after();
    col = BW_SILVER*halo(d, 0.55)*0.8 + BW_BLUE*halo(d, 1.4)*0.35 + BW_WHITE*halo(d, 0.12)*0.5;
    float risen = uAge < 2000.0 ? smoothstep(-2.0, 1.0, (uLength + 6.0)*smoothstep(0.2, 1.2, uAge) - b.y) : 1.0;
    // effect 2: silver steam, smooth curls of pale light drifting up off its edges and fading
    if (part(1.0) > 0.5 && uFx > 0.0){
      vec2 q = vec2(b.x*0.9 + clamp(b.x*2.0, -1.0, 1.0)*0.5*sin(b.y*0.2 - uTime*0.15), b.y*0.22 - uTime*0.2 - 0.2*bw_movingI());
      float curl = rxFlow(q + 0.8*vec2(rxFlow(q*0.6 + 3.0), rxFlow(q*0.6 + 7.0)));
      float reach = 1.4 + 1.5*curl + 1.5*mv;
      float steam = smoothstep(0.5, 0.85, curl)*(1.0 - smoothstep(0.2*reach, reach, d))*smoothstep(0.05, 0.4, d);
      col += (BW_SILVER*0.55 + BW_BLUE*0.15)*steam*(0.8 + 0.8*mv)*after*risen*(1.0 - smoothstep(top, top + 2.5, b.y))*uFx;
    }
    // effect 4: a heat-haze shimmer clinging to it
    if (part(3.0) > 0.5 && uFx > 0.0){
      float w = rxFlow(vec2(b.x*3.5 + uTime*0.1, b.y*0.6 - uTime*0.35));
      col += BW_WHITE*pow(w, 7.0)*halo(d, 1.0)*smoothstep(0.1, 0.4, d)*0.35*after*uFx;
    }
    // effect 5: moving on its own, its afterimages stepping out to either side and back
    if (mv > 0.01 && uFx > 0.0){
      float off = 1.6*mv*(0.6 + 0.4*sin(uClock*1.4));
      float yy = clamp(b.y, 0.0, top);
      float dl = length(vec2(b.x + off, b.y - yy));
      float dr = length(vec2(b.x - off, b.y - yy));
      col += (BW_SILVER*(halo(dl, 0.35) + halo(dr, 0.35))*0.45 + BW_BLUE*(halo(dl, 1.0) + halo(dr, 1.0))*0.12)*mv*uFx;
    }
  }
  col += lvl(g)*BW_SILVER*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.5;
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.9*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core_d(vec3 c, vec2 b, float px){ return c*0.3*(1.0 - smoothstep(0.3, 2.0, b.y)); }

// effect 1: the silver body, so clear it seems hollow: bright edges, a cool blue heart down its
// middle and a slow rainbow sheen sliding over it, running bright when it moves on its own
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = bw_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = rxHalf(clamp(b.y, 0.0, top), top, 0.48, 1.0);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float mv = bw_moving();
  vec3 col = mix(BW_BLUE*0.7, BW_SILVER*1.1, smoothstep(0.2, 0.95, u));
  if (part(0.0) > 0.5){
    float sheen = fract(b.y*0.04 - uTime*0.02 - 0.08*bw_movingI() + u*0.3);
    vec3 rainbow = hsv(sheen, 0.35, 1.0);
    float band = pow(0.5 + 0.5*sin(b.y*0.25 - uTime*0.25 - 0.6*bw_movingI()), 4.0);
    col = mix(col, rainbow, band*(0.3 + 0.4*mv));
  }
  // (update 18, Creyon: more beautiful) a fine white seam down the middle and both edges lit, a slow
  // run of light climbing the seam
  col += BW_WHITE*halo(u, 0.14)*(0.35 + 0.5*pow(0.5 + 0.5*sin(b.y*0.3 - uTime*0.55), 5.0));
  col += BW_WHITE*smoothstep(0.84, 1.0, u)*0.55;
  col += BW_WHITE*halo(max(b.y, 0.0), 0.8)*0.6;
  col = mix(col, BW_WHITE, 0.6*clamp(uClash, 0.0, 1.0));
  float a = cov*mix(mix(0.75, 0.92, u), 1.0, smoothstep(uLength - 6.0, uLength - 3.0, b.y));
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// (1/11/2026, update 12, Creyon: its speed was too fast: its ribbons now live about 33 seconds
// and drift round half as fast)
// effect 3: floating light (1/11/2026, replacing the orbiting comets: Creyon found them spinning
// far too fast): a calm cloud of tiny silver-white motes hanging round the blade, drifting
// slowly, twinkling; when it moves on its own they brighten.
vec3 bw_motes(vec3 ro, vec3 rd, float px, float mv){
  return rxStarCloud(ro, rd, px, 34.0, 3.8, 0.8, uLength*0.92, 0.05, 41.0, BW_WHITE, BW_BLUE)*(0.8 + 0.8*mv);
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float after = bw_after();
  float mv = bw_moving();
  float t;
  if (part(2.0) > 0.5 && after > 0.01) fx += bw_motes(ro, rd, px, mv)*after;
  // the balance: flakes of white light sink past flakes of deep blue that rise
  if (part(3.0) > 0.5 && after > 0.01 && fxNear(ro, rd, 0.0, uLength) < 6.0){
    fx += (0.9 + 0.5*mv)*syPetalsAir(ro, rd, 8.0, 4.0, 0.045, 71.0, BW_WHITE, BW_SILVER)*after;
    fx += (0.9 + 0.5*mv)*syPetalsAir(ro, rd, 8.0, 4.0, -0.045, 72.0, BW_SILVER, BW_BLUE*1.2)*after;
  }
  // effect 8: let go, the silver steam rising away
  if (uAge >= 2000.0 && uAge < 2001.4) fx += rxBorn(ro, rd, px, 16.0, 0.5, uLength, 1.2, 1.0, 5.0, 0.07, 3.0, BW_WHITE, BW_SILVER)*(1.0 - (uAge - 2000.0)/1.4);
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += BW_WHITE*halo(rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), t), 1.4)*cl*1.5;
  // the afterimages (1/11/2026, its own shape): bright streaks of silver light sweeping round the
  // blade where it has moved on its own, each a short slanted comet trail, coming and going; more
  // and brighter when it moves
  if (part(1.0) > 0.5 && after > 0.01 && fxNear(ro, rd, 0.0, uLength) < 9.0){
    // the silver steam (1/11/2026): slim wisps of silver light rising straight up off it and
    // fading, as the steam of a body moving beyond thought
    for (int i = 0; i < 12; i++){
      float k = float(i);
      float yR = uLength*(0.04 + 0.72*fract(k*0.618 + 0.05));
      float len = min(6.0 + 3.5*h11(k*5.7), uLength + 0.5 - yR);
      fx += (1.0 + 0.6*mv)*rxTongue(ro, rd, yR, (k + 0.5)/12.0 + 0.0016*tCalm(), len, 0.7 + 0.5*h11(k*3.1), 0.45 + 0.2*h11(k*2.2), k + 120.0, BW_WHITE, BW_SILVER, BW_BLUE)*after;
    }
    for (int i = 0; i < 5; i++){
      float k = float(i);
      float q = fract(tCalm()*0.055 + h11(k*4.9));
      float life = sin(3.1416*q);
      float y0 = uLength*(0.06 + 0.6*h11(k*2.3 + floor(tCalm()*0.055 + h11(k*4.9))));
      float r = 2.4 + 1.6*h11(k*6.1);
      fx += (1.2 + 0.8*mv)*life*rxRibbon(ro, rd, r, r + 0.6, 0.32, k/5.0 + 0.0032*tCalm(), y0, min(y0 + uLength*0.22, uLength*0.95), 0.45, k + 120.0, BW_WHITE, BW_SILVER)*after;
    }
    fx += BW_BLUE*halo(fxNear(ro, rd, 0.0, uLength*0.7), 1.6)*roomFade(fxNear(ro, rd, 0.0, uLength*0.7))*0.25*after;
  }
  return fx*uFx;
}


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  return fx*hiltFade(ro, rd);
}

// effect 6 (1/11/2026, update 14): balanced stones, the Bendu's balance between light and dark: two
// smooth river stones, one pale silver and one dark slate, hover on either side of the blade,
// each lit at its rim by the other's light, rising and falling against each other slowly as a
// scale does, never quite still
vec4 x14_bendu_equilibrium(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  // (update 15, Creyon: "may be more, as long as it looks good") six stones, pale and dark in
  // turn, up the whole blade, each pair rising and falling against each other as a scale does,
  // the whole set turning slowly round the blade
  for (int i = 0; i < 6; i++){
    float k = float(i);
    float pale = mod(k, 2.0) < 0.5 ? 1.0 : 0.0;
    float pair = floor(k*0.5);
    float tilt = 0.7*sin(uClock*0.5 + pair*2.1)*(pale > 0.5 ? 1.0 : -1.0);
    vec3 c = around(3.0 + 0.5*h11(k*3.3), uClock*0.03 + uSeed + k*0.37 + 0.5*pale, uLength*(0.22 + 0.25*pair + 0.06*pale) + tilt);
    vec3 n;
    float R = 0.75 + 0.3*h11(k*5.9);
    float cov = xBall(ro, rd, c, R, 0.72, px, n);
    if (cov <= 0.0) continue;
    vec3 l = normalize(vec3(-c.x, 0.7, -c.z));
    float di = (pale > 0.5 ? 0.65 : 0.3) + 0.5*max(dot(n, l), 0.0);
    float rim = pow(1.0 - abs(dot(n, rd)), 3.0);
    vec3 base = pale > 0.5 ? vec3(1.0, 1.02, 1.1) : vec3(0.24, 0.26, 0.31);
    vec3 col = base*di + (pale > 0.5 ? vec3(0.15, 0.2, 0.35) : vec3(0.85, 0.9, 1.0))*rim*0.9;
    o = vec4(mix(o.rgb, col, cov), max(o.a, cov));
  }
  return o;
}
vec4 sig_vol_solid_34(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  return o;
}
// (update 26) the body starts softly in the emitter, no hard end above the hilt
vec4 sig_solid_e3(vec4 o, vec2 b, float px){
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
const vec3 SX_HOT = vec3(0.800, 0.900, 1.000);
const vec3 SX_DEEP = vec3(0.450, 0.550, 0.750);
const vec3 SX_PALE = vec3(0.960, 0.980, 1.000);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  if (part(5.0) > 0.5) a += vec3(0.0);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_d(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (sxFlames(b, 0.85, 2.4, 1.0, vec3(1.0), SX_PALE*0.5)*step(0.0, b.x)*0.75)*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = sig_vol_solid_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  float L = uLength, C = uClock;
  vec4 n = (vec4(0.0))*rxLife(0.45, 1.3, 0.4)*uFx*hiltFade(ro, rd);
  return vec4(mix(o.rgb, n.rgb, n.a), max(o.a, n.a));
}
// <<< update 34

// >>> dim: less glow, so its effects and body can be seen
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){ return sig_glow_d(g, b, r, px)*0.45; }
vec3 sig_core(vec3 c, vec2 b, float px){ return sig_core_d(c, b, px)*0.55; }
// <<< dim

// >>> update 52: this blade's great effects in the air
vec3 sig_vol_e3(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 prevT = uTurnFx;
  vec3 base = sig_vol_p1(ro, rd, px)*0.5;
  uTurnFx = prevT + (uTurnFx - prevT)*0.5;
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  a += 1.15*syMistSpin(ro, rd, 0.3, L*0.97, 1.3, 5.0, 0.06, 421.0, vec3(0.4, 0.46, 0.55), vec3(0.05, 0.06, 0.1));
  for (int i = 0; i < 8; i++){
    float k = float(i);
    float run = C*(0.26 + 0.08*h11(k + 423.0)) + k*0.29, cyc = floor(run), ph = fract(run);
    float y = L*((k + h11(k*2.3 + cyc*1.3))/8.0)*0.86 + 0.3;
    a += 1.0*sin(3.1416*ph)*syLick(ro, rd, px, y, h11(k*5.7 + cyc*0.7 + 423.0) + 0.08*ph, 1.0 + 1.2*h11(k*3.1 + cyc), 4.5 + 3.5*h11(k*4.3 + cyc), 0.55, k*3.0 + cyc + 423.0, vec3(1.0, 1.0, 1.0), vec3(0.62, 0.72, 0.9), vec3(0.1, 0.14, 0.3));
  }
  a += 1.15*xBeads(ro, rd, 2.8, 3.2, 1.2, C*0.08, L*0.05, L*0.95, 0.08, 424.0, vec3(1.0), vec3(0.6, 0.72, 1.0));
  a += 0.9*xBeads(ro, rd, 3.2, 2.8, -1.2, C*0.08 + 0.5, L*0.05, L*0.95, 0.07, 425.0, vec3(0.5, 0.55, 0.7), vec3(0.14, 0.18, 0.35));
  return base + a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<< update 52
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e3(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// Ink in still water: white on one side of it, black on the other, slowly changing places, always in balance.
vec4 sig_solid(vec4 o, vec2 b, float px){
  if (part(4.0) > 0.5 && uFx > 0.0 && b.y > 0.0 && b.y < uLength){
    float d = abs(b.x), swap = 0.5 + 0.5*sin(uClock*0.09);
    float light = b.x > 0.0 ? swap : 1.0 - swap;
    float reach = (2.6 + 2.0*rxFlow(vec2(b.y*0.1 - uClock*0.05, b.x > 0.0 ? 2.0 : 6.0)))*smoothstep(0.0, uLength*0.2, b.y)*(1.0 - smoothstep(uLength*0.74, uLength, b.y));
    vec2 q = vec2(d*0.55 - uClock*0.05, b.y*0.2 + 0.03*sin(uClock*0.2));
    float n = rxFlow(q + 0.9*(rxFlow(q*1.3 + 4.0) - 0.5));
    float a = (1.0 - smoothstep(reach*0.2, reach, d))*smoothstep(0.32, 0.72, n)*0.82*rxLife(0.4, 1.6, 0.3);
    if (a > 0.003){
      vec3 ink = mix(vec3(0.012, 0.014, 0.02), vec3(0.82, 0.86, 0.94), light*smoothstep(0.35, 0.8, n));
      o = vec4(mix(o.rgb, ink, a), max(o.a, a));
    }
  }
  return sig_solid_e3(o, b, px);
}

// >>> what this blade has in the air of its own
vec3 sig_vol_m26(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.25*gxKin(ro, rd, 3.0, 6.0, 3.20, 0.040, 0.20, 0.08, 0.96, -1.0, 9068.0, BW_WHITE, BW_BLUE*0.8);
  a += 1.05*gxHelix(ro, rd, px, 16.0, 2.4, 2.0, 0.035, 207.0, vec3(0.960, 0.980, 1.000), vec3(0.800, 0.900, 1.000));
  a += 1.3*gxBinary(ro, rd, px, 4.0, 0.7, vec3(1.00, 0.98, 0.92), vec3(0.12, 0.20, 0.50), vec3(0.800, 0.900, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(60.0, 22.0, 32.0, 38.0, 48.0), L = uLength;
  // the two bodies part of their dance only in the wide; here all the small lights come to rest near the blade, and it goes still
  a += m*1.3*gxBinary(ro, rd, px, 1.4 + 2.2*(1.0 - m), 1109.0, vec3(1.0, 0.98, 0.92), vec3(0.12, 0.2, 0.5), vec3(0.800, 0.900, 1.000));
  return base*(1.0 - 0.55*m) + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

