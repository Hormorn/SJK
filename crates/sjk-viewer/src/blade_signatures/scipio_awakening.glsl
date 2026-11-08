// Scipio Awakening: a warrior's first golden awakening. A shimmering gold blade, a spiky aura of golden
// ki standing up off its edges and flickering upward, and golden ki dust rising off it. No
// figures.
// @part Spiky aura
// @part Ki dust
// @part Shimmer
// @raycache fast
// @trail flare
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.6,0.4,0.0],"corona.rim_hot":[1.0,0.82,0.2],"corona.inner":[1.0,0.96,0.7],"core.white":[1.0,0.99,0.9],"core.fringe_cool":[0.9,0.6,0.05],"core.fringe_hot":[1.0,0.9,0.45],"light.color":[1.0,0.8,0.2],"trail":[1.0,0.8,0.2]}
// @speed 2.0

const vec3 GK_GOLD = vec3(1.0, 0.78, 0.18);
const vec3 GK_PALE = vec3(1.0, 0.95, 0.65);

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  vec3 col = recol(g, GK_GOLD)*0.3;
  float top = uLength;
  float y = clamp(b.y, 0.0, top);
  float d = length(vec2(b.x, b.y - y));
  // effect 3: the shimmer, bands of brighter gold drifting up the blade
  if (part(2.0) > 0.5) col += GK_PALE*halo(d, 0.5)*pow(bands(b.y + 0.4*sin(b.x*3.0), 0.12, 0.8), 3.0)*0.35;
  // effect 1: the spiky aura, soft spikes of golden ki standing up off its edges, leaning up,
  // flickering slowly (rounded at their points, never cut)
  if (part(0.0) > 0.5 && uFx > 0.0 && b.y > 0.3){
    float side = clamp(b.x*2.0, -1.0, 1.0);
    float lane = b.y*0.9 - d*0.8 + side*2.0;
    float spike = 0.5 + 0.5*sin(lane*1.6 + 0.6*sin(lane*0.7 + (uTime*0.4)*0.9) - (uTime*0.4)*1.5);
    spike = pow(spike, 3.0);
    float reach = 0.8 + 2.2*spike*(0.75 + 0.25*sin((uTime*0.4)*2.1 + lane));
    float aura = (1.0 - smoothstep(reach*0.25, reach, d))*smoothstep(0.05, 0.4, d);
    col += (GK_GOLD*0.55 + GK_PALE*0.2*spike)*aura*(1.0 - smoothstep(top - 1.0, top + 2.5, b.y))*uFx;
  }
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }

