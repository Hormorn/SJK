// Dromund Kaas Vitiate: the hunger of a destroyer that grows stronger with every blow. Its own
// body is dark violet with a magenta edge and a heart that beats, slow when calm and faster and
// brighter the more it is swung and struck; that hunger also grows its aura, a blaze of magenta and
// violet flame round it, darkest at its heart. Crackles of purple lightning run through the aura
// and motes of magenta light rise off it. Every 25 seconds its ego rises: the flames tower high
// and turn from magenta to a white-hot pink, the body cracks with light from within, and a pulse
// of power rolls up it; then it calms. Lit, the aura erupts up the blade from the emitter; let go,
// it sinks back down into it and fades. A clash feeds it: a magenta flare and its heart racing.
// Its trail is the ego (`@trail vitiate`). No figures.
// @part Beating heart
// @part Ego aura
// @part Destroyer's lightning
// @part Lightning surge
// @part Ego rising
// @part Drained life
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail vitiate
// @palette {"corona.rim_cool":[0.4,0.0,0.45],"corona.rim_hot":[1.0,0.2,0.85],"corona.inner":[1.0,0.65,0.95],"core.white":[1.0,0.9,1.0],"core.fringe_cool":[0.6,0.05,0.7],"core.fringe_hot":[1.0,0.45,0.9],"light.color":[0.95,0.2,0.85],"trail":[0.95,0.25,0.85]}
// @speed 2.4

const vec3 UE_MAGENTA = vec3(1.0, 0.18, 0.82);
const vec3 UE_VIOLET = vec3(0.45, 0.08, 0.75);
const vec3 UE_DARK = vec3(0.12, 0.0, 0.18);
const vec3 UE_PINK = vec3(1.0, 0.7, 0.95);
const vec3 UE_WHITE = vec3(1.0, 0.93, 1.0);

