// Nightsister Ichor (rebuilt 2/11/2026, update 21; Creyon: it looks bad, the effect does not reach the
// top, it needs transitions): magick of the Nightsisters of Dathomir, thick as sap. The blade is
// a stream of glowing green ichor running down over a black stem, marbled with pale light, its
// outline swollen where it gathers. Heavy drops form on its edge, run down it, stretch and fall away.
// Green witch-mist clings round it from hilt to point, slow pale spirit lights rise through the mist.
// Every 20 seconds the magick wells: the marbling brightens from the hand to the point, the drops
// run faster and the mist rises, all of it swelling and easing back slowly.
// @part Ghostly mist
// @part Falling ichor
// @part Pale heart
// @part Spirit breath
// @part Welling magick
// @raycache fast
// @room 10
// @solid 1
// @trail hex
// @palette {"corona.rim_cool":[0.05,0.4,0.15],"corona.rim_hot":[0.3,1.0,0.45],"corona.inner":[0.8,1.0,0.8],"core.white":[0.95,1.0,0.92],"core.fringe_cool":[0.1,0.6,0.2],"core.fringe_hot":[0.55,1.0,0.6],"light.color":[0.25,0.95,0.4],"trail":[0.25,0.95,0.4]}
// @speed 1.0

const vec3 NI_GREEN = vec3(0.16, 0.95, 0.32);
const vec3 NI_PALE = vec3(0.78, 1.0, 0.7);
const vec3 NI_DEEP = vec3(0.01, 0.2, 0.06);
const vec3 NI_BLACK = vec3(0.012, 0.03, 0.018);