// effect 2: golden ki dust, tiny round motes rising off the blade and fading
// The 3D structure round it (1/11/2026, Creyon: the lower tiers filled out so they are never
// plain, each its own shape): the spiky aura grown up: slim spikes of golden ki standing straight up round the blade. Lit, it grows in; let go, it fades.
vec3 cx_more(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  float aft = rxLife(0.2, 1.0, 0.6);
  if (part(0.0) < 0.5 || aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  {
    for (int i = 0; i < 12; i++){
      float k = float(i);
      float yR = L*(0.04 + 0.66*fract(k*0.618 + 1.61));
      float len = min(3.0 + 1.4*h11(k*5.7), L + 0.5 - yR);
      fx += 1.2*rxTongue(ro, rd, yR, (k + 0.5)/12.0 + 0.006*tCalm(), len, 0.7 + 0.4*h11(k*3.1), 0.45, k + 161.0, GK_PALE, GK_GOLD, GK_GOLD*0.6);
    }
  }
  return fx*aft;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  if (part(1.0) < 0.5) return cx_more(ro, rd, px)*uFx;
  float L = uLength, t;
  if (fxNear(ro, rd, 0.0, L + 3.0) > 4.0) return fx + cx_more(ro, rd, px)*uFx;
  for (int i = 0; i < 14; i++){
    float k = float(i);
    float cyc = (uTime*0.4)*(0.2 + 0.12*h11(k*2.3)) + h11(k*7.1);
    float n = floor(cyc), q = fract(cyc);
    vec3 p = around(0.5 + 1.2*q, h11(n*3.7 + k), L*(0.05 + 0.9*h11(n*5.1 + k)) + 3.0*q);
    float d = rayPt(ro, rd, p, t);
    float life = smoothstep(0.0, 0.12, q)*(1.0 - smoothstep(0.55, 1.0, q));
    fx += (GK_PALE*halo(d, max(0.05, px))*1.4 + GK_GOLD*halo(d, 0.22)*0.4)*life*roomAt(p);
  }
  return (fx*uFx) + cx_more(ro, rd, px)*uFx;
}

// ---- update 13: its floating layer (tools/sw_extras.py) ----
const vec3 XM = vec3(1.0, 0.8, 0.2);
const vec3 XH = vec3(1.0, 0.93, 0.72);
const vec3 XA = vec3(1.0, 0.82, 0.2);
const vec3 XD = vec3(0.3, 0.24, 0.06);
vec3 sw_extra(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  fx += 1.0*shStrands(ro, rd, 1.25, 1.0, uLength - 0.6, 3.0, 0.02, 0.03, 0.14, 2.5, 213.0, XH, XM);
  for (int ci = 0; ci < 2; ci++){
    float k = float(ci);
    fx += 1.0*rxComet(ro, rd, px, 2.6 + 0.8*k, uLength*(0.12 + 0.22*k), uLength*0.35, 11.0 + 3.0*k, k + 214.0, XH, XA);
  }
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
  float fade = smoothstep(-0.25, 0.1, b.y);
  if (b.y < -0.3 || b.y > top) return o;
  float e0 = 1.0 - clamp((top - b.y)/1.14, 0.0, 1.0);
  float half = 0.76*sqrt(max(1.0 - e0*e0, 0.0))*mix(0.6, 1.0, smoothstep(0.0, 1.2, b.y));
  float cov = fillD(abs(b.x) - half, px)*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(abs(b.x)/max(half, 0.01), 0.0, 1.0);
  float sw = 0.5 + 0.5*sin(b.x*3.4 + b.y*0.55 - uClock*0.8 + 1.6*sin(b.y*0.21 - uClock*0.3));
  vec3 col = mix(vec3(1.00, 0.97, 0.84), vec3(1.00, 0.84, 0.42), smoothstep(0.25, 1.0, u));
  col = mix(col, vec3(0.92, 0.56, 0.14), 0.4*sw*sw*smoothstep(0.1, 0.9, u));
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
vec3 sig_glow_34(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(1.000, 0.820, 0.200);
const vec3 SX_DEEP = vec3(0.600, 0.400, 0.000);
const vec3 SX_PALE = vec3(1.000, 0.960, 0.700);
vec3 sig_vol_e1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 1.0*gxKin(ro, rd, 1.0, 10.0, 2.20, 0.160, 0.14, 0.08, 0.96, 1.0, 8001.0, SX_PALE, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_e1(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (vec3(0.0))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34
// >>> e1: the great effect round this blade (4/11/2026)
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the awakening rising: broad veils of gold going up round it, golden ki flakes and a slow bright dust
  a += 1.5*xDust(ro, rd, 1.6, 3.6, 1.0, C*0.04, L*0.03, L*0.97, 1.3, 414.0, GK_PALE, GK_GOLD);
  a += 1.5*shFlakes(ro, rd, 3.0, 0.5, L - 0.5, 16.0, 2.6, 1.1, 0.03, 0.09, 0.6, 0.0, 415.0, GK_PALE, GK_GOLD*0.6);

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_e1(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  // the aura swelling: slow waves of gold light going up it
  float sw = pow(0.5 + 0.5*sin(b.y*0.34 - C*1.0), 3.0);
  col += mix(GK_GOLD, GK_PALE, 0.25)*sw*(halo(d, 1.3)*0.34 + halo(d, 3.4)*0.2)*on;

  return col;
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  a += 1.4*vec3(0.0);
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  if (part(2.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.57*gxKin(ro, rd, 3.0, 8.0, 2.00, 0.140, 0.10, 0.08, 0.96, 1.0, 9340.0, vec3(1.000, 0.960, 0.700), vec3(1.000, 0.820, 0.200));
  a += 1.2*gxFlares(ro, rd, px, 7.0, 5.0, 36.0, vec3(1.000, 0.960, 0.700), vec3(1.000, 0.820, 0.200));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

