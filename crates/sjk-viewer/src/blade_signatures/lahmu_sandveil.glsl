// Lah'mu Sandveil: a blade of living sand that guards its bearer. Its own body is golden sand
// packed hard by the Force, its grains streaming slowly upward through it with a glint here and
// there; streams of sand spiral up round it grain by grain, a fine haze of dust hangs in its
// glow, and sand trickles off its edges and falls. Every 22 seconds the coffin closes: the sand
// round it rises in a whirling vortex, wraps the blade in a thick shell of darker sand that
// tightens and grinds, then bursts outward in a cloud of grains that drifts down and settles. Lit,
// sand pours up out of the emitter and packs itself into the blade; let go, it collapses and pours
// away in falling sand. A clash throws a spray of sand. Its trail is sand (`@trail sand`). No
// figures.
// @part Packed sand
// @part Sand streams
// @part Falling sand
// @part The coffin
// @part Dust haze
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail sand
// @palette {"corona.rim_cool":[0.55,0.32,0.05],"corona.rim_hot":[1.0,0.75,0.3],"corona.inner":[1.0,0.92,0.7],"core.white":[1.0,0.97,0.88],"core.fringe_cool":[0.8,0.5,0.15],"core.fringe_hot":[1.0,0.82,0.45],"light.color":[1.0,0.7,0.3],"trail":[1.0,0.72,0.32]}
// @speed 3.6

const vec3 SC_GOLD = vec3(1.0, 0.72, 0.3);
const vec3 SC_PALE = vec3(1.0, 0.9, 0.65);
const vec3 SC_DARK = vec3(0.42, 0.26, 0.08);
const vec3 SC_WHITE = vec3(1.0, 0.97, 0.88);

float sc_top(){ return (uLength + 0.4)*rxLife(0.0, 0.4, 1.0); }
float sc_after(){ return rxLife(0.5, 1.0, 0.6); }
// the coffin on its 22-second cycle: the vortex rising (0 to 1), the shell closed round the
// blade (0 to 1) and the burst (0 to 1 over three seconds, below 0 when none)
float sc_coffin(out float vortex, out float shell){
  vortex = 0.0; shell = 0.0;
  if (part(3.0) < 0.5) return -1.0;
  float on = sc_after();
  vortex = rxCycle(22.0, 11.0, 13.5, 16.0, 19.0)*on;
  shell = rxCycle(22.0, 13.0, 15.2, 17.8, 20.8)*on;
  return on > 0.5 ? rxMoment(22.0, 17.6, 3.0) : -1.0;
}
// how long the vortex has turned so far (update 16): what its faster spin adds, so the sand
// speeds up smoothly instead of racing (Creyon: an effect suddenly went too fast)
float sc_vortexI(){ return part(3.0) > 0.5 ? rxCycleInt(22.0, 11.0, 13.5, 16.0, 17.0) : 0.0; }
float sc_half(float y, float top, float shell){
  float base = rxHalf(y, top, 0.62, 1.0);
  // the shell: a thick, slowly churning layer of sand over it
  float churn = 0.12*rxFlow(vec2(y*0.6 - (uTime*0.4)*1.5, 2.0));
  return base + shell*(0.35 + churn)*sqrt(clamp((top - y)/3.0, 0.0, 1.0));
}

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = sc_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float vortex, shell;
    float burst = sc_coffin(vortex, shell);
    float half = sc_half(clamp(b.y, 0.0, top), top, shell);
    float d = rxOut(b, top, half);
    col = SC_GOLD*halo(d, 0.5)*0.7 + SC_DARK*halo(d, 1.3)*0.5 + SC_PALE*halo(d, 0.12)*0.35;
    // effect 5: dust haze, a fine drifting haze of golden dust round it, thick in the vortex
    if (part(4.0) > 0.5 && uFx > 0.0){
      float dust = rxFlow(vec2(b.x*1.3 + (uTime*0.4)*0.25, b.y*0.2 - (uTime*0.4)*0.3));
      col += SC_GOLD*smoothstep(0.45, 0.9, dust)*halo(d, 1.8 + 2.0*vortex)*(0.25 + 0.4*vortex)*sc_after()*uFx;
    }
    if (burst >= 0.0) col += SC_PALE*halo(d, 1.0 + 6.0*burst)*(1.0 - burst)*(1.0 - burst)*0.5;
  }
  col += lvl(g)*SC_GOLD*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.5;
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.7*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){
  return c*vec3(1.0, 0.8, 0.5)*0.25*(1.0 - smoothstep(0.3, 2.0, b.y));
}

