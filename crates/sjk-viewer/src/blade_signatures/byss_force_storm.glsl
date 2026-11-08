// Byss Force Storm (new 2/11/2026, update 21, in place of Nur Shadowflame): the Emperor's own weapon, the
// Force storm over the throne world of Byss, a tear in space that swallowed fleets. The blade is the
// storm's funnel: a thin dark rope at the hand, opening steadily into a wide turning tornado at
// the point, its walls twisting strands of black and violet, blue-white Force lightning burning
// inside it. Torn wreckage is carried round it on a rising spiral and flung off the top. At the crown
// the eye of the storm burns white. Every 24 seconds the storm surges: the funnel swells, the
// lightning blazes along its whole length and the wreckage whirls wider.
// @part Storm funnel
// @part Emperor's lightning
// @part Storm arms
// @part Eye of the storm
// @part The wind
// @part Storm surge
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail shadowflame
// @palette {"corona.rim_cool":[0.2,0.05,0.5],"corona.rim_hot":[0.6,0.3,1.0],"corona.inner":[0.85,0.78,1.0],"core.white":[0.95,0.92,1.0],"core.fringe_cool":[0.3,0.12,0.75],"core.fringe_hot":[0.7,0.5,1.0],"light.color":[0.55,0.3,1.0],"trail":[0.55,0.3,1.0]}
// @speed 1.0

const vec3 BY_BLACK = vec3(0.012, 0.006, 0.03);
const vec3 BY_VIOLET = vec3(0.42, 0.14, 0.95);
const vec3 BY_PALE = vec3(0.72, 0.6, 1.0);
const vec3 BY_BOLT = vec3(0.68, 0.82, 1.0);
const vec3 BY_WHITE = vec3(0.96, 0.96, 1.0);

