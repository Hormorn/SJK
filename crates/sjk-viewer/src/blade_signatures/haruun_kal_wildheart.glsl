// Haruun Kal Wildheart: a power that cannot be held back, as wild as Haruun Kal's jungles. Its own
// body is a blazing violet blade, the colour of Vaapad, a white-hot heart in it; round it rages a
// storm of violet energy rising, long violet ribbons sweep round it, violet lightning lashes through it, and the ground's
// rocks torn loose and rising slowly round it in the solid layer, dark and lit green on their
// edges. Every 20 seconds it erupts: the storm swells wide, the lightning doubles, the blade
// blazes white and a great burst of green sparks is thrown out, then it seethes down again. Lit,
// a burst of green light; let go, the storm collapses and the rocks fall. A clash is a green
// flash. Its trail is berserk (`@trail wildheart`). No figures, no rings.
// @part Raging body
// @part Green storm
// @part Lashing lightning
// @part Rising rocks
// @part Eruption
// @part Fireflies
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail wildheart
// @palette {"corona.rim_cool":[0.1,0.5,0.05],"corona.rim_hot":[0.55,1.0,0.3],"corona.inner":[0.85,1.0,0.7],"core.white":[0.95,1.0,0.9],"core.fringe_cool":[0.25,0.7,0.1],"core.fringe_hot":[0.7,1.0,0.45],"light.color":[0.45,1.0,0.25],"trail":[0.4,1.0,0.25]}
// @speed 2.6

const vec3 BK_GREEN = vec3(0.62, 0.25, 1.0);
const vec3 BK_LIME = vec3(0.85, 0.6, 1.0);
const vec3 BK_DEEP = vec3(0.15, 0.03, 0.35);
const vec3 BK_WHITE = vec3(0.96, 0.92, 1.0);

float bk_top(){ return (uLength + 0.4)*rxLife(0.0, 0.3, 0.6); }
float bk_after(){ return rxLife(0.15, 0.9, 0.7); }
float bk_erupt(){ return part(4.0) > 0.5 ? rxCycle(20.0, 12.0, 14.2, 16.2, 19.2)*bk_after() : 0.0; }

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = bk_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half = rxHalf(clamp(b.y, 0.0, top), top, 0.62, 1.0);
    float d = rxOut(b, top, half);
    float er = bk_erupt();
    float rage = rxFlow(vec2(b.x*2.0, b.y*0.5 - tCalm()*3.0));
    col = mix(BK_GREEN, BK_LIME, rage)*halo(d, 0.6 + 0.4*er)*1.0 + BK_DEEP*halo(d, 2.2)*0.6 + BK_WHITE*halo(d, 0.18)*0.6;
  }
  col += lvl(g)*BK_GREEN*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.5;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.9*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*vec3(0.7, 1.0, 0.6)*0.3*(1.0 - smoothstep(0.3, 2.0, b.y)); }

