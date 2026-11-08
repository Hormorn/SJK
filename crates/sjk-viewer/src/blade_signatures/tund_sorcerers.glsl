// Tund Sorcerers: the sorcery of the Sorcerers of Tund, masters of illusion. Its own body is pale
// gold light with a calm violet flame licking along its outline; round it sweep long ribbons of
// violet and gold sorcery, each rising, curving round the blade and fading slowly, and violet
// motes of light hang in the air in a soft violet haze. Every 26 seconds the sorcery gathers: the
// ribbons draw in close round the blade and it blazes gold, then they drift out again. Lit, gold
// light rises up it from the hand; let go, it fades. A clash is a gold flash. Its trail is the
// sorcery (`@trail sorcery`). No figures, no rings, no symbols, no orbs (1/11/2026, Creyon).
// @part Gold body
// @part Sorcery ribbons
// @part Violet flame
// @part Violet motes
// @part The gathering
// @part Threads of fate
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail sorcery
// @palette {"corona.rim_cool":[0.32,0.12,0.5],"corona.rim_hot":[0.85,0.6,1.0],"corona.inner":[1.0,0.92,0.75],"core.white":[1.0,0.98,0.9],"core.fringe_cool":[0.6,0.3,0.85],"core.fringe_hot":[1.0,0.85,0.55],"light.color":[0.8,0.5,1.0],"trail":[0.85,0.55,1.0]}
// @speed 4.5

const vec3 SP_GOLD = vec3(0.72, 0.35, 1.0);
const vec3 SP_SUN = vec3(1.0, 0.8, 0.35);
const vec3 SP_PALE = vec3(1.0, 0.94, 0.75);
const vec3 SP_WHITE = vec3(1.0, 0.98, 0.92);
const vec3 SP_BLACK = vec3(0.02, 0.02, 0.025);

float sp_top(){ return (uLength + 0.4)*rxLife(0.0, 0.4, 0.7); }
float sp_after(){ return rxLife(0.2, 1.2, 0.7); }
float sp_full(){
  float c = mod(uClock + uSeed*7.0, 26.0);
  return part(4.0) > 0.5 ? smoothstep(15.0, 18.0, c)*(1.0 - smoothstep(21.0, 24.0, c)) : 0.0;
}

// the mirage (update 19): the Sorcerers' craft is illusion, so the blade will not hold still to the
// eye: a slow wave passes along it and bends its picture sideways, and every few seconds it lies
// harder. Only its picture: where it strikes never moves.
vec2 sp_mirage(vec2 b){
  float lie = 0.55 + 0.45*smoothstep(0.55, 0.95, sin(uClock*0.7));
  float env = smoothstep(1.0, 6.0, b.y)*(1.0 - smoothstep(uLength - 5.0, uLength, b.y));
  return vec2(b.x + env*lie*(0.26*sin(b.y*0.3 - uClock*1.1) + 0.09*sin(b.y*0.7 + uClock*1.6)), b.y);
}
vec3 sig_glow_34(vec3 g, vec2 b0, float r, float px){
  vec2 b = sp_mirage(b0);
  float top = sp_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half = rxHalf(clamp(b.y, 0.0, top), top, 0.58, 1.0);
    float d = rxOut(b, top, half);
    float full = sp_full();
    col = SP_GOLD*halo(d, 0.55 + 0.4*full)*0.95 + SP_PALE*halo(d, 0.18)*0.6 + SP_GOLD*halo(d, 1.8)*0.25*(1.0 + full);
    // effect 3: a calm golden flame licking along its outline
    if (part(2.0) > 0.5 && uFx > 0.0) col += rxFlames(b, d, top, 1.4 + 0.8*full, 1.4, 7.0, SP_PALE, SP_GOLD)*0.9*sp_after()*uFx;
  }
  col += lvl(g)*SP_GOLD*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.5;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*vec3(1.0, 0.9, 0.6)*0.3*(1.0 - smoothstep(0.3, 2.0, b.y)); }