float ni_top(){ return (uLength + 0.3)*rxLife(0.0, 0.5, 0.5); }
float ni_after(){ return rxLife(0.3, 1.2, 0.5); }
float ni_well(){ return part(4.0) > 0.5 ? rxCycle(20.0, 9.0, 13.0, 15.0, 19.0)*ni_after() : 0.0; }
// a drop's place on the edge: `side`, its height and size. One slides down each lane at its own pace,
// slowly at first, then running.
vec3 ni_drop(float k, out float side){
  float cyc = uClock*(0.11 + 0.05*h11(k*2.3)) + h11(k*6.1);
  float n = floor(cyc), q = fract(cyc);
  side = h11(n*3.3 + k) < 0.5 ? -1.0 : 1.0;
  float y = uLength*(0.25 + 0.72*h11(n*5.7 + k)) - uLength*0.42*q*q;
  float size = (0.22 + 0.2*h11(n*2.1 + k))*smoothstep(0.0, 0.25, q)*(1.0 - smoothstep(0.9, 1.0, q));
  return vec3(q, y, size);
}
float ni_half(float y0, float top, float s0){
  float y = clamp(y0, 0.0, top);
  float swell = rxFlow(vec2(y*0.32 + uClock*0.4, s0*5.0 + 2.0));
  float w = 0.42 + 0.26*swell*swell;
  if (part(1.0) > 0.5){
    for (int i = 0; i < 5; i++){
      float side;
      vec3 d = ni_drop(float(i), side);
      if (side*s0 > 0.0){
        float e = (y - d.y)/max(d.z*2.2, 0.05);
        // a teardrop on the edge: round below, drawn out above
        w = max(w, 0.42 + d.z*1.4*(e < 0.0 ? exp(-e*e*3.0) : exp(-e*e*0.28)));
      }
    }
  }
  return rxHalf(y, top, w, 1.0);
}
vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = ni_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float yc = clamp(b.y, 0.0, top);
    float d = max(length(vec2(b.x, b.y - yc)) - ni_half(yc, top, b.x), 0.0);
    float wl = ni_well();
    col = NI_GREEN*(halo(d, 0.18)*0.42 + halo(d, 0.85)*(0.07 + 0.16*wl));
  }
  col += lvl(g)*NI_GREEN*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.3;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.6*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = ni_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = ni_half(b.y, top, b.x);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float wl = ni_well();
  vec2 q = vec2(b.x*2.2, b.y*0.36 + uClock*0.42);
  vec2 w = vec2(rxFlow(q*1.3 + 3.0), rxFlow(q*1.7 + 11.0)) - 0.5;
  float marble = rxFlow(q + w*2.6);
  float vein = 1.0 - smoothstep(0.0, 0.1, abs(marble - 0.5));
  float vein2 = 1.0 - smoothstep(0.0, 0.05, abs(rxFlow(q*2.3 + w*3.4 + 7.0) - 0.5));
  vec3 col = mix(NI_DEEP*0.7, NI_GREEN*0.72, smoothstep(0.35, 0.7, marble));
  col = mix(col, NI_BLACK, smoothstep(0.45, 0.0, marble)*0.8);
  col += NI_PALE*(vein*0.75 + vein2*0.4)*(0.55 + 1.2*wl*smoothstep(0.0, 1.0, wl*1.4 - b.y/max(uLength, 1.0)*0.4));
  if (part(2.0) > 0.5) col += NI_PALE*(1.0 - smoothstep(0.0, 0.45, u))*0.42*(0.75 + 0.25*sin(b.y*0.3 + uClock*1.2));
  col += NI_PALE*smoothstep(0.86, 1.0, u)*0.55;
  col += NI_PALE*halo(max(b.y, 0.0), 0.5)*0.4;
  col = mix(col, NI_PALE, 0.5*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t, s;
  float aft = ni_after();
  float wl = ni_well();
  // the witch-mist, low and clinging, from the hilt to the point
  if (part(0.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 4.5) fx += (0.55 + 0.6*wl)*shSheath(ro, rd, 1.5 + 0.5*wl, 0.1, L - 0.1, 1.4, 0.06, 2.2, 0.46, 0.0, NI_GREEN*0.65, NI_DEEP)*aft;
  // the drops that fall away, stretched, a thin thread behind each
  if (part(1.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 5.0){
    for (int i = 0; i < 5; i++){
      float side;
      vec3 d0 = ni_drop(float(i), side);
      if (d0.x < 0.82) continue;
      float f = (d0.x - 0.82)/0.18;
      vec3 start = vec3(side*0.6, d0.y, 0.0);
      vec3 p = start + vec3(side*0.15*f, -5.0*f*f, 0.0);
      float d = rayPt(ro, rd, p, t);
      fx += (NI_PALE*halo(d, max(0.09, px))*1.5 + NI_GREEN*halo(d, 0.3)*0.5)*(1.0 - f);
      float dt = raySeg(ro, rd, start, p, s);
      fx += NI_GREEN*halo(dt, max(0.02, px))*0.5*s*(1.0 - f);
    }
  }
  // spirit lights rising slowly through the mist, swelling and fading
  if (part(3.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 6.0){
    for (int i = 0; i < 10; i++){
      float k = float(i);
      float cyc = uClock*(0.07 + 0.03*h11(k*2.9)) + h11(k*7.3);
      float n = floor(cyc), q = fract(cyc);
      vec3 p = around(1.4 + 2.2*h11(n*3.1 + k) + 0.3*sin(uClock*0.8 + k), h11(n*5.3 + k) + 0.15*q, L*(0.04 + 0.92*q));
      float d = rayPt(ro, rd, p, t);
      fx += (NI_PALE*halo(d, max(0.09, px))*0.95 + NI_GREEN*halo(d, 0.45)*0.3)*sin(3.1416*q)*roomAt(p)*aft;
    }
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += (NI_PALE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 0.9)*1.1 + NI_GREEN*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 2.8)*0.3)*cl;
  // Dathomir's magick: green flame licking up it and a low mist of ichor round it (update 25)
  float L2 = uLength;
  if (fxNear(ro, rd, 0.0, L2) < 8.0){
    for (int i = 0; i < 5; i++){
      float k = float(i);
      fx += 0.85*rxTongue(ro, rd, L2*(0.1 + 0.17*k), k*0.33 + 0.012*uClock, 3.8, 1.1, 0.42, k + 3.0, NI_PALE, NI_GREEN, NI_DEEP);
    }
    fx += 0.5*syMist(ro, rd, L2*0.1, L2*0.8, 3.0, 0.15, 7.0, NI_GREEN*0.5, NI_DEEP);
  }
  return fx*uFx*hiltFade(ro, rd);
}
// (update 26) the body starts softly in the emitter, no hard end above the hilt
vec4 sig_solid_e1(vec4 o, vec2 b, float px){
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
const vec3 SX_HOT = vec3(0.300, 1.000, 0.450);
const vec3 SX_DEEP = vec3(0.050, 0.400, 0.150);
const vec3 SX_PALE = vec3(0.800, 1.000, 0.800);
vec3 sig_vol_e1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += sxDrops(ro, rd, px, 9.0, 1.2, 0.08, 131.0, SX_PALE, SX_HOT);  a += 0.9*syMistSpin(ro, rd, L*0.05, L*0.98, 1.2, 4.2, 0.02, 133.0, SX_HOT*0.35, SX_DEEP*0.4);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.8*sxFlames(b, 0.7, 2.8, 1.3, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34
// >>> e1: the great effect round this blade (4/11/2026)
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // spirits of Dathomir turning slowly in the dark round it, long and pale, and black-green magick rising through them
  if (part(3.0) > 0.5) a += 1.5*gxKin(ro, rd, 5.0, 4.0, 2.60, 0.050, 0.50, 0.08, 0.96, 1.0, 7819.0, NI_PALE, NI_GREEN*0.8);
  if (part(4.0) > 0.5) for (int i = 0; i < 5; i++){
    float k = float(i), run = C*(0.13 + 0.03*h11(k)) + k*0.31, cyc = floor(run), ph = fract(run), en = sin(3.1416*ph);
    a += 1.1*en*en*syLick(ro, rd, px, L*(0.06 + 0.72*h11(k*2.3 + cyc)), h11(k*5.7 + cyc) + 0.05*ph, 1.8 + 1.6*h11(k*3.1 + cyc), 3.8 + 3.0*h11(k*4.3 + cyc), 0.5, k*3.0 + cyc, NI_GREEN*0.8, NI_DEEP*2.6, NI_DEEP*0.8);
  }

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// The dark it stands in: a slow black cloud of Dathomir's night hanging behind it, green flecks of magick adrift in it, soft where it ends.
vec4 sig_solid(vec4 o, vec2 b, float px){
  if (part(0.0) > 0.5 && uFx > 0.0 && b.y > 0.0 && b.y < uLength){
    float d = abs(b.x), side = b.x > 0.0 ? 2.0 : 6.0;
    float reach = (2.2 + 2.0*rxFlow(vec2(b.y*0.11 - uClock*0.07, side)))*smoothstep(0.0, uLength*0.2, b.y)*(1.0 - smoothstep(uLength*0.74, uLength, b.y));
    float a = (1.0 - smoothstep(reach*0.25, reach, d))*0.88*rxLife(0.4, 1.4, 0.3);
    if (a > 0.003){
      vec2 q = vec2(d*0.7 - uClock*0.07 + side, b.y*0.2 - uClock*0.05);
      float n = rxFlow(q + 0.8*(rxFlow(q*1.3 + 4.0) - 0.5));
      vec3 dark = mix(NI_BLACK*0.5, NI_DEEP*0.9, smoothstep(0.45, 0.8, n));
      vec2 sc = vec2(b.x*2.0, b.y*1.2 + uClock*0.1), si = floor(sc), sp = si + 0.25 + 0.5*h22(si);
      dark += NI_GREEN*halo(length(sc - sp), 0.06)*step(0.72, h21(si + 5.0))*(0.5 + 0.5*sin(uClock*(0.8 + h21(si)) + h21(si)*20.0));
      o = vec4(mix(o.rgb, dark, a), max(o.a, a));
    }
  }
  return sig_solid_e1(o, b, px);
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
  if (part(4.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.23*gxKin(ro, rd, 3.0, 6.0, 2.40, 0.050, 0.16, 0.08, 0.96, -1.0, 9255.0, vec3(0.800, 1.000, 0.800), vec3(0.300, 1.000, 0.450));
  a += 1.2*gxLanterns(ro, rd, px, 7.0, 4.6, 44.0, vec3(0.800, 1.000, 0.800), vec3(0.300, 1.000, 0.450));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

