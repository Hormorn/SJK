// Iridonia Rite: a body pushed past its limits. Over a slow cycle eight gates of light open one by
// one up the blade, each turning slowly round it, while the blade's colour climbs from jade green
// through blue to crimson; steam pours off it, thicker as more gates stand open, and its heart
// throbs with each gate. Then the gates close, softly, from the top down. No figures.
// @part Gates
// @part Rising colour
// @part Steam
// @part Gate throb
// @raycache fast
// @trail sorcery
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.0,0.45,0.2],"corona.rim_hot":[0.2,0.95,0.55],"corona.inner":[0.75,1.0,0.85],"core.white":[0.95,1.0,0.96],"core.fringe_cool":[0.05,0.65,0.3],"core.fringe_hot":[0.5,1.0,0.7],"light.color":[0.2,0.9,0.5],"trail":[0.2,0.9,0.5]}
// @speed 2.0

const vec3 EG_JADE = vec3(0.15, 0.9, 0.5);
const vec3 EG_BLUE = vec3(0.2, 0.55, 1.0);
const vec3 EG_RED = vec3(1.0, 0.12, 0.1);
const vec3 EG_STEAM = vec3(0.9, 0.95, 0.95);

// How many gates stand open on a 30-second cycle (0 to 8, each opening over a second and a
// half, one every two seconds; closing from the top down after it holds).
float eg_open(){
  float c = mod((uTime*0.5) + uSeed*5.0, 30.0);
  if (c < 18.0) return clamp((c - 2.0)/2.0, 0.0, 8.0);
  return clamp(8.0 - (c - 20.0)/1.0, 0.0, 8.0);
}
vec3 eg_colour(float open){
  float k = open/8.0;
  if (part(1.0) < 0.5) return EG_JADE;
  vec3 c = mix(EG_JADE, EG_BLUE, smoothstep(0.2, 0.55, k));
  return mix(c, EG_RED, smoothstep(0.6, 0.95, k));
}
float eg_flow(vec2 p){
  float v = sin(p.x + 1.5*sin(p.y*0.9 + 0.4));
  v += 0.8*sin(p.y*1.3 + 1.3*sin(p.x*1.2 + 1.1));
  v += 0.5*sin((p.x + p.y)*1.9 + 1.1*sin(p.x*2.1 - p.y*0.6 + 2.7));
  return 0.5 + v/4.6;
}

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float open = eg_open();
  vec3 tint = eg_colour(open);
  vec3 col = recol(g, tint)*0.16;
  float top = uLength;
  float y = clamp(b.y, 0.0, top);
  float d = length(vec2(b.x, b.y - y));
  // effect 4: the gate throb, its heart throbbing brighter as each gate opens
  if (part(3.0) > 0.5){
    float th = pow(0.5 + 0.5*sin(uTime*0.62), 4.0); // (update 16) a steady speed: one that followed the moment jumped by the blade's whole age
    col += tint*halo(d, 0.7)*th*0.25*(open/8.0);
  }
  // effect 3: steam pouring off it, soft plumes rising from its sides, thicker with more gates
  if (part(2.0) > 0.5 && uFx > 0.0){
    vec2 q = vec2(b.x*0.8, b.y*0.3 - (uTime*0.4)*1.1);
    float plume = eg_flow(q + 0.7*vec2(eg_flow(q*0.6 + 3.0), eg_flow(q*0.6 + 8.0)));
    float thick = 0.25 + 0.75*open/8.0;
    float reach = 1.0 + 2.5*thick*plume;
    col += EG_STEAM*smoothstep(0.45, 0.85, plume)*(1.0 - smoothstep(reach*0.3, reach, d))*smoothstep(0.2, 0.7, d)*0.3*thick*(1.0 - smoothstep(top, top + 4.0, b.y))*uFx;
  }
  // the rite's own aura: tall flames of its colour up both sides, thicker with every gate
  if (part(0.0) > 0.5 && uFx > 0.0){
    float side = b.x > 0.0 ? 2.0 : 8.0;
    float f1 = rxFlow(vec2(b.y*0.3 - uClock*2.3, side));
    float f2 = rxFlow(vec2(b.y*0.72 - uClock*3.4, side + 3.0));
    float along = smoothstep(0.5, 3.5, b.y)*(1.0 - smoothstep(top - 3.0, top - 0.5, b.y));
    float reach = 0.5 + (1.2 + 1.6*open/8.0)*f1*f1 + 0.7*f2*f2;
    float qf = clamp(max(abs(b.x) - 0.72, 0.0)/max(reach, 0.05), 0.0, 1.0);
    col += tint*(1.0 - qf)*(1.0 - qf)*0.5*along*rxLife(0.25, 1.0, 0.4);
  }
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){
  return c*0.0;
}

