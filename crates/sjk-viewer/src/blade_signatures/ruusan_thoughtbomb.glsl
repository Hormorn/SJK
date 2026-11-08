// Ruusan Thoughtbomb: the light of the Jedi bound into the Force on the field of Ruusan, shining
// like a hopeful dawn. Its own body is silver-white light with a clear blue
// heart and a fine bright edge; pale blue-white flames lick calmly up off it, silver sparks rise
// from them, and soft shafts of golden dawn light break out of it at a slant. Every 24 seconds the
// dawn: the golden light swells, the flames rise tall and white-gold and a shower of bright sparks
// is thrown out, then it calms to silver again. Lit, a white flame runs up it from the hilt as if
// reforged; let go, the flames sink and a last gleam runs to the point. A clash rings out in white
// sparks. Its trail is the westflame (`@trail thoughtbomb`). No figures, no rings, no symbols.
// @part Reforged body
// @part Pale flames
// @part Dawn light
// @part Silver sparks
// @part The dawn
// @part Kyber fragments
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail thoughtbomb
// @palette {"corona.rim_cool":[0.35,0.45,0.65],"corona.rim_hot":[0.8,0.88,1.0],"corona.inner":[0.96,0.98,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.5,0.6,0.85],"core.fringe_hot":[0.88,0.93,1.0],"light.color":[0.85,0.9,1.0],"trail":[0.85,0.9,1.0]}
// @speed 3.6

const vec3 WF_SILVER = vec3(0.85, 0.9, 1.0);
const vec3 WF_BLUE = vec3(0.45, 0.6, 1.0);
const vec3 WF_GOLD = vec3(1.0, 0.82, 0.45);
const vec3 WF_WHITE = vec3(1.0, 1.0, 1.0);

float wf_top(){ return (uLength + 0.4)*rxLife(0.0, 0.5, 0.8); }
float wf_after(){ return rxLife(0.3, 1.2, 0.8); }
float wf_dawn(){ return part(4.0) > 0.5 ? rxCycle(24.0, 15.0, 17.5, 19.0, 22.0)*wf_after() : 0.0; }

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = wf_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half = rxHalf(clamp(b.y, 0.0, top), top, 0.62, 1.0);
    float d = rxOut(b, top, half);
    float dawn = wf_dawn();
    col = mix(WF_SILVER, WF_GOLD, dawn*0.5)*halo(d, 0.5)*0.9 + WF_BLUE*halo(d, 1.4)*0.3 + WF_WHITE*halo(d, 0.12)*0.7;
    // the reforging: a white flame running up it as it is lit
    if (uAge < 1.6) col += WF_WHITE*halo(abs(b.y - top*smoothstep(0.0, 1.2, uAge)), 1.5)*halo(d, 0.8)*1.2;
  }
  col += lvl(g)*WF_SILVER*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.4;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.3*(1.0 - smoothstep(0.3, 2.0, b.y)); }