float by_top(){ return (uLength + 0.2)*rxLife(0.0, 0.6, 0.5); }
float by_after(){ return rxLife(0.3, 1.2, 0.5); }
float by_surge(){ return part(5.0) > 0.5 ? rxCycle(24.0, 14.0, 17.0, 18.5, 22.0)*by_after() : 0.0; }
// the funnel's half width: a thin rope opening to a wide mouth, breathing, rounding off at the top
float by_half(float y0, float top){
  float y = clamp(y0, 0.0, top);
  float s = clamp(y/max(uLength, 1.0), 0.0, 1.0);
  float open = 0.3 + (1.7 + 0.5*by_surge())*s*s*sqrt(s);
  float wob = 0.12*sin(y*0.22 - uClock*1.2) + 0.04*sin(y*0.5 + uClock*1.5);
  float e0 = 1.0 - clamp((top - y)/2.2, 0.0, 1.0);
  float end = sqrt(max(1.0 - e0*e0, 0.0));
  return (open + wob*(0.3 + s))*end;
}
vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = by_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float yc = clamp(b.y, 0.0, top);
    float d = max(length(vec2(b.x, b.y - yc)) - by_half(b.y, top), 0.0);
    float s = clamp(b.y/max(uLength, 1.0), 0.0, 1.0);
    float sg = by_surge();
    col = BY_VIOLET*(halo(d, 0.22)*0.45 + halo(d, 0.9 + 0.7*s)*(0.07 + 0.14*s + 0.2*sg));
    // the eye's light spilling over the crown
    if (part(3.0) > 0.5) col += BY_PALE*halo(length(vec2(b.x, b.y - uLength*0.97)), 1.7)*0.14*(1.0 + sg);
  }
  col += lvl(g)*BY_PALE*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.3;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.7*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core_d(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = by_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = by_half(b.y, top);
  float x = abs(b.x);
  float cov = (1.0 - smoothstep(half - 0.06 - px, half + 0.03 + px, x))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float across = clamp(b.x/max(half, 0.01), -1.0, 1.0);
  float nz = sqrt(max(1.0 - across*across, 0.0));
  float s = clamp(b.y/max(uLength, 1.0), 0.0, 1.0);
  float sg = by_surge();
  // (update 30, Creyon: you see the lines in it; one whole effect) the walls are one turning storm cloud,
  // thick and soft, darker in its folds; the lightning lights it from inside as blooms of light, no line
  float around_ = asin(across);
  vec2 sq = vec2(around_*1.25 + b.y*0.13 - uClock*0.85, b.y*0.06 - uClock*0.22);
  float warp = rxFlow(sq*0.7 + vec2(3.0, 1.0));
  float cloud = part(0.0) > 0.5 ? rxFlow(sq + vec2(1.1*warp, 0.7*warp)) : 0.5;
  vec3 col = mix(BY_BLACK + BY_VIOLET*0.16, BY_VIOLET*1.1, smoothstep(0.22, 0.72, cloud));
  col = mix(col, BY_PALE*1.05, smoothstep(0.6, 0.95, cloud)*0.75);
  col *= 0.55 + 0.65*nz;
  col += BY_VIOLET*pow(1.0 - nz, 2.5)*0.5;
  if (part(1.0) > 0.5){
    col += BY_BOLT*pow(nz, 3.0)*(0.14 + 0.1*sin(uClock*1.3))*(1.0 + sg);
    for (int i = 0; i < 3; i++){
      float k = float(i);
      float cyc = uClock*(0.31 + 0.09*k) + k*0.37;
      float q = fract(cyc);
      float yy = uLength*(0.15 + 0.78*h11(floor(cyc)*3.7 + k));
      float env = smoothstep(0.0, 0.07, q)*(1.0 - smoothstep(0.07, 0.7, q));
      col += mix(BY_BOLT, BY_WHITE, cloud)*halo(abs(b.y - yy), 3.0 + 2.0*half)*env*(0.5 + 0.9*cloud)*nz;
    }
    col += BY_WHITE*sg*0.3*(0.5 + 0.5*cloud);
  }
  col += BY_WHITE*halo(max(b.y, 0.0), 0.5)*0.45;
  col = mix(col, BY_WHITE, 0.5*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t;
  float aft = by_after();
  float sg = by_surge();
  // three great arms of the storm winding up and out, and lightning leaping from them
  if (part(2.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 9.0){
    for (int i = 0; i < 3; i++){
      float k = float(i), f2, sa;
      float bWs = 0.0;
      float best = 1e3, bf = 0.0;
      vec3 prev = around(0.7, k/3.0 - 0.06*uClock, L*0.08);
      for (int j = 1; j <= 22; j++){
        float g = float(j)/22.0;
        vec3 q = around(0.7 + (4.2 + 1.2*sg)*g*g, k/3.0 + 0.9*g - 0.06*uClock, L*(0.08 + 0.9*g));
        float d = raySeg(ro, rd, prev, q, sa);
        { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(j) - 1.0 + sa)/22.0 - bf)*wN/bWs; }
        prev = q;
      }
      float wv = 0.5 + 0.5*sin(bf*8.0 - uClock*1.8 + k*2.0);
      fx += (BY_PALE*halo(best, 0.2)*(0.25 + 0.4*wv) + BY_VIOLET*(halo(best, 0.42)*0.6 + halo(best, 1.0)*0.25))*smoothstep(0.0, 0.08, bf)*(1.0 - smoothstep(0.82, 1.0, bf))*aft;
    }
    // (update 28) its lightning takes the wearer's colour with the blade
    uTurnFx += (1.0 + 0.8*sg)*liveArcs(ro, rd, px, L*0.3, L*0.97, 4.0, 2.8, 0.5, 37.0, BY_BOLT)*aft*uFx*hiltFade(ro, rd);
  }
  // the wind: the storm's air, opening with the funnel
  if (part(4.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 9.0) fx += (0.5 + 0.7*sg)*syMistSpin(ro, rd, L*0.25, L, 1.2, 5.6, 0.05, 90.0, BY_VIOLET*0.9, BY_BLACK + BY_VIOLET*0.2)*aft;
  // dust and small things drawn up the spiral, a storm of specks widening with the funnel
  if (part(4.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 8.5){
    for (int i = 0; i < 26; i++){
      float k = float(i);
      float cyc = uClock*(0.12 + 0.06*h11(k*2.3)) + h11(k*6.1);
      float n = floor(cyc), q = fract(cyc);
      float rr = 0.7 + 5.0*q*q;
      vec3 p = around(rr, h11(n*5.3 + k) + 2.6*q*q, L*(0.04 + 0.96*q));
      float d = rayPt(ro, rd, p, t);
      fx += (BY_PALE*halo(d, max(0.05, px))*1.0 + BY_VIOLET*halo(d, 0.22)*0.28)*sin(3.1416*q)*roomFade(rr)*aft*(1.0 + sg);
    }
  }
  // pulses of pale light climbing the funnel's mouth, felt from the crown
  if (part(0.0) > 0.5 && fxNear(ro, rd, L*0.3, L) < 4.0){
    float pl = fract(uClock*0.33);
    float yp = L*(0.15 + 0.85*pl);
    fx += BY_PALE*halo(rayPt(ro, rd, vec3(0.0, yp, 0.0), t), 0.5 + 1.4*pl)*sin(3.1416*pl)*0.32*aft;
  }
  // the eye of the storm
  if (part(3.0) > 0.5){
    vec3 eye = vec3(0.0, L*0.97, 0.0);
    float d = rayPt(ro, rd, eye, t);
    float br = 0.8 + 0.2*sin(uClock*3.1)*sin(uClock*1.7);
    fx += (BY_WHITE*halo(d, 0.42)*1.1 + BY_PALE*halo(d, 1.3)*0.4)*br*(1.0 + sg)*aft;
  }
  // lightning leaping from the crown when it surges
  if (part(1.0) > 0.5 && sg > 0.02 && fxNear(ro, rd, L*0.4, L) < 7.0){
    for (int i = 0; i < 3; i++){
      float k = float(i);
      float cyc = uClock*(0.7 + 0.2*k) + k*0.3;
      float n = floor(cyc), q = fract(cyc);
      if (q > 0.4) continue;
      vec3 a = around(1.0, h11(n*3.3 + k), L*(0.72 + 0.22*h11(n*1.3 + k)));
      vec3 c = around(4.5 + 1.5*h11(n*2.7 + k), h11(n*3.3 + k) + 0.05, a.y + (h11(n*5.1 + k) - 0.6)*5.0);
      float d = bolt3(ro, rd, a, c, n*7.7 + k, 0.2, px);
      uTurnFx += (BY_WHITE*halo(d, max(0.05, px))*1.5 + BY_BOLT*halo(d, 0.26)*0.6)*(1.0 - q/0.4)*sg*roomFade(5.5)*uFx*hiltFade(ro, rd);
    }
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += (BY_WHITE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 1.0)*1.3 + BY_VIOLET*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 3.0)*0.35)*cl;
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
vec3 sig_glow_d(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(0.600, 0.300, 1.000);
const vec3 SX_DEEP = vec3(0.200, 0.050, 0.500);
const vec3 SX_PALE = vec3(0.850, 0.780, 1.000);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += vec3(0.0); a += 0.7*syMistSpin(ro, rd, L*0.2, L*0.98, 1.4, 5.0, 0.03, 152.0, vec3(0.2, 0.14, 0.4), vec3(0.04, 0.03, 0.1));
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  float L = uLength, C = uClock;
  vec4 n = (vec4(0.0))*rxLife(0.45, 1.3, 0.4)*uFx*hiltFade(ro, rd);
  return vec4(mix(o.rgb, n.rgb, n.a), max(o.a, n.a));
}
// <<< update 34
// >>> dim: less glow, so its effects and body can be seen
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){ return sig_glow_d(g, b, r, px)*0.8; }
vec3 sig_core(vec3 c, vec2 b, float px){ return sig_core_d(c, b, px)*1.0; }
// <<< dim

// >>> update 52: this blade's great effects in the air
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  a += 1.5*xDust(ro, rd, 0.9, 5.4, 3.2, C*0.22, L*0.03, L*0.97, 1.1, 501.0, vec3(0.92, 0.82, 1.0), vec3(0.42, 0.18, 1.0));
  a += 1.2*xDust(ro, rd, 0.7, 4.6, -2.6, C*0.17 + 0.4, L*0.03, L*0.97, 0.9, 502.0, vec3(0.75, 0.6, 1.0), vec3(0.2, 0.05, 0.6));
  a += 1.05*syMistSpin(ro, rd, L*0.2, L*0.98, 1.2, 5.8, 0.12, 503.0, vec3(0.2, 0.08, 0.5), vec3(0.03, 0.01, 0.1));
  a += 1.0*xPuffs(ro, rd, 1.0, 5.2, 2.4, C*0.14, L*0.05, L*0.97, 0.9, 504.0, vec3(0.5, 0.3, 0.9), vec3(0.08, 0.03, 0.2));
  return base + a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<< update 52
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  if (part(5.0) > 0.5){
    col += vec3(0.0);
    col += vec3(0.0);
  }
  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol_m26(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.41*vec3(0.0);
  a += 1.25*gxThunderheads(ro, rd, px, 4.0, 5.2, 14.0, vec3(0.850, 0.780, 1.000), vec3(0.600, 0.300, 1.000));
  a += 1.05*gxComets(ro, rd, px, 3.0, 4.6, 0.04, 2.2, 15.0, vec3(0.850, 0.780, 1.000), vec3(0.600, 0.300, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(52.0, 19.0, 28.0, 33.0, 42.0), L = uLength;
  a += 1.3*gxField(ro, rd, px, 14.0, m, 5.6, 2.0, 1.1, 0.62, 0.0, 0.3, 1110.0, vec3(0.850, 0.780, 1.000), vec3(0.600, 0.300, 1.000));
  a += m*1.4*gxThunderheads(ro, rd, px, 5.0, 3.0, 1111.0, vec3(0.850, 0.780, 1.000), vec3(0.600, 0.300, 1.000));
  return base*1.0 + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