float ue_top(){ return (uLength + 0.4)*rxLife(0.0, 0.35, 0.6); }
float ue_after(){ return rxLife(0.2, 1.2, 0.8); }
// the hunger: rising with each swing (it eases in over the swing's first seconds and wears off
// over eight seconds) and a clash, 0 to 1
float ue_hunger(){
  // (update 21, Creyon's video: it reset all at once) the time since a swing jumps back to nothing on
  // each new swing, and all this followed it; only the swing's own smooth strength now
  return clamp(clamp(uSwing, 0.0, 1.0)*0.5 + clamp(uClash, 0.0, 1.0)*0.5, 0.0, 1.0)*ue_after();
}
float ue_rise(){ return part(4.0) > 0.5 ? rxCycle(25.0, 15.0, 18.0, 21.0, 24.0)*ue_after() : 0.0; }
// how long its ego has risen so far (update 16): what its quicker heartbeat adds, so the beat speeds
// up smoothly instead of jumping
float ue_riseI(){ return part(4.0) > 0.5 ? rxCycleInt(25.0, 15.0, 18.0, 21.0, 24.0) : 0.0; }
// the body's half width: heavy with what it has eaten, each heartbeat travelling up it as a swell,
// its edges torn and slowly crawling
float ue_half(float y0, float top){
  float y = clamp(y0, 0.0, top);
  float beat = fract(uClock*0.5);
  float swell = halo(abs(y - beat*(uLength + 10.0) + 5.0), 3.0)*(1.0 - beat*0.6) + 0.5*halo(abs(y - fract(uClock*0.5 - 0.22)*(uLength + 10.0) + 5.0), 2.2);
  float torn = rxFlow(vec2(y*0.7 - uClock*0.5, 2.0));
  return rxHalf(y, top, 0.5*(1.0 + 0.5*swell) + 0.16*smoothstep(0.35, 0.8, torn), 1.0);
}
// the heart's beat: faster with hunger and in the ego rising
float ue_beat(float hunger, float rise){
  // (update 16) its speed adds up as the ego rises; hunger brightens the beat instead of changing
  // its speed (a speed that followed each swing made the heart and the light jump)
  float p = fract(uTime*0.42 + 0.4*ue_riseI());
  return exp(-pow((p - 0.1)/0.06, 2.0)) + 0.6*exp(-pow((p - 0.32)/0.06, 2.0));
}

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = ue_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half = ue_half(b.y, top);
    float d = rxOut(b, top, half);
    float hunger = ue_hunger();
    float rise = ue_rise();
    float beat = ue_beat(hunger, rise);
    col = UE_MAGENTA*halo(d, 0.5 + 0.2*beat)*0.85 + UE_VIOLET*halo(d, 1.5)*0.55 + UE_PINK*halo(d, 0.15)*0.4;
    if (part(1.0) > 0.5 && uFx > 0.0){
      float risen = uAge < 2000.0 ? smoothstep(-2.0, 1.0, (uLength + 8.0)*smoothstep(0.2, 1.4, uAge) - b.y)
                                  : smoothstep(-2.0, 1.0, (uLength + 8.0)*(1.0 - smoothstep(0.0, 0.8, uAge - 2000.0)) - b.y);
      // effect 2: the ego aura, magenta and violet flame, its heart dark, growing with hunger and
      // towering white-pink in the ego rising
      vec3 hot = mix(UE_MAGENTA, UE_PINK, rise);
      vec3 cool = mix(UE_VIOLET, UE_MAGENTA, rise*0.5);
      float reach = 1.5 + 0.8*hunger + 1.5*rise;
      vec3 fl = rxFlames(b, max(d - 0.2, 0.0), top, reach, 3.0, 7.0, hot, cool);
      col += fl*(0.62 + 0.3*hunger)*risen*uFx;
      col += cool*halo(d, reach)*0.07*risen;
      // the pulse of power rolling up it in the ego rising
      float py = mod((uTime*0.4)*8.0, uLength + 10.0) - 3.0;
      col += UE_WHITE*halo(abs(b.y - py), 1.5)*halo(d, 2.0)*rise*0.6;
    }
  }
  col += lvl(g)*UE_MAGENTA*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.5;
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.9*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core_d(vec3 c, vec2 b, float px){
  return c*vec3(1.0, 0.5, 0.9)*0.25*(1.0 - smoothstep(0.3, 2.0, b.y));
}