// effect 1: the reforged body, silver-white with a clear blue heart and a fine bright edge
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = wf_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = rxHalf(clamp(b.y, 0.0, top), top, 0.62, 1.0);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  vec3 col = mix(WF_BLUE*1.2, WF_SILVER*1.1, smoothstep(0.0, 0.6, u));
  col = mix(col, WF_WHITE*1.2, smoothstep(0.85, 1.0, u));
  if (part(0.0) > 0.5){
    col += WF_WHITE*pow(0.5 + 0.5*sin(b.y*0.4 - tCalm()*1.5), 8.0)*0.3*(1.0 - u);
    // (update 19) seams of old gold through the white, as a broken blade mended with gold
    float seam = abs(rxFlow(vec2(b.x*2.6 + 3.0, b.y*0.22 + 0.04*uTime)) - 0.5);
    float seam2 = abs(rxFlow(vec2(b.x*4.1 - 2.0, b.y*0.37 - 0.03*uTime + 5.0)) - 0.5);
    col = mix(col, WF_GOLD*1.25, (1.0 - smoothstep(0.0, 0.05, seam))*0.9);
    col = mix(col, WF_GOLD*1.1, (1.0 - smoothstep(0.0, 0.035, seam2))*0.65);
  }
  // (update 21, no more orb) the thought bomb in the blade itself: all its light drains to one place
  // from both ends, leaving dark steel with only the gold seams; there it bursts, and the white runs
  // back out to both ends
  if (part(4.0) > 0.5){
    float c = mod(uClock + uSeed*7.0, 24.0);
    float mid = uLength*0.58;
    float gone = smoothstep(8.0, 14.0, c)*(1.0 - smoothstep(15.0, 19.5, c));
    float dist = abs(b.y - mid)/max(uLength*0.6, 1.0);
    float drained = smoothstep(1.0 - gone*1.1, 1.08 - gone*1.1, dist);
    float back = clamp((c - 15.0)/4.5, 0.0, 1.0);
    float front = halo(dist - back, 0.14)*smoothstep(14.6, 15.4, c)*(1.0 - back);
    vec3 steel = vec3(0.09, 0.1, 0.13) + WF_GOLD*(1.0 - smoothstep(0.0, 0.05, abs(rxFlow(vec2(b.x*2.6 + 3.0, b.y*0.22 + 0.04*uTime)) - 0.5)))*0.75;
    col = mix(col, steel, drained*gone);
    col += WF_WHITE*halo(dist, 0.03 + 0.1*gone)*gone*gone*2.0;
    col += (WF_WHITE*1.4 + WF_GOLD*0.6)*front;
  }
  col = mix(col, WF_GOLD*1.1, wf_dawn()*0.3);
  col = mix(col, WF_WHITE, 0.5*clamp(uClash, 0.0, 1.0));
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t;
  float aft = wf_after();
  float dawn = wf_dawn();
  if (fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  // effect 2 (1/11/2026, Creyon: the flames looked cheap): long ribbons of silver and gold light
  // sweeping slowly round it, and a soft silver-blue sheath of Force light rising off it
  if (part(1.0) > 0.5){
    fx += (0.5 + 0.5*dawn)*shSheath(ro, rd, 1.4, 1.0, L - 0.6, 1.6, 0.09, 2.0, 0.48, 0.0, WF_SILVER, WF_BLUE*0.4)*aft;
    for (int ri = 0; ri < 6; ri++){
      float k = float(ri);
      float q = fract(tCalm()*0.08 + h11(k*4.7 + 51.0));
      float y0 = uLength*(0.06 + 0.6*h11(k*2.7 + floor(tCalm()*0.08 + h11(k*4.7 + 51.0))));
      float rr = 2.1 + 1.5*h11(k*6.9);
      fx += ((1.1 + 0.9*dawn)*aft)*sin(3.1416*q)*xBeads(ro, rd, rr, rr + 0.6, 0.32, k/6.0 + 0.0048*tCalm(), y0, min(y0 + uLength*0.23, uLength*0.95), 0.4, k + 51.0, WF_WHITE, mix(WF_SILVER, WF_GOLD, mod(k, 2.0)));
    }
  }
  // effect 3: spirit lights of the fallen hanging in the air round it, gold at the dawn
  if (part(2.0) > 0.5) fx += (1.0 + 1.0*dawn)*rxStarCloud(ro, rd, px, 30.0, 3.8, 0.8, L*0.95, 0.06, 5.0, WF_WHITE, mix(WF_SILVER, WF_GOLD, dawn))*aft;
  // the dawn's haze (update 18): soft gold and rose light hangs round the blade like the morning
  // over the field of Ruusan, rising slowly, burning brightly at the dawn
  if (part(2.0) > 0.5) fx += (0.55 + 1.1*dawn)*syMist(ro, rd, 0.6, L*0.93, 3.1, 0.16, 37.0, WF_GOLD*0.8, vec3(1.0, 0.55, 0.62)*0.3)*aft;
  // the thought bomb (update 19): before each dawn the spirits are drawn out of the air into one
  // place on the blade, a sphere of white light tightens round them, and it breaks into the dawn
  if (part(4.0) > 0.5) fx += wf_bomb(ro, rd, px)*aft;
  // effect 4: silver sparks rising from the flames
  if (part(3.0) > 0.5) fx += rxBorn(ro, rd, px, 16.0, 0.5, L*0.9, 0.3, 2.6, 5.0, 0.05, 53.0, WF_WHITE, WF_SILVER)*aft;
  // effect 5: the dawn's shower of bright sparks
  {
    float c = mod(uClock + uSeed*7.0, 24.0);
    if (part(4.0) > 0.5) fx += rxSparkShower(ro, rd, px, vec3(0.0, L*0.55, 0.0), (c - 17.5)/1.8, 22.0, 6.0, floor((uClock + uSeed*7.0)/24.0), WF_WHITE, WF_GOLD)*aft;
  }
  // moments: let go, a last gleam running to the point; a clash, white sparks
  if (uAge >= 2000.0 && uAge < 2000.8){
    float q = (uAge - 2000.0)/0.8;
    fx += WF_WHITE*halo(rayPt(ro, rd, vec3(0.0, L*q, 0.0), t), 0.6)*1.4*(1.0 - q);
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += rxSparkShower(ro, rd, px, vec3(0.0, L*0.6, 0.0), 1.0 - cl, 16.0, 4.0, floor(uTime), WF_WHITE, WF_SILVER) + WF_WHITE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 0.9)*cl;
  return fx*uFx;
}


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  return fx*hiltFade(ro, rd);
}

