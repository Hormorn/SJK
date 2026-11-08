// Vulptex Crystalfire (rebuilt 2/11/2026, update 22; Creyon: no tails, no lanterns, and no little crystal
// needles): the vulptex, the crystal foxes of Crait, and the spirit fire in them. The blade is a long
// shard of ember-red crystal, cut in large clean facets, fire moving inside it. Round it burns soft
// foxfire: wide smooth veils of gold and rose flame that rise along it, fold over each other and
// thin away, with no hard edge anywhere. Embers rise out of it and red crystal dust glitters. Every 16
// seconds the fox's howl: a surge climbs the blade, the foxfire stands tall and the fire inside blazes.
// @part Crystal body
// @part Foxfire
// @part Embers
// @part Inner fire
// @part Crystal dust
// @part The howl
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail foxfire
// @palette {"corona.rim_cool":[0.6,0.05,0.02],"corona.rim_hot":[1.0,0.42,0.08],"corona.inner":[1.0,0.78,0.42],"core.white":[1.0,0.93,0.8],"core.fringe_cool":[1.0,0.3,0.04],"core.fringe_hot":[1.0,0.65,0.2],"light.color":[1.0,0.42,0.1],"trail":[1.0,0.38,0.06]}
// @speed 1.0

const vec3 VC_RED = vec3(0.92, 0.1, 0.05);
const vec3 VC_ORANGE = vec3(1.0, 0.42, 0.08);
const vec3 VC_GOLD = vec3(1.0, 0.8, 0.35);
const vec3 VC_DEEP = vec3(0.22, 0.01, 0.015);
const vec3 VC_WHITE = vec3(1.0, 0.95, 0.85);