// effects 1 and 4: the packed sand, grains streaming slowly up through it with glints, its
// rim darker; the coffin's shell darker still, churning and grinding over it
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = sc_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float vortex, shell;
  float burst = sc_coffin(vortex, shell);
  float y = clamp(b.y, 0.0, top);
  float half = sc_half(y, top, shell);
  float inner = rxHalf(y, top, 0.62, 1.0);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(inner, 0.01), 0.0, 1.5);
  vec3 col = mix(SC_PALE, SC_GOLD, smoothstep(0.1, 0.9, u));
  if (part(0.0) > 0.5){
    // grains: fine ripples of light and shade streaming up (smooth fields at a fine scale)
    // (1/11/2026, Creyon: "the blade looks bad"): sand carved by the wind, soft ripples of light
    // and shade lying across it like a dune's, drifting slowly up it, a warm glowing heart, and
    // only here and there a single glint
    float warp = rxFlow(vec2(b.x*1.5, b.y*0.15 - tCalm()*0.1));
    float dune = 0.5 + 0.5*sin(b.y*1.6 - tCalm()*0.9 + b.x*2.0 + 3.0*warp);
    dune = smoothstep(0.2, 1.0, dune);
    col = mix(col*0.8, col*1.15, dune);
    col += SC_WHITE*(1.0 - smoothstep(0.0, 0.45, u))*0.35;
    float gl = rxFlow(vec2(b.x*9.0 + 4.0, b.y*2.5 - tCalm()*0.6));
    col += SC_WHITE*pow(gl, 14.0)*3.0;
  }
  // the shell over it: darker grinding sand
  if (x > inner){
    float grind = rxFlow(vec2(b.x*8.0 + (uTime*0.4)*2.0*clamp(b.x*2.0, -1.0, 1.0), b.y*2.0 - (uTime*0.4)*0.8));
    col = mix(SC_DARK, SC_GOLD*0.8, grind*0.6);
  }
  col += SC_WHITE*halo(max(b.y, 0.0), 0.8)*0.5;
  col = mix(col, SC_WHITE, 0.4*clamp(uClash, 0.0, 1.0));
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// effect 2: sand streams, grains travelling up three slow spirals round the blade (each grain a
// soft glinting point), whirling faster and wider in the vortex
vec3 sc_streams(vec3 ro, vec3 rd, float px, float vortex){
  vec3 acc = vec3(0.0);
  float L = uLength, t;
  if (fxNear(ro, rd, 0.0, L) > 4.5 + 2.0*vortex) return acc;
  for (int i = 0; i < 36; i++){
    float k = float(i);
    float strand = mod(k, 3.0);
    float q = fract(uTime*0.028 + 0.032*sc_vortexI() + k/36.0*3.0 + strand*0.13);
    // (update 16, Creyon: grains went through the blade) the spirals keep 1.4 units off the blade
    float R = 1.75 + 0.35*sin(q*6.0 + strand) + 1.5*vortex*(1.0 - q);
    vec3 p = around(R, strand/3.0 + q*(1.8 + 1.5*vortex) + (uTime*0.4)*0.05, L*(0.02 + 0.95*q));
    float d = rayPt(ro, rd, p, t);
    float tw = 0.6 + 0.4*sin((uTime*0.4)*5.0 + k*2.3);
    acc += (SC_PALE*halo(d, max(0.08, px))*1.6*tw + SC_GOLD*halo(d, 0.35)*0.5)*smoothstep(0.0, 0.08, q)*(1.0 - smoothstep(0.85, 1.0, q))*roomAt(p);
  }
  return acc;
}

