// Sullust Magma: the molten rivers of Sullust shaped into a blade. Its body is dark crusted
// lava with bright magma flowing up through the gaps in its crust; bubbles of magma swell and
// burst on its surface; drops of lava spatter off it and fall glowing; heat glows round it. No
// figures.
// @part Lava flow
// @part Magma bubbles
// @part Spatter
// @part Heat glow
// @raycache fast
// @trail cinder
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.5,0.05,0.0],"corona.rim_hot":[1.0,0.45,0.05],"corona.inner":[1.0,0.8,0.4],"core.white":[1.0,0.92,0.75],"core.fringe_cool":[0.75,0.15,0.0],"core.fringe_hot":[1.0,0.6,0.15],"light.color":[1.0,0.4,0.05],"trail":[1.0,0.4,0.05]}
// @speed 2.0

const vec3 SL_CRUST = vec3(0.12, 0.04, 0.02);
const vec3 SL_MAGMA = vec3(1.0, 0.45, 0.05);
const vec3 SL_HOT = vec3(1.0, 0.85, 0.4);

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float half = rxHalf(clamp(b.y, 0.0, L), L, 0.6, 1.0);
  float d = rxOut(b, L, half);
  vec3 col = recol(g, SL_MAGMA)*0.5;
  if (part(3.0) > 0.5) col += SL_MAGMA*halo(d, 0.9)*(0.4 + 0.1*sin((uTime*0.4)*0.8)) + vec3(0.6, 0.08, 0.0)*halo(d, 2.2)*0.25;
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.4; }

// effects 1 and 2: crusted lava with magma flowing up through the gaps, and bubbles swelling and
// bursting on it (round, each kept inside its own cell)
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.2 || b.y > L) return o;
  float half = rxHalf(clamp(b.y, 0.0, L), L, 0.6, 1.0);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  vec3 col = SL_CRUST;
  if (part(0.0) > 0.5){
    float f = rxFlow(vec2(b.x*3.0, b.y*0.5 - (uTime*0.4)*0.6));
    float flow = smoothstep(0.45, 0.62, f);
    col = mix(SL_CRUST, mix(SL_MAGMA, SL_HOT, smoothstep(0.62, 0.8, f)), flow);
  }
  if (part(1.0) > 0.5){
    vec2 gq = vec2(b.x*2.4, b.y*0.9 - (uTime*0.4)*0.4);
    vec2 c0 = floor(gq);
    vec2 ctr = c0 + 0.3 + 0.4*h22(c0);
    float life = fract((uTime*0.4)*0.35 + h21(c0));
    float rad = 0.05 + 0.2*life;
    float dist = length(gq - ctr);
    float shown = step(0.55, h21(c0 + 4.0));
    col += SL_HOT*halo(dist, 0.5*rad)*shown*(0.6 + 1.5*smoothstep(0.85, 1.0, life))*(1.0 - smoothstep(0.95, 1.0, life));
  }
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// effect 3: spatter, drops of lava flung off and falling, glowing as they cool
// The 3D structure round it (1/11/2026, Creyon: the lower tiers filled out so they are never
// plain, each its own shape): the heat: a red haze of heat and smoke rising fast off the blade, and drops of magma spat out of it. Lit, it grows in; let go, it fades.
vec3 cx_more(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  float aft = rxLife(0.2, 1.0, 0.6);
  if (part(3.0) < 0.5 || aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  {

    fx += 1.6*syMist(ro, rd, 0.5, L*0.95, 4.2, 0.7, 18.0, SL_MAGMA*0.7, SL_CRUST + SL_MAGMA*0.15);
    fx += rxBorn(ro, rd, px, 18.0, 0.5, L*0.9, 0.5, 3.0, 2.0, 0.07, 18.0, SL_HOT, SL_MAGMA);
  }
  return fx*aft;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0 || part(2.0) < 0.5) return fx;
  float L = uLength, t;
  if (fxNear(ro, rd, -4.0, L) > 4.0) return fx + cx_more(ro, rd, px)*uFx;
  for (int i = 0; i < 12; i++){
    float k = float(i);
    float cyc = (uTime*0.4)*(0.35 + 0.2*h11(k*2.3)) + h11(k*7.1);
    float n = floor(cyc), q = fract(cyc);
    vec3 p = around(0.6 + 1.8*q, h11(n*3.7 + k), L*(0.2 + 0.75*h11(n*5.1 + k)) + 2.0*q - 6.0*q*q);
    vec3 c = mix(SL_HOT, vec3(0.6, 0.08, 0.0), q);
    fx += (c*halo(rayPt(ro, rd, p, t), max(0.07, px))*1.5 + SL_MAGMA*halo(rayPt(ro, rd, p, t), 0.25)*0.3)*smoothstep(0.0, 0.05, q)*(1.0 - q)*roomAt(p);
  }
  return (fx*uFx) + cx_more(ro, rd, px)*uFx;
}