// effect 1: the sage body, white-gold light with a slow glow rising in it
vec4 sig_solid_raw(vec4 o, vec2 b0, float px){
  vec2 b = sp_mirage(b0);
  float top = sp_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = rxHalf(clamp(b.y, 0.0, top), top, 0.58, 1.0);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  vec3 col = mix(SP_WHITE*1.1, SP_GOLD, smoothstep(0.3, 1.0, u));
  if (part(0.0) > 0.5) col *= 0.9 + 0.2*rxFlow(vec2(b.x*3.0, b.y*0.3 - tCalm()*0.8));
  col = mix(col, SP_WHITE*1.2, sp_full()*0.4);
  col = mix(col, SP_WHITE, 0.5*clamp(uClash, 0.0, 1.0));
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t;
  float aft = sp_after();
  float full = sp_full();
  if (fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  // effect 2: sorcery ribbons, long sweeps of violet and gold light rising and curving round the
  // blade, each living about twenty seconds; gathering in close at the moment
  if (part(1.0) > 0.5){
    for (int i = 0; i < 6; i++){
      float k = float(i);
      float q = fract(tCalm()*0.08 + h11(k*4.9));
      float life = sin(3.1416*q);
      float y0 = L*(0.06 + 0.6*h11(k*2.3 + floor(tCalm()*0.08 + h11(k*4.9))));
      float r = mix(2.2 + 1.6*h11(k*6.1), 1.0, full);
      vec3 hot = mod(k, 2.0) < 0.5 ? SP_WHITE : SP_PALE;
      vec3 col = mod(k, 2.0) < 0.5 ? SP_GOLD : SP_SUN;
      fx += (1.1 + 0.9*full)*life*rxRibbon(ro, rd, r, r + 0.7, 0.34, k/6.0 + 0.0048*tCalm(), y0, min(y0 + L*0.24, L*0.95), 0.42, k + 60.0, hot, col)*aft;
    }
  }
  // effect 4: motes of sage light and a soft golden haze
  if (part(3.0) > 0.5){
    fx += rxStarCloud(ro, rd, px, 28.0, 3.8, 0.8, L*0.95, 0.055, 9.0, SP_WHITE, SP_GOLD)*(0.9 + 0.6*full)*aft;
    fx += (0.5 + 0.6*full)*syMist(ro, rd, 0.5, L*0.95, 4.2, 0.12, 9.0, SP_GOLD, SP_GOLD*0.3)*aft;
  }
  // moments: lit, gold light rising up it; let go, a fading gold glow; a clash, a gold flash
  if (uAge < 1.4){
    float y = L*smoothstep(0.0, 1.0, uAge);
    fx += (SP_WHITE*halo(rayPt(ro, rd, vec3(0.0, y, 0.0), t), 0.8)*1.3 + SP_GOLD*halo(rayPt(ro, rd, vec3(0.0, y, 0.0), t), 2.5)*0.4)*(1.0 - smoothstep(1.0, 1.4, uAge));
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += (SP_WHITE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 1.0)*1.3 + SP_GOLD*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 3.0)*0.5)*cl;
  return fx*uFx;
}


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(5.0) > 0.5) fx += x14_tund_sorcerers(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  return fx*hiltFade(ro, rd);
}

// effect 6 (1/11/2026, update 14): threads of fate, the Sorcerers of Tund's craft: fine threads of
// gold light strung between knots of light that hang round the blade, a cat's cradle slowly
// shifting as the knots drift, pulses of violet running along the threads
vec3 x14_tund_sorcerers(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return acc;
  float s, t;
  vec3 prev = around(2.4, uTime*0.01, uLength*0.22);
  for (int i = 1; i <= 7; i++){
    float k = float(i);
    vec3 p = around(2.0 + 1.0*sin(uTime*0.13 + k*1.7), k*0.385 + uTime*0.01, uLength*(0.22 + 0.1*k) + 0.6*sin(uTime*0.2 + k));
    float d = raySeg(ro, rd, prev, p, s);
    float pulse = pow(0.5 + 0.5*sin(s*10.0 - uTime*1.5 + k), 12.0);
    float ends = smoothstep(0.0, 0.15, s)*(1.0 - smoothstep(0.85, 1.0, s));
    acc += vec3(1.0, 0.85, 0.45)*halo(d, max(0.02, px))*(0.5 + 0.3*ends) + vec3(0.75, 0.4, 1.0)*halo(d, 0.08 + px)*pulse*1.2 + vec3(0.65, 0.3, 1.0)*halo(d, 0.2)*0.08;
    acc += vec3(1.0, 0.9, 0.6)*halo(rayPt(ro, rd, p, t), max(0.07, px))*1.6;
    prev = p;
  }
  return acc*roomFade(3.0);
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

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(0.850, 0.600, 1.000);
const vec3 SX_DEEP = vec3(0.320, 0.120, 0.500);
const vec3 SX_PALE = vec3(1.000, 0.920, 0.750);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.35*sxFlames(b, 0.7, 1.8, 1.0, SX_HOT, SX_DEEP*0.5) + 0.8*sySmoke(b, 3.6, 0.32, 216.0, SX_PALE))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(5.0) > 0.5){
  a += 1.6*gxKin(ro, rd, 3.0, 7.0, 3.00, 0.060, 0.20, 0.08, 0.96, -1.0, 8144.0, vec3(1.000, 0.920, 0.750), vec3(0.850, 0.600, 1.000));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.44, 4.82, -0.015, 651.0, vec3(0.850, 0.600, 1.000)*0.24, vec3(0.320, 0.120, 0.500)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.850, 0.600, 1.000)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(5.0) > 0.5){
    a += 1.3*vec3(0.0);
    a += 1.3*vec3(0.0);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.37*gxKin(ro, rd, 5.0, 4.0, 3.00, 0.030, 0.70, 0.08, 0.96, 1.0, 9374.0, vec3(1.000, 0.920, 0.750), vec3(0.850, 0.600, 1.000));
  a += 1.2*gxSwarm(ro, rd, px, 10.0, 4.4, 47.0, vec3(1.000, 0.920, 0.750), vec3(0.850, 0.600, 1.000));
  a += 1.0*gxFlares(ro, rd, px, 4.0, 4.6, 48.0, vec3(1.000, 0.920, 0.750), vec3(0.850, 0.600, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