// effect 1: the beating heart, dark violet with a magenta edge, a brighter heart down its middle
// that beats; in the ego rising it cracks with light from within
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = ue_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = ue_half(b.y, top);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float hunger = ue_hunger();
  float rise = ue_rise();
  // (update 18, Creyon: it looked childish) a smooth run of colour: deep violet at the hand climbing
  // to magenta at the point, white-pink along its heart, softly shaded across
  float along = smoothstep(0.0, 1.0, b.y/max(uLength, 1.0));
  vec3 run = mix(UE_VIOLET*1.15, UE_MAGENTA*0.95, along);
  vec3 col = mix(mix(UE_PINK, run, 0.45), run, smoothstep(0.05, 0.75, u));
  col = mix(col, UE_DARK*2.6, smoothstep(0.72, 0.93, u)*0.55);
  col += mix(UE_MAGENTA, UE_PINK, along)*smoothstep(0.86, 1.0, u)*1.1;
  if (part(0.0) > 0.5){
    float beat = ue_beat(hunger, rise);
    float heart = 1.0 - smoothstep(0.0, 0.45, u);
    col += mix(UE_MAGENTA, UE_PINK, beat)*heart*(0.35 + 0.9*beat)*(0.6 + 0.6*hunger);
    // the veins of hunger: thin magenta threads running up it, brighter with hunger
    float vein = 1.0 - smoothstep(0.0, 0.05, abs(rxFlow(vec2(b.x*4.0, b.y*0.4 - (uTime*0.4)*0.6)) - 0.5));
    col += UE_MAGENTA*vein*(0.3 + 0.8*hunger);
  }
  // the blade's flesh: dark between glowing veins that branch up it and brighten with each beat
  float beat2 = ue_beat(hunger, rise);
  float crack = 1.0 - smoothstep(0.0, 0.04, abs(rxFlow(vec2(b.x*2.6 + 5.0, b.y*0.26 - uClock*0.12)) - 0.5));
  float crack2 = 1.0 - smoothstep(0.0, 0.03, abs(rxFlow(vec2(b.x*5.0 - 2.0, b.y*0.5 + uClock*0.09 + 3.0)) - 0.5));
  col = mix(col, UE_DARK*1.4, 0.55*(1.0 - max(crack, crack2))*smoothstep(0.0, 0.5, u));
  col += mix(UE_MAGENTA, UE_WHITE, rise)*(crack + 0.65*crack2)*(0.75 + 1.1*beat2 + 1.2*rise);
  col += UE_PINK*halo(top - b.y, 0.9)*0.4 + UE_WHITE*halo(max(b.y, 0.0), 0.8)*0.6;
  col = mix(col, UE_WHITE, 0.55*clamp(uClash, 0.0, 1.0));
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// effect 3: the destroyer's lightning (update 16, Creyon: "better lightning, purple, over the whole
// blade, not in slow motion, purple and white, and it must go with the RGB"): forked lightning
// crawling over the whole blade from the emitter right to its point, white-hot in its forks and
// violet in its glow, a steady quick crackle; and longer arcs leaping off it and back, each lit for a
// third of a second. Purple and white only, so a chroma turns the purple to its wearer's colour and
// the white stays white.
vec3 ue_lightning(vec3 ro, vec3 rd, float px, float hunger, float rise){
  vec3 acc = vec3(0.0);
  float L = uLength, f;
  if (fxNear(ro, rd, 0.0, L) > 16.0) return acc;
  float top = ue_top();
  float boost = 1.0 + 0.6*hunger + 0.8*rise;
  acc += 2.2*boost*shBolts(ro, rd, px, 0.75, 0.3, top - 0.1, 4.0, 6.0, 2.708, 0.35, 21.0, UE_WHITE, UE_VIOLET);
  acc += 1.5*boost*shBolts(ro, rd, px, 1.3, 0.6, top - 0.3, 3.0, 9.0, 2.708, 0.55, 27.0, UE_WHITE, UE_VIOLET*1.2);
  for (int i = 0; i < 4; i++){
    float k = float(i);
    float cyc = uTime*(0.75 + 0.25*h11(k*3.1)) + h11(k*5.7);
    float n = floor(cyc), q = fract(cyc);
    if (q > 0.35) continue;
    float life = smoothstep(0.0, 0.03, q)*(1.0 - smoothstep(0.12, 0.35, q));
    float y0 = L*(0.05 + 0.75*h11(n*1.7 + k));
    float a = h11(n*4.3 + k);
    vec3 pa = around(0.6, a, y0);
    vec3 pm = around(1.5 + 0.7*h11(n*2.1 + k), a + 0.04, y0 + 2.5 + 2.0*h11(n*2.9 + k));
    vec3 pc = around(0.6, a + 0.08, min(y0 + 5.0 + 3.0*h11(n*3.7 + k), L - 0.2));
    float d = min(bolt3(ro, rd, pa, pm, n*9.1 + k, 0.3, px), bolt3(ro, rd, pm, pc, n*5.3 + k + 3.0, 0.3, px));
    acc += (UE_WHITE*halo(d, max(0.045, px))*1.6 + UE_VIOLET*halo(d, 0.35)*0.7 + UE_VIOLET*halo(d, 1.2)*0.15)*life*boost;
  }
  return acc;
}
// effect 4, the lightning surge (update 16, the transition Creyon asked for): every three seconds
// a knot of white lightning races up the blade from the emitter to its point in two thirds of a
// second, the bolts round it flaring; behind it the light cools from white back to violet
float ue_front(out float q){
  float c = mod(uClock + uSeed*3.0, 3.2);
  q = c/0.7;
  return q < 1.0 ? ue_top()*q*q*(3.0 - 2.0*q) : -100.0;
}
vec3 ue_surge(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float q;
  float y = ue_front(q);
  if (y < -50.0 || fxNear(ro, rd, max(y - 6.0, 0.0), y + 1.0) > 3.5) return acc;
  for (int i = 0; i < 3; i++){
    float k = float(i);
    float n = floor(uTime*12.0) + k*7.0;
    float a = k/3.0 + h11(n);
    vec3 pa = around(0.5, a, max(y - 4.0 - 2.0*h11(n*2.1), 0.2));
    vec3 pc = around(0.9 + 0.6*h11(n*3.3), a + 0.15, y);
    float d = bolt3(ro, rd, pa, pc, n*1.7 + k, 0.35, px);
    acc += (UE_WHITE*halo(d, max(0.05, px))*2.0 + mix(UE_WHITE, UE_VIOLET, 0.5)*halo(d, 0.4)*0.7);
  }
  float t;
  acc += (UE_WHITE*halo(rayPt(ro, rd, vec3(0.0, y, 0.0), t), 0.7)*1.4 + UE_VIOLET*halo(rayPt(ro, rd, vec3(0.0, y, 0.0), t), 2.2)*0.5);
  return acc*sin(3.1416*clamp(q, 0.0, 1.0))*0.5*(1.0 + smoothstep(0.0, 0.2, q));
}