// effect 3: falling sand, grains trickling off its edges and falling away
vec3 sc_fall(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float L = uLength, t;
  if (fxNear(ro, rd, -6.0, L) > 3.0) return acc;
  for (int i = 0; i < 16; i++){
    float k = float(i);
    float q = fract((uTime*0.4)*(0.3 + 0.15*h11(k*2.3)) + h11(k*7.1));
    float n = floor((uTime*0.4)*(0.3 + 0.15*h11(k*2.3)) + h11(k*7.1));
    vec3 p = around(0.9 + 0.8*q, h11(n*3.7 + k), L*(0.15 + 0.8*h11(n*5.1 + k)) - 7.0*q*q);
    float d = rayPt(ro, rd, p, t);
    acc += (SC_GOLD*halo(d, max(0.045, px))*1.2)*smoothstep(0.0, 0.1, q)*(1.0 - q)*roomAt(p);
  }
  return acc;
}

// effects 5, 7 and 8: the coffin's burst (a cloud of grains thrown out and drifting down); lit,
// sand pouring up out of the emitter; let go, the blade pouring away
vec3 sc_moments(vec3 ro, vec3 rd, float px, float burst){
  vec3 acc = vec3(0.0);
  float L = uLength, t;
  if (burst >= 0.0){
    float e = 1.0 - (1.0 - burst)*(1.0 - burst);
    for (int i = 0; i < 26; i++){
      float k = float(i);
      vec3 p = around(0.8 + 5.0*e*(0.5 + 0.5*h11(k*4.1)), h11(k*6.3), L*(0.05 + 0.9*h11(k*2.9)) + 1.5*e - 4.0*burst*burst);
      acc += (SC_PALE*halo(rayPt(ro, rd, p, t), max(0.07, px))*1.3 + SC_GOLD*halo(rayPt(ro, rd, p, t), 0.35)*0.35)*(1.0 - burst)*roomAt(p);
    }
  }
  if (uAge < 1.2){
    for (int i = 0; i < 14; i++){
      float k = float(i);
      float q = clamp(uAge/1.0 - h11(k)*0.3, 0.0, 1.0);
      vec3 p = around(0.3 + 0.5*h11(k*3.1), h11(k*5.3) + q, (uLength + 0.4)*q);
      acc += SC_PALE*halo(rayPt(ro, rd, p, t), max(0.06, px))*1.4*sin(3.1416*q);
    }
  } else if (uAge >= 2000.0 && uAge < 2002.0){
    float q = clamp((uAge - 2000.0)/1.8, 0.0, 1.0);
    for (int i = 0; i < 20; i++){
      float k = float(i);
      vec3 p = around(0.4 + 0.8*q*h11(k*4.1), h11(k*6.3), L*h11(k*2.9) - 8.0*q*q);
      acc += SC_GOLD*halo(rayPt(ro, rd, p, t), max(0.06, px))*1.3*(1.0 - q)*roomAt(p);
    }
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01){
    vec3 at = vec3(0.0, L*0.6, 0.0);
    for (int i = 0; i < 12; i++){
      float k = float(i);
      vec3 p = at + around(0.5 + 3.0*(1.0 - cl), k/12.0, -2.0*(1.0 - cl)*(1.0 - cl) + (h11(k) - 0.5));
      acc += SC_PALE*halo(rayPt(ro, rd, p, t), 0.08)*1.4*cl;
    }
  }
  return acc;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float after = sc_after();
  float vortex, shell;
  float burst = sc_coffin(vortex, shell);
  if (part(1.0) > 0.5 && after > 0.01) fx += sc_streams(ro, rd, px, vortex)*after;
  if (part(2.0) > 0.5 && after > 0.01) fx += sc_fall(ro, rd, px)*after;
  fx += sc_moments(ro, rd, px, burst);
  // the sandstorm (1/11/2026, its own shape): a wide storm of golden sand turning round the blade and
  // widening as it climbs, streams of finer sand racing up through it; wilder in the vortex
  if (part(1.0) > 0.5 && after > 0.01 && fxNear(ro, rd, 0.0, uLength) < 9.0){
    fx += (1.7 + 0.8*vortex)*rxMistSpinP(ro, rd, 0.3, uLength*0.95, 3.5, 5.8, 0.0125*uTime + 0.03*sc_vortexI(), 100.0, SC_GOLD, SC_DARK*0.6)*after;
    for (int i = 0; i < 6; i++){
      float k = float(i);
      fx += 0.9*xBeads(ro, rd, 1.0, 4.2, 0.25, k/6.0 + 0.012*(tCalm() + 2.0*sc_vortexI()), uLength*0.03 + k*0.5, uLength*0.9, 0.35, k + 100.0, SC_PALE, SC_GOLD)*after;
    }
  }
  // the hourglass (1/11/2026): thin veils of sand pouring down round the blade from high on it
  // and spilling away below, slow and steady, as through an hourglass's neck
  if (part(2.0) > 0.5 && after > 0.01 && fxNear(ro, rd, 0.0, uLength) < 9.0){
    float f;
    for (int i = 0; i < 5; i++){
      float k = float(i);
      float a = k/5.0 + 0.0032*tCalm();
      float r0 = 1.0 + 0.6*h11(k*3.3);
      float y0 = uLength*(0.55 + 0.35*h11(k*5.1));
      vec3 pa = around(r0, a, y0);
      vec3 pc = around(r0 + 1.6 + 0.8*h11(k*2.2), a + 0.03, 0.8);
      float sr;
      float d = raySeg(ro, rd, pa, pc, sr);
      float grain = rxFlow(vec2(sr*30.0 - tCalm()*3.0, k*4.0));
      float veil = (1.0 - smoothstep(0.03, 0.35, d))*(0.5 + 0.8*grain)*smoothstep(0.0, 0.1, sr)*(1.0 - smoothstep(0.75, 1.0, sr));
      fx += (SC_PALE*veil*0.55 + SC_GOLD*halo(d, 0.5)*0.15)*roomAt(pc)*after;
    }
  }
  return fx*uFx;
}


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  return fx*hiltFade(ro, rd);
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
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(1.000, 0.750, 0.300);
const vec3 SX_DEEP = vec3(0.550, 0.320, 0.050);
const vec3 SX_PALE = vec3(1.000, 0.920, 0.700);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 0.9*syMistSpin(ro, rd, L*0.05, L*0.98, 1.2, 4.6, 0.03, 206.0, SX_HOT*0.32, SX_DEEP*0.3);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  float L = uLength, C = uClock;
  vec4 n = (sxShards(ro, rd, px, 5.0, 3.6, 0.95, 0.03, 207.0, vec3(0.13, 0.09, 0.04), SX_PALE))*rxLife(0.45, 1.3, 0.4)*uFx*hiltFade(ro, rd);
  return vec4(mix(o.rgb, n.rgb, n.a), max(o.a, n.a));
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(4.0) > 0.5){
  a += 1.1*shStreaks(ro, rd, 3.35, L*0.05, L*0.95, 11.7, 3.0, 1.24, 0.4, 557.0, vec3(1.000, 0.920, 0.700), vec3(1.000, 0.750, 0.300));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.28, 4.55, 0.014, 560.0, vec3(1.000, 0.750, 0.300)*0.24, vec3(0.550, 0.320, 0.050)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.750, 0.300)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  if (part(4.0) > 0.5){
    col += vec3(0.0);
    col += vec3(0.0);
  }
  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.29*gxKin(ro, rd, 0.0, 13.0, 3.20, 0.120, 0.28, 0.08, 0.96, 1.0, 9204.0, vec3(1.000, 0.920, 0.700), vec3(1.000, 0.750, 0.300));
  a += 1.2*gxGale(ro, rd, px, 15.0, 5.4, 37.0, vec3(1.000, 0.920, 0.700), vec3(1.000, 0.750, 0.300));
  a += 1.0*gxTumble(ro, rd, px, 7.0, 4.8, 1.0, 1.0, 0.24, 38.0, vec3(1.000, 0.920, 0.700), vec3(1.000, 0.750, 0.300));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

