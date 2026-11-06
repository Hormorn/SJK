// Wobani Projection: a teal blade of the shadow-copy arts. Now and then a soft puff of white smoke
// bursts from it, and as the puff clears a faint copy of the blade stands beside it a moment and
// fades; white smoke ribbons curl off it as it moves, and smoke drifts slowly through its glow.
// No figures.
// @part Smoke puffs
// @part Shadow copy
// @part Smoke ribbons
// @part Drifting smoke
// @raycache fast
// @trail neon
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.0,0.4,0.42],"corona.rim_hot":[0.15,0.85,0.8],"corona.inner":[0.7,1.0,0.96],"core.white":[0.92,1.0,0.98],"core.fringe_cool":[0.05,0.6,0.6],"core.fringe_hot":[0.5,0.95,0.9],"light.color":[0.15,0.8,0.75],"trail":[0.2,0.85,0.8]}
// @speed 2.0

const vec3 CS_TEAL = vec3(0.12, 0.82, 0.76);
const vec3 CS_SMOKE = vec3(0.86, 0.92, 0.92);
const vec3 CS_DEEP = vec3(0.0, 0.35, 0.36);

float cs_flow(vec2 p){
  float v = sin(p.x + 1.5*sin(p.y*0.9 + 0.2));
  v += 0.8*sin(p.y*1.3 + 1.3*sin(p.x*1.2 + 1.6));
  v += 0.5*sin((p.x + p.y)*1.8 + 1.0*sin(p.x*2.0 - p.y*0.7 + 2.4));
  return 0.5 + v/4.6;
}
// The puff on a 9-second cycle: how far it has gone (0 to 1, below 0 when none), and the copy's
// strength after it (0 to 1, smooth both ways). The copy stands on one side, chosen per puff.
float cs_puff(out float copy, out float side){
  float c = mod(uClock + uSeed*7.0, 9.0);
  float n = floor((uClock + uSeed*7.0)/9.0);
  side = h11(n*3.1) > 0.5 ? 1.0 : -1.0;
  copy = part(1.0) > 0.5 ? 0.35 + 0.65*smoothstep(0.5, 1.4, c)*(1.0 - smoothstep(2.4, 6.0, c)) : 0.0;
  return (part(0.0) > 0.5 && c < 2.2) ? c/2.2 : -1.0;
}

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  vec3 col = recol(g, CS_TEAL)*0.85;
  float top = uLength;
  float y = clamp(b.y, 0.0, top);
  float d = length(vec2(b.x, b.y - y));
  // effect 4: drifting smoke, pale smoke drifting slowly up through its glow
  if (part(3.0) > 0.5 && uFx > 0.0){
    float sm = cs_flow(vec2(b.x*1.2 + 0.4*sin((uTime*0.4)*0.3), b.y*0.18 - (uTime*0.4)*0.5));
    col += CS_SMOKE*smoothstep(0.55, 0.9, sm)*halo(d, 1.6)*0.25*uFx;
  }
  // effect 3: smoke ribbons, white ribbons curling off the blade's sides and up, longer as it
  // is swung
  if (part(2.0) > 0.5 && uFx > 0.0 && b.y > 0.5){
    float sw = smoothstep(0.1, 0.8, clamp(uSwing, 0.0, 1.0));
    float rib = abs(sin(b.y*0.5 - d*0.9 + 2.5*cs_flow(vec2(d*0.5, b.y*0.1 - (uTime*0.4)*0.4)) - (uTime*0.4)*0.8 + clamp(b.x*2.0, -1.0, 1.0)*1.3));
    rib = 1.0 - smoothstep(0.0, 0.3, rib);
    float reach = 1.4 + 2.8*sw;
    col += CS_SMOKE*rib*(1.0 - smoothstep(reach*0.3, reach, d))*smoothstep(0.1, 0.5, d)*0.45*(1.0 - smoothstep(top - 1.0, top + 2.0, b.y))*uFx;
  }
  // effect 2: the shadow copy, a faint teal copy of the blade standing beside it after a puff
  float copy, side;
  cs_puff(copy, side);
  if (copy > 0.0){
    // (update 20) two copies stand off it, swaying out of step, thin as projections and flickering
    // in bands as they climb
    for (int ci = 0; ci < 2; ci++){
      float s = ci == 0 ? 1.0 : -1.0;
      float dx = b.x - s*(1.5 + 0.35*sin(uClock*0.8 + float(ci)*2.0 + b.y*0.15));
      float dd = length(vec2(dx, b.y - y));
      float flick = 0.65 + 0.35*sin(b.y*1.1 - uClock*3.0 + float(ci)*2.0);
      col += (CS_TEAL*halo(dd, 0.35)*0.5 + CS_SMOKE*halo(dd, 0.09)*0.4)*copy*flick*smoothstep(0.0, 2.5, b.y);
    }
  }
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }

// effect 1: the smoke puffs, a soft round cloud of white smoke bursting out of the blade's middle
// and swelling, thinning as it spreads (several soft balls of smoke round where it burst)
// The 3D structure round it (1/11/2026, Creyon: the lower tiers filled out so they are never
// plain, each its own shape): the drifting smoke grown thick: a soft cloud of white smoke rolling slowly up round the blade. Lit, it grows in; let go, it fades.
vec3 cx_more(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  float aft = rxLife(0.2, 1.0, 0.6);
  if (part(3.0) < 0.5 || aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  {

    fx += 0.0*L;
  }
  return fx*aft;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float copy, side;
  float q = cs_puff(copy, side);
  if (q < 0.0) return fx + cx_more(ro, rd, px)*uFx;
  float L = uLength, t;
  vec3 at = vec3(0.0, L*0.5, 0.0);
  if (rayPtRaw(ro, rd, at, t) > 9.0) return fx + cx_more(ro, rd, px)*uFx;
  float e = 1.0 - (1.0 - q)*(1.0 - q);
  for (int i = 0; i < 9; i++){
    float k = float(i);
    vec3 p = at + around((0.4 + 2.6*h11(k*3.3))*e, h11(k*5.1), (h11(k*7.7) - 0.5)*L*0.7*(0.4 + 0.6*e) + 1.2*q);
    float size = (0.6 + 1.6*e)*(0.7 + 0.3*h11(k*1.9));
    float d = rayPtRaw(ro, rd, p, t);
    float puff = halo(d, size)*(1.0 - q)*smoothstep(0.0, 0.08, q);
    fx += (CS_SMOKE*0.55 + CS_TEAL*0.12)*puff*roomAt(p);
  }
  return (fx*uFx) + cx_more(ro, rd, px)*uFx;
}

// ---- update 13: its floating layer (tools/sw_extras.py) ----
const vec3 XM = vec3(0.15, 0.8, 0.75);
const vec3 XH = vec3(0.703, 0.93, 0.912);
const vec3 XA = vec3(0.15, 0.85, 0.8);
const vec3 XD = vec3(0.045, 0.24, 0.225);
vec3 sw_extra(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  fx += 1.0*rxStarCloud(ro, rd, px, 20.0, 3.4, 1.0, uLength*0.95, 0.06, 141.0, XH, XA);
  fx += 0.9*shStreaks(ro, rd, 1.8, 1.5, uLength - 0.6, 16.0, 2.4, 3.5, 0.3, 142.0, XH, XA);
  fx += 1.0*shStrands(ro, rd, 1.25, 1.0, uLength - 0.6, 3.0, 0.02, 0.03, 0.14, 2.5, 143.0, XH, XM);
  return fx;
}

vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(1.0) > 0.5) fx += sw_extra(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  return fx*hiltFade(ro, rd);
}
// tip_cap (update 29, Creyon: every tip solid, never see-through): the top of the blade closes into
// an opaque, rounded end in the blade's own colours; light drifts slowly through it. The rest of the
// blade stays as it was.
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength, top = L + 0.2;
  float fade = smoothstep(L - 6.5, L - 2.6, b.y);
  if (fade <= 0.0 || b.y > top) return o;
  float e0 = 1.0 - clamp((top - b.y)/1.08, 0.0, 1.0);
  float half = 0.72*sqrt(max(1.0 - e0*e0, 0.0));
  float cov = fillD(abs(b.x) - half, px)*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(abs(b.x)/max(half, 0.01), 0.0, 1.0);
  float sw = 0.5 + 0.5*sin(b.x*3.4 + b.y*0.55 - uClock*0.8 + 1.6*sin(b.y*0.21 - uClock*0.3));
  vec3 col = mix(vec3(0.88, 1.00, 1.00), vec3(0.42, 0.92, 0.95), smoothstep(0.25, 1.0, u));
  col = mix(col, vec3(0.08, 0.56, 0.62), 0.4*sw*sw*smoothstep(0.1, 0.9, u));
  float a = cov*fade;
  float keep = max(o.a, a);
  return vec4(mix(o.rgb, col, a/max(keep, 0.001)), keep);
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
const vec3 SX_HOT = vec3(0.150, 0.850, 0.800);
const vec3 SX_DEEP = vec3(0.000, 0.400, 0.420);
const vec3 SX_PALE = vec3(0.700, 1.000, 0.960);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 7.0, 4.0, 2.80, 0.040, 0.45, 0.08, 0.96, 1.0, 8235.0, SX_PALE, SX_HOT);a += 0.9*xPuffs(ro, rd, 2.4, 1.3, 1.0, C*0.02, L*0.06, L*0.96, 0.46, 351.0, SX_PALE, SX_HOT*0.5);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.1*shStreaks(ro, rd, 3.38, L*0.05, L*0.95, 9.0, 3.0, 1.34, 0.4, 832.0, vec3(0.700, 1.000, 0.960), vec3(0.150, 0.850, 0.800));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.48, 4.59, -0.012, 835.0, vec3(0.150, 0.850, 0.800)*0.24, vec3(0.000, 0.400, 0.420)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.150, 0.850, 0.800)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(3.0) > 0.5){
    a += 1.3*syPetalsAir(ro, rd, 8.0, 4.16, 0.036, 1592.0, vec3(0.700, 1.000, 0.960), vec3(0.150, 0.850, 0.800));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  if (part(3.0) > 0.5){
    col += vec3(0.0);
  }
  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  a += 1.1*gxFlares(ro, rd, px, 4.0, 3.8, 72.0, vec3(0.700, 1.000, 0.960), vec3(0.150, 0.850, 0.800));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