float vc_top(){ return (uLength + 0.3)*rxLife(0.0, 0.45, 0.5); }
float vc_after(){ return rxLife(0.3, 1.2, 0.5); }
// the howl's front: 0 to 1 up the blade, and how strong it is at y
float vc_howl(float y){
  if (part(5.0) < 0.5) return 0.0;
  float c = mod(uClock + uSeed*7.0, 16.0);
  float front = (uLength + 8.0)*smoothstep(9.0, 11.5, c) - 4.0;
  return smoothstep(front + 4.0, front - 2.0, y)*(1.0 - smoothstep(12.5, 15.5, c))*vc_after();
}
float vc_half(float y0, float top){
  float y = clamp(y0, 0.0, top);
  return rxHalf(y, top, 0.5, 1.0)*(0.9 + 0.1*sin(y*2.1));
}
// the foxfire at b, d out from the edge: warped currents rising, eating into a body of light that thins
// with distance, so veils form, fold and burn out; soft all the way (light only, nothing solid)
vec3 vc_foxfire(vec2 b, float d, float top){
  if (part(1.0) < 0.5 || d > 5.5 || b.y < 0.2) return vec3(0.0);
  float side = b.x < 0.0 ? 2.7 : 0.0;
  float h = vc_howl(b.y);
  vec2 p = vec2(side + d*0.42, b.y*0.17 - uClock*0.62 - d*0.14);
  vec2 w = vec2(rxFlow(p*1.2 + 11.0), rxFlow(p*1.5 + 29.0)) - 0.5;
  float v = 0.7*rxFlow(p + w*1.5) + 0.34*rxFlow(p*2.1 + w*2.2 + 7.0);
  float come = smoothstep(0.25, 0.7, rxFlow(vec2(b.y*0.08 - uClock*0.2, side*1.3 + uClock*0.09)));
  float reach = (1.1 + 2.5*come)*(1.0 + 0.8*h)*vc_after();
  float f = v - 0.16 - d/max(reach, 0.05)*0.68;
  float a = smoothstep(-0.04, 0.3, f);
  // long licks inside the veil, so it reads as flame and never as a flat patch of colour
  vec2 lp = vec2(side*2.0 + d*1.5 + w.x*1.2, b.y*0.13 - uClock*0.95 - d*0.1);
  float lick = rxFlow(lp + w*0.9);
  float lick2 = rxFlow(lp*2.2 + 3.0);
  float strand = smoothstep(0.34, 0.86, lick)*(0.55 + 0.6*lick2);
  a *= 0.14 + 0.95*strand;
  float s = strand*smoothstep(0.0, 0.3, f);
  vec3 col = mix(vec3(0.7, 0.05, 0.03), vec3(1.0, 0.24, 0.05), smoothstep(0.0, 0.4, s));
  col = mix(col, vec3(1.0, 0.56, 0.12), smoothstep(0.32, 0.72, s));
  col = mix(col, VC_GOLD*1.15, smoothstep(0.68, 1.0, s));
  float span = smoothstep(0.2, 3.5, b.y)*(1.0 - smoothstep(top - 3.5, top + 0.2, b.y));
  return col*a*span*(1.05 + 0.55*h);
}
vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = vc_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float yc = clamp(b.y, 0.0, top);
    float d = max(length(vec2(b.x, b.y - yc)) - vc_half(yc, top), 0.0);
    float h = vc_howl(b.y);
    col = VC_ORANGE*(halo(d, 0.5)*0.26 + halo(d, 1.4)*(0.05 + 0.2*h));
    col += vc_foxfire(b, d, top);
  }
  col += lvl(g)*VC_ORANGE*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.3;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.6*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = vc_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = vc_half(b.y, top);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.04))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float across = clamp(b.x/max(half, 0.01), -1.0, 1.0);
  // facets: long flat planes of the shard, each catching the light its own way
  vec2 cell;
  float edge = voro(vec2(across*1.3 + 5.0, b.y*0.34), cell);
  float face = h21(cell);
  float h = vc_howl(b.y);
  vec3 col = part(0.0) > 0.5 ? mix(VC_DEEP*1.6, VC_RED*0.95, 0.2 + 0.8*face) : VC_RED*0.8;
  // the fire moving inside it, seen through the facets
  if (part(3.0) > 0.5){
    vec2 q = vec2(across*1.6 + face*2.0, b.y*0.32 - uClock*1.1);
    vec2 w = vec2(rxFlow(q*1.3 + 3.0), rxFlow(q*1.7 + 8.0)) - 0.5;
    float fire = 0.65*rxFlow(q + w*1.8) + 0.4*rxFlow(q*2.3 + w*2.6 + 5.0);
    col += mix(VC_ORANGE, VC_GOLD, smoothstep(0.55, 0.9, fire))*smoothstep(0.42, 0.82, fire)*(0.8 + 1.2*h);
  }
  // the bright lines where the facets meet, and a hard glint on one plane at a time
  col += VC_GOLD*(1.0 - smoothstep(0.0, 0.05, edge))*(0.45 + 0.9*h);
  col += VC_WHITE*pow(0.5 + 0.5*sin(face*19.0 + uClock*0.9), 22.0)*0.65;
  col += VC_WHITE*smoothstep(0.86, 1.0, abs(across))*0.35;
  col += VC_WHITE*halo(max(b.y, 0.0), 0.5)*0.4;
  col = mix(col, VC_WHITE, 0.5*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t;
  float aft = vc_after();
  // embers torn off the tips: bright, rising, curling away and dying
  if (part(2.0) > 0.5 && fxNear(ro, rd, 0.0, L + 3.0) < 6.5){
    for (int i = 0; i < 26; i++){
      float k = float(i);
      float cyc = uClock*(0.2 + 0.12*h11(k*2.3)) + h11(k*7.1);
      float n = floor(cyc), q = fract(cyc);
      vec3 p = around(0.9 + 3.0*sqrt(q)*h11(n*4.1 + k), h11(n*3.7 + k) + 0.18*q + 0.03*sin(q*9.0 + k), L*(0.06 + 0.86*h11(n*5.1 + k)) + 5.0*q);
      float d = rayPt(ro, rd, p, t);
      float size = mix(0.13, 0.03, q);
      vec3 c = mix(VC_GOLD*1.2, VC_RED, smoothstep(0.2, 0.9, q));
      fx += (c*halo(d, max(size, px))*1.8 + c*halo(d, size*3.5)*0.3)*smoothstep(0.0, 0.06, q)*(1.0 - smoothstep(0.75, 1.0, q))*roomAt(p);
    }
  }
  // red crystal dust, each grain flashing as it turns
  if (part(4.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 6.0) fx += vec3(0.0)*aft;
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += (VC_WHITE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 0.9)*1.2 + VC_ORANGE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 2.8)*0.35)*cl;
  return fx*uFx*hiltFade(ro, rd);
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
const vec3 SX_HOT = vec3(1.000, 0.420, 0.080);
const vec3 SX_DEEP = vec3(0.600, 0.050, 0.020);
const vec3 SX_PALE = vec3(1.000, 0.780, 0.420);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 1.2*shFlakes(ro, rd, 3.2, 0.6, L - 0.5, 16.0, 2.8, 0.35, 0.12, 0.12, 0.3, 0.55, 201.0, vec3(1.0, 0.92, 0.7), SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.6*sxFlames(b, 0.7, 2.4, 1.5, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34

// >>> update 52: this blade's great effects in the air
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  for (int i = 0; i < 8; i++){
    float k = float(i);
    float run = C*(0.42 + 0.08*h11(k + 521.0)) + k*0.29, cyc = floor(run), ph = fract(run);
    float y = L*((k + h11(k*2.3 + cyc*1.3))/8.0)*0.86 + 0.3;
    a += 1.0*sin(3.1416*ph)*syLick(ro, rd, px, y, h11(k*5.7 + cyc*0.7 + 521.0) + 0.08*ph, 0.9 + 1.2*h11(k*3.1 + cyc), 4.0 + 3.5*h11(k*4.3 + cyc), 0.72, k*3.0 + cyc + 521.0, vec3(1.0, 0.86, 0.45), vec3(1.0, 0.36, 0.06), vec3(0.45, 0.05, 0.01));
  }
  for (int i = 0; i < 6; i++){
    float k = float(i);
    a += 1.5*rxComet(ro, rd, px, 2.2 + 0.55*k, L*(0.06 + 0.04*k), L*0.78, 4.2 + 0.8*k, k*3.3 + 522.0, vec3(1.0, 0.9, 0.6), vec3(1.0, 0.42, 0.08));
  }
  a += 1.3*xDust(ro, rd, 1.3, 3.8, 1.6, C*0.14, L*0.03, L*0.97, 0.85, 523.0, vec3(1.0, 0.85, 0.5), vec3(1.0, 0.35, 0.05));
  return base + a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<< update 52
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
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
vec3 sig_vol_m26(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.43*gxKin(ro, rd, 1.0, 8.0, 2.20, 0.120, 0.16, 0.08, 0.96, 1.0, 9425.0, vec3(1.000, 0.780, 0.420), vec3(1.000, 0.420, 0.080));
  a += 0.9*gxInfinity(ro, rd, px, 14.0, 3.0, 2.0, vec3(1.000, 0.780, 0.420), vec3(1.000, 0.420, 0.080));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(50.0, 18.0, 26.0, 32.0, 41.0), L = uLength;
  a += 1.4*gxField(ro, rd, px, 13.0, m, 1.0, 4.8, 0.5, -1.0, 1.0, 0.34, 1101.0, vec3(1.000, 0.780, 0.420), vec3(1.000, 0.420, 0.080));
  a += m*1.1*gxInfinity(ro, rd, px, 18.0, 2.0 + 2.6*m, 1102.0, vec3(1.000, 0.780, 0.420), vec3(1.000, 0.420, 0.080));
  return base*1.0 + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