// effect 1: the gates, opening one by one up the blade: as each opens, tongues of steaming
// light burst out round the blade at its height in the gate's colour, and sink as it closes (no rings)
// The 3D structure round it (1/11/2026, Creyon: the lower tiers filled out so they are never
// plain, each its own shape): the steam: as the gates open, plumes of hot steam rise off the blade, thicker with every gate, tinted with the gates' colour, and a haze of it round the blade. Lit, it grows in; let go, it fades.
vec3 cx_more(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  float aft = rxLife(0.2, 1.0, 0.6);
  if (part(2.0) < 0.5 || aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  {

    float open = eg_open();
    vec3 tint = eg_colour(open);
    for (int i = 0; i < 8; i++){
      float k = float(i);
      if (open < k*0.8) continue;
      float yR = L*(0.05 + 0.7*fract(k*0.618 + 0.2));
      float len = min(5.0 + 3.0*h11(k*5.7), L + 0.5 - yR);
      fx += 0.08*rxTongue(ro, rd, yR, (k + 0.5)/8.0 + 0.0016*tCalm(), len, 0.8 + 0.5*h11(k*3.1), 0.45, k + 191.0, mix(tint, vec3(1.0), 0.25), tint*0.8, tint*0.25);
    }
    fx += (0.16 + 0.04*open)*syMist(ro, rd, 0.5, L*0.95, 4.2, 0.5, 14.0, mix(EG_STEAM, tint, 0.5), tint*0.4);
  }
  return fx*aft;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  if (part(0.0) < 0.5) return cx_more(ro, rd, px)*uFx;
  float L = uLength;
  if (fxNear(ro, rd, 0.0, L) > 16.0) return fx + cx_more(ro, rd, px)*uFx;
  float open = eg_open();
  vec3 tint = eg_colour(open);
  for (int i = 0; i < 8; i++){
    float k = float(i);
    float o = clamp(open - k, 0.0, 1.0);
    if (o <= 0.0) continue;
    o = o*o*(3.0 - 2.0*o);
    float y = L*(0.06 + 0.12*k);
    for (int j = 0; j < 2; j++){
      float m = float(j);
      fx += 0.06*rxTongue(ro, rd, y, k*0.37 + m*0.5 + 0.004*tCalm(), min((3.0 + 1.5*h11(k*3.3 + m))*(0.4 + 0.6*o), L + 0.5 - y), (1.6 + 0.8*h11(k*1.9 + m))*o, 0.5, k*2.0 + m + 190.0, mix(tint, vec3(1.0), 0.25), tint*0.8, tint*0.25)*o;
    }
  }
  return (fx*uFx) + cx_more(ro, rd, px)*uFx;
}

// ---- update 13: its floating layer (tools/sw_extras.py) ----
const vec3 XM = vec3(0.2, 0.9, 0.5);
const vec3 XH = vec3(0.72, 0.965, 0.825);
const vec3 XA = vec3(0.2, 0.95, 0.55);
const vec3 XD = vec3(0.06, 0.27, 0.15);
vec3 sw_extra(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  fx += 0.5*shStreaks(ro, rd, 2.2, 1.5, uLength - 0.6, 16.0, 2.4, 3.5, 0.3, 144.0, XH, XA);
  
  for (int ci = 0; ci < 2; ci++){
    float k = float(ci);
    fx += 1.0*rxComet(ro, rd, px, 2.8 + 0.8*k, uLength*(0.12 + 0.22*k), uLength*0.35, 11.0 + 3.0*k, k + 146.0, XH, XA);
  }
  return fx;
}

vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(1.0) > 0.5) fx += sw_extra(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  // (update 30, Creyon: the effects hid under the glow) soft flame, power and steam in the rite's own colour, never white-out
  if (uFx > 0.0 && fxNear(ro, rd, 0.0, uLength) < 9.0){
    float L = uLength, op = eg_open(), aft = rxLife(0.25, 1.0, 0.4);
    float lv = clamp(op/8.0, 0.0, 1.0);
    vec3 tint = eg_colour(op);
    vec3 more = vec3(0.0);
    if (part(0.0) > 0.5){
      more += (0.3 + 0.35*lv)*shFire(ro, rd, 1.2 + 0.9*lv, 0.3, L, 1.7, 0.55, 5.0, tint*0.05, tint*0.4, tint*0.8);
      more += vec3(0.0);
      more += (0.7 + 0.8*lv)*gxKin(ro, rd, 1.0, 10.0, 2.40, 0.140, 0.16, 0.08, 0.96, 1.0, 7494.0, mix(tint, vec3(1.0), 0.4), tint);
    }
    if (part(2.0) > 0.5) more += (0.42 + 0.3*lv)*syMistSpin(ro, rd, L*0.06, L*0.98, 1.2, 4.2 + 1.2*lv, 0.035, 72.0, tint*0.45, tint*0.16);
    fx += more*aft*uFx;
  }
  return fx*hiltFade(ro, rd);
}
// (update 30, Creyon: everything hid under the glow, the tip not filled) a real blade from hilt to point in the rite's
// colour: living energy flowing up it, each open gate a band of bright light on it, the last at the very tip
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength, top = L + 0.2;
  if (b.y < -0.3 || b.y > top) return o;
  float e0 = 1.0 - clamp((top - b.y)/1.05, 0.0, 1.0);
  float half = 0.72*sqrt(max(1.0 - e0*e0, 0.0))*mix(0.6, 1.0, smoothstep(0.0, 1.2, b.y));
  float cov = fillD(abs(b.x) - half, px)*smoothstep(0.02, 0.12 + px, half)*smoothstep(-0.25, 0.1, b.y);
  if (cov <= 0.0) return o;
  float op = eg_open();
  vec3 tint = eg_colour(op);
  vec3 pale = mix(tint, vec3(1.0), 0.62);
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0);
  float nz = sqrt(max(1.0 - u*u, 0.0));
  vec2 q = vec2(asin(u)*1.2, b.y*0.17 - uClock*1.2);
  float n = rxFlow(q + 0.8*rxFlow(q*0.7 + 4.0));
  vec3 col = mix(tint*0.32, tint*1.0, smoothstep(0.22, 0.75, n));
  col += pale*(1.0 - smoothstep(0.0, 0.07, abs(n - 0.55)))*0.65;
  if (part(0.0) > 0.5){
    for (int i = 0; i < 8; i++){
      float k = float(i), g = clamp(op - k, 0.0, 1.0);
      if (g <= 0.0) continue;
      float dy = b.y - L*(0.06 + 0.12*k);
      col += (pale*halo(abs(dy), 0.5)*1.05 + tint*halo(abs(dy), 2.0)*0.5)*g*(0.8 + 0.2*sin(uClock*2.4 + k*1.3));
    }
  }
  col *= 0.5 + 0.65*nz;
  col += tint*pow(1.0 - nz, 2.0)*0.7;
  col = mix(col, pale, (1.0 - smoothstep(0.0, 0.3, abs(u)))*0.3);
  col = mix(col, vec3(1.0), 0.45*clamp(uClash, 0.0, 1.0));
  float keep = max(o.a, cov);
  return vec4(mix(o.rgb, col, cov/max(keep, 0.001)), keep);
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
const vec3 SX_HOT = vec3(0.200, 0.950, 0.550);
const vec3 SX_DEEP = vec3(0.000, 0.450, 0.200);
const vec3 SX_PALE = vec3(0.750, 1.000, 0.850);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += vec3(0.0);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.44, 4.22, 0.009, 504.0, vec3(0.200, 0.950, 0.550)*0.24, vec3(0.000, 0.450, 0.200)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.200, 0.950, 0.550)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
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
  a += 1.51*gxKin(ro, rd, 8.0, 8.0, 2.20, 0.140, 0.50, 0.08, 0.96, 1.0, 9170.0, vec3(0.750, 1.000, 0.850), vec3(0.200, 0.950, 0.550));
  a += 1.2*gxLanterns(ro, rd, px, 6.0, 4.8, 32.0, vec3(0.750, 1.000, 0.850), vec3(0.200, 0.950, 0.550));
  a += 1.0*gxHelix(ro, rd, px, 12.0, 2.0, 1.6, 0.05, 33.0, vec3(0.750, 1.000, 0.850), vec3(0.200, 0.950, 0.550));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