// effect 8 and the clash: let go, the aura sinking back into the blade as glittering motes;
// a clash, a magenta flare
vec3 ue_moments(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float t;
  if (uAge >= 2000.0 && uAge < 2001.0){
    float q = uAge - 2000.0;
    for (int i = 0; i < 16; i++){
      float k = float(i);
      vec3 p = around((2.5 + 1.5*h11(k*4.1))*(1.0 - q), h11(k*6.3), uLength*h11(k*2.9) - 2.0*q);
      acc += (UE_PINK*halo(rayPt(ro, rd, p, t), max(0.07, px))*1.4)*(1.0 - q)*roomAt(p);
    }
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) acc += (UE_WHITE*halo(rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), t), 1.0)*1.3 + UE_MAGENTA*halo(rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), t), 3.0)*0.5)*cl;
  return acc;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float after = ue_after();
  float hunger = ue_hunger();
  float rise = ue_rise();
  if (part(2.0) > 0.5 && after > 0.01) fx += ue_lightning(ro, rd, px, hunger, rise)*after;
  // effect 4: the lightning surge (update 16, in place of the rising motes Creyon did not like)
  if (part(3.0) > 0.5 && after > 0.01) fx += ue_surge(ro, rd, px)*after;
  fx += ue_moments(ro, rd, px);
  if (part(5.0) > 0.5 && after > 0.01) fx += ue_drain(ro, rd, px)*after;
  // the ego storm (1/11/2026, its own shape): a vast storm of violet and magenta energy round the
  // blade, pushed up from below and widening as it climbs, darker at its heart, swelling with
  // hunger and in the ego rising; sparks of it shoot up through the storm
  if (part(1.0) > 0.5 && after > 0.01 && fxNear(ro, rd, 0.0, uLength) < 9.0){
    fx += (0.6 + 0.45*hunger + 0.7*rise)*syMistSpin(ro, rd, 0.3, uLength + 0.3, 1.5, 2.8, 0.0, 20.0, mix(UE_MAGENTA, UE_PINK, rise), UE_VIOLET*0.6)*after;
    // the ego flames (1/11/2026, Creyon: "the blade feels empty"): sharp, spiky tongues of magenta
    // flame standing up off the whole blade, violet at their tips, taller with hunger and towering
    // in the ego rising, flickering slowly
    for (int i = 0; i < 8; i++){
      float k = float(i);
      float yR = uLength*(0.04 + 0.7*fract(k*0.618 + 0.4));
      float len = min((4.5 + 2.5*h11(k*5.7))*(1.0 + 0.3*hunger + 0.4*rise), uLength + 0.5 - yR);
      float lean = 0.65 + 0.45*h11(k*3.1) + 0.3*hunger;
      fx += 0.85*rxTongue(ro, rd, yR, (k + 0.5)/8.0 - 0.004*tCalm(), len, lean, 0.5 + 0.22*h11(k*2.2), k + 20.0, mix(UE_PINK, UE_WHITE, rise), UE_MAGENTA, UE_VIOLET)*after;
    }
  }
  return fx*uFx;
}


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  return fx*hiltFade(ro, rd);
}