// ---- update 13: its floating layer (tools/sw_extras.py) ----
const vec3 XM = vec3(1.0, 0.4, 0.05);
const vec3 XH = vec3(1.0, 0.79, 0.667);
const vec3 XA = vec3(1.0, 0.45, 0.05);
const vec3 XD = vec3(0.3, 0.12, 0.015);
vec3 sw_extra(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  fx += 1.3*vec3(0.0);
  for (int ri = 0; ri < 4; ri++){
    float k = float(ri);
    float q = fract(uTime*0.1 + h11(k*4.3 + 157.0));
    float y0 = uLength*(0.06 + 0.6*h11(k*2.9 + 157.0 + floor(uTime*0.1 + h11(k*4.3 + 157.0))));
    float rr = 2.0 + 1.3*h11(k*6.7 + 157.0);
    fx += 1.15*sin(3.1416*q)*xComets(ro, rd, rr, rr + 0.6, 0.38, k/4.0 + 0.01*uTime + 20.41, y0, min(y0 + uLength*0.26, uLength*0.95), 0.42, k + 157.0, XH, XM);
  }
  fx += 1.2*gxKin(ro, rd, 2.0, 7.0, 2.20, 0.120, 0.22, 0.08, 0.96, -1.0, 8053.0, XH, XA);
  return fx;
}

vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(1.0) > 0.5) fx += sw_extra(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
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
vec3 sig_glow_34(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(1.000, 0.450, 0.050);
const vec3 SX_DEEP = vec3(0.500, 0.050, 0.000);
const vec3 SX_PALE = vec3(1.000, 0.800, 0.400);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += sxDrops(ro, rd, px, 11.0, 1.6, 0.14, 210.0, vec3(1.0, 0.8, 0.4), SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.6*sxFlames(b, 0.7, 2.5, 1.8, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.6*gxKin(ro, rd, 1.0, 9.0, 2.60, 0.180, 0.14, 0.08, 0.96, 1.0, 8066.0, vec3(1.000, 0.800, 0.400), vec3(1.000, 0.450, 0.050));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.26, 5.11, -0.010, 577.0, vec3(1.000, 0.450, 0.050)*0.24, vec3(0.500, 0.050, 0.000)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.450, 0.050)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(3.0) > 0.5){
    a += 1.3*vec3(0.0);
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
  float L = uLength;
  a += 1.43*gxKin(ro, rd, 5.0, 4.0, 2.60, 0.050, 0.60, 0.08, 0.96, 1.0, 9357.0, vec3(1.000, 0.800, 0.400), vec3(1.000, 0.450, 0.050));
  a += 1.2*vec3(0.0);
  a += 1.0*gxLanterns(ro, rd, px, 4.0, 3.6, 41.0, vec3(1.000, 0.800, 0.400), vec3(1.000, 0.450, 0.050));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