// effect 1: the raging body, green-yellow with a white-hot heart churning in it
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = bk_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = rxHalf(clamp(b.y, 0.0, top), top, 0.62, 1.0);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float churn = part(0.0) > 0.5 ? rxFlow(vec2(b.x*4.0, b.y*0.8 - tCalm()*2.5)) : 0.5;
  vec3 col = mix(BK_WHITE*1.1, BK_LIME, smoothstep(0.1, 0.6, u + 0.3*(churn - 0.5)));
  col = mix(col, BK_GREEN, smoothstep(0.6, 1.0, u));
  col = mix(col, BK_WHITE*1.2, bk_erupt()*0.5);
  col = mix(col, BK_WHITE, 0.5*clamp(uClash, 0.0, 1.0));
  // (update 19) sun through the canopy: patches of green-gold light sliding down the blade
  float dapple = rxFlow(vec2(b.x*1.6 + 0.3*uClock, b.y*0.32 + 0.45*uClock));
  col += vec3(0.65, 0.95, 0.4)*smoothstep(0.6, 0.86, dapple)*0.55;
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// effect 4: rocks torn loose and rising slowly round it, dark with green light on their edges
// (1/11/2026, update 13, Creyon: the black rocks came from nothing and faded into nothing: gone)

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t, f;
  float aft = bk_after();
  float er = bk_erupt();
  if (fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  // effect 2: the green storm, turning and swelling round it
  if (part(1.0) > 0.5){
    fx += (0.7 + 0.6*er)*shSheath(ro, rd, 1.5, 1.0, L - 0.6, 1.8, 0.1, 2.6, 0.46, 0.0, BK_GREEN, BK_DEEP)*aft;
    fx += (0.4 + 0.4*er)*shSheath(ro, rd, 2.6, 1.5, L - 0.6, 1.3, 0.07, 1.8, 0.52, 0.5, BK_LIME, BK_DEEP)*aft;
    for (int ri = 0; ri < 5; ri++){
      float k = float(ri);
      float q = fract(tCalm()*0.09 + h11(k*4.1));
      float y0 = L*(0.06 + 0.6*h11(k*2.2 + floor(tCalm()*0.09 + h11(k*4.1))));
      float rr = 1.9 + 1.4*h11(k*6.3);
      fx += (1.1 + 0.9*er)*sin(3.1416*q)*xDust(ro, rd, rr, rr + 0.6, 0.34, k/5.0 + 0.0056*tCalm(), y0, min(y0 + L*0.22, L*0.95), 0.36, k + 90.0, BK_WHITE, BK_GREEN)*aft;
    }
  }
  // effect 3: green lightning lashing through it, doubled in the eruption
  if (part(2.0) > 0.5){
    // (update 16) a steady crackle: a rate that changed with the moment made the bolts race (`rxCycleInt`)
    fx += (1.2 + 1.2*er)*shBolts(ro, rd, px, 1.0, 1.0, L - 0.3, 3.0, 5.0, 1.6, 0.4, 6.0, BK_WHITE, BK_GREEN)*aft;
    fx += 0.9*shBolts(ro, rd, px, 2.2, 1.5, L - 0.6, 2.0, 8.0, 1.0, 0.8, 13.0, BK_WHITE, BK_GREEN)*aft*(1.0 + 1.0*er);
  }
  // effect 5: the eruption's burst of green sparks
  {
    float c = mod(uClock + uSeed*7.0, 20.0);
    if (part(4.0) > 0.5) fx += rxSparkShower(ro, rd, px, vec3(0.0, L*0.5, 0.0), (c - 12.5)/1.8, 24.0, 7.0, floor((uTime + uSeed*7.0)/20.0), BK_WHITE, BK_GREEN)*aft;
  }
  if (uAge < 1.0) fx += (BK_WHITE*halo(fxNear(ro, rd, 0.0, L*0.9), 1.0)*1.3 + BK_GREEN*halo(fxNear(ro, rd, 0.0, L*0.9), 3.0)*0.5)*exp(-pow((uAge - 0.5)/0.15, 2.0));
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += (BK_WHITE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 1.0)*1.4 + BK_GREEN*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 3.0)*0.6)*cl;
  return fx*uFx;
}


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(5.0) > 0.5) fx += x14_haruun_kal_wildheart(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  return fx*hiltFade(ro, rd);
}

// effect 6 (1/11/2026, update 14): fireflies of the Haruun Kal jungle drawn to the blade's wild
// heart, a swarm of green-gold sparks darting round it, each blinking on and off its own way
vec3 x14_haruun_kal_wildheart(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return acc;
  float t;
  for (int i = 0; i < 14; i++){
    float k = float(i);
    float blink = smoothstep(0.5, 0.9, sin(uTime*(1.1 + 0.6*h11(k*4.1)) + k*5.0));
    if (blink <= 0.0) continue;
    float a = h11(k*3.1) + 0.02*uTime*(h11(k*2.2) - 0.5) + 0.06*sin(uTime*0.9 + k*2.0);
    float r = 1.6 + 3.0*h11(k*5.3) + 0.5*sin(uTime*1.3 + k);
    float y = uLength*(0.1 + 0.85*h11(k*7.9)) + 1.2*sin(uTime*0.7 + k*3.3) + 0.4*sin(uTime*2.3 + k);
    vec3 p = around(r, a, y);
    float d = rayPt(ro, rd, p, t);
    acc += (vec3(0.85, 1.0, 0.45)*halo(d, max(0.07, px))*2.0 + vec3(0.4, 0.9, 0.2)*halo(d, 0.6)*0.35)*blink*roomAt(p);
  }
  return acc;
}
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
const vec3 SX_HOT = vec3(0.550, 1.000, 0.300);
const vec3 SX_DEEP = vec3(0.100, 0.500, 0.050);
const vec3 SX_PALE = vec3(0.850, 1.000, 0.700);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += vec3(0.0);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.55*sxFlames(b, 0.65, 2.0, 1.8, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(5.0) > 0.5){
  a += 1.6*gxKin(ro, rd, 6.0, 6.0, 2.40, 0.100, 0.80, 0.08, 0.96, 1.0, 7416.0, vec3(0.850, 1.000, 0.700), vec3(0.550, 1.000, 0.300));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.39, 4.09, -0.012, 691.0, vec3(0.550, 1.000, 0.300)*0.24, vec3(0.100, 0.500, 0.050)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.550, 1.000, 0.300)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(5.0) > 0.5){
    a += 1.4*vec3(0.0);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  if (part(5.0) > 0.5){
    col += vec3(0.0);
  }
  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.41*gxKin(ro, rd, 1.0, 9.0, 2.80, 0.140, 0.20, 0.08, 0.96, 1.0, 9136.0, vec3(0.850, 1.000, 0.700), vec3(0.550, 1.000, 0.300));
  a += 1.2*gxTumble(ro, rd, px, 11.0, 5.0, 1.0, 1.0, 0.36, 49.0, vec3(0.850, 1.000, 0.700), vec3(0.550, 1.000, 0.300));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