// Drained life (update 20, Creyon: the orbs came from nothing): the air round it gives up its light.
// Sparks kindle faintly in the dark far out, brighten as they are pulled, and race in along a tight
// curve, stretching into streaks, and the blade's veins answer where each goes in.
vec3 ue_drain(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return acc;
  float s;
  for (int i = 0; i < 20; i++){
    float k = float(i);
    float cyc = uClock*(0.22 + 0.1*h11(k*2.7)) + h11(k*8.9);
    float n = floor(cyc), q = fract(cyc);
    float e = q*q*q, e2 = max(pow(max(q - 0.07, 0.0), 3.0), 0.0);
    float a0 = h11(n*3.1 + k), y0 = uLength*(0.08 + 0.84*h11(n*5.9 + k));
    vec3 head = around(mix(6.0, 0.55, e), a0 + 0.5*e, y0 + 2.0*(1.0 - e));
    vec3 tail = around(mix(6.0, 0.55, e2), a0 + 0.5*e2, y0 + 2.0*(1.0 - e2));
    float d = raySeg(ro, rd, tail, head, s);
    float grow = q*q;
    acc += (UE_PINK*halo(d, max(0.03 + 0.04*s, px))*1.5*s + UE_MAGENTA*halo(d, 0.22)*0.45)*grow*(1.0 - smoothstep(0.97, 1.0, q))*roomFade(mix(6.0, 0.55, e));
  }
  return acc;
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){ return vec4(0.0); }
// (update 26) the body starts softly in the emitter, no hard end above the hilt
vec4 sig_solid(vec4 o, vec2 b, float px){
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
const vec3 SX_HOT = vec3(1.000, 0.200, 0.850);
const vec3 SX_DEEP = vec3(0.400, 0.000, 0.450);
const vec3 SX_PALE = vec3(1.000, 0.650, 0.950);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 1.3*sxStrike(ro, rd, px, 2.0, 0.26, 6.5, 91.0, SX_HOT); a += 1.2*xDust(ro, rd, 6.0, 0.9, 1.3, C*0.03, L*0.08, L*0.95, 0.42, 92.0, SX_PALE, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_d(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.6*sxFlames(b, 0.9, 2.0, 1.5, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34

// >>> dim: less glow, so its effects and body can be seen
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){ return sig_glow_d(g, b, r, px)*0.78; }
vec3 sig_core(vec3 c, vec2 b, float px){ return sig_core_d(c, b, px)*1.0; }
// <<< dim

// >>> update 52: this blade's great effects in the air
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  a += 1.5*sxStrike(ro, rd, px, 2.0, 0.42, 7.0, 431.0, vec3(1.0, 0.5, 1.0));
  a += 1.1*liveArcs(ro, rd, px, L*0.08, L*0.95, 3.0, 2.6, 6.5, 432.0, vec3(1.0, 0.42, 1.0));
  a += 0.8*fxFall(ro, rd, px, 1.0, L - 0.8, 3.6, 16.0, 4.6, 1.4, 0.08, 433.0, vec3(0.7, 0.55, 0.85));
  a += 0.9*syMistSpin(ro, rd, 0.3, L*0.97, 1.3, 5.2, 0.06, 434.0, vec3(0.32, 0.03, 0.34), vec3(0.03, 0.003, 0.05));
  return base + a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<< update 52
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