// effect 6 (1/11/2026, update 14): kyber fragments, splinters of the crystals the thought bomb
// shattered, clear as ice, drifting slowly round the blade and turning, the dawn's gold and rose
// caught in their facets and burning along their edges
vec4 x14_ruusan_thoughtbomb(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  for (int i = 0; i < 7; i++){
    float k = float(i);
    vec3 c = around(2.4 + 1.2*h11(k*5.1), h11(k*1.7) + uClock*(0.02 + 0.015*h11(k*2.9)), uLength*(0.2 + 0.72*fract(k*0.618 + 0.1)) + 0.5*sin(uTime*0.4 + k*2.0));
    vec2 sh;
    float cov = xGem(ro, rd, c, 0.9 + 0.5*h11(k*9.1), uClock*(0.4 + 0.5*h11(k)), k, px, sh);
    if (cov <= 0.0) continue;
    vec3 col = mix(vec3(0.75, 0.85, 0.95), vec3(1.0, 0.85, 0.7), sh.x)*(0.35 + 0.5*pow(sh.x, 2.0)) + mix(vec3(1.0, 0.75, 0.4), vec3(1.0, 0.55, 0.7), sh.x)*sh.y*1.4;
    o = vec4(mix(o.rgb, col, cov*0.85), max(o.a, cov*0.85));
  }
  return o;
}
vec4 sig_vol_solid_34(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx > 0.0 && part(5.0) > 0.5){
    vec4 h = x14_ruusan_thoughtbomb(ro, rd, px);
    h.a *= uFx*rxLife(0.3, 1.0, 0.6)*hiltFade(ro, rd);
    o = vec4(mix(o.rgb, h.rgb, h.a), max(o.a, h.a));
  }
  return o;
}

vec3 wf_bomb(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float c = mod(uClock + uSeed*7.0, 24.0);
  if (c < 8.5 || c > 15.6) return acc;
  vec3 at = vec3(0.0, uLength*0.58, 0.0);
  if (fxNear(ro, rd, at.y - 8.0, at.y + 8.0) > 9.0) return acc;
  float t;
  // the spirits drawn in over seven seconds, each a soft point with a short tail
  float q = clamp((c - 8.5)/7.0, 0.0, 1.0);
  float s;
  for (int i = 0; i < 18; i++){
    float k = float(i);
    float e = clamp(q*(1.0 + 0.5*h11(k*2.3)), 0.0, 1.0);
    e = e*e;
    float e2 = max(e - 0.05, 0.0);
    float a0 = h11(k*5.3), y0 = (h11(k*7.9) - 0.5)*18.0, r0 = 3.0 + 3.5*h11(k*3.1);
    vec3 head = at + around(mix(r0, 0.1, e), a0 + 0.8*e, y0*(1.0 - e));
    vec3 tail = at + around(mix(r0, 0.1, e2), a0 + 0.8*e2, y0*(1.0 - e2));
    float d = raySeg(ro, rd, tail, head, s);
    acc += (WF_WHITE*halo(d, max(0.04 + 0.05*s, px))*1.1*s + WF_SILVER*halo(d, 0.26)*0.3)*smoothstep(0.0, 0.1, q)*(1.0 - smoothstep(0.88, 1.0, e))*roomFade(mix(r0, 0.1, e));
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
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(0.800, 0.880, 1.000);
const vec3 SX_DEEP = vec3(0.350, 0.450, 0.650);
const vec3 SX_PALE = vec3(0.960, 0.980, 1.000);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 3.0, 9.0, 3.20, 0.100, 0.22, 0.08, 0.96, 1.0, 7962.0, vec3(1.0), vec3(1.0, 0.78, 0.35));
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = sig_vol_solid_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  float L = uLength, C = uClock;
  vec4 n = (sxShards(ro, rd, px, 5.0, 3.2, 0.9, 0.018, 218.0, vec3(0.16, 0.18, 0.2), vec3(1.0, 0.85, 0.6)))*rxLife(0.45, 1.3, 0.4)*uFx*hiltFade(ro, rd);
  return vec4(mix(o.rgb, n.rgb, n.a), max(o.a, n.a));
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(5.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.45, 4.60, 0.014, 878.0, vec3(0.800, 0.880, 1.000)*0.24, vec3(0.350, 0.450, 0.650)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.800, 0.880, 1.000)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(5.0) > 0.5){
    a += 1.3*vec3(0.0);
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
  a += 1.40*gxKin(ro, rd, 8.0, 6.0, 3.00, 0.100, 0.50, 0.08, 0.96, 1.0, 9323.0, vec3(0.960, 0.980, 1.000), vec3(0.800, 0.880, 1.000));
  a += 1.25*gxVortex(ro, rd, px, 15.0, 5.6, 50.0, vec3(0.960, 0.980, 1.000), vec3(0.800, 0.880, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

