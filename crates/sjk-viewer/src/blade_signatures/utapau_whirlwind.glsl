// Utapau Whirlwind (rebuilt 2/11/2026, update 24; Creyon: a weird white flicker, from the many bright
// strands and glints, all gone): the winds of Utapau's sinkholes wound into a blade. The body IS a tornado:
// a rope at the hand, opening to a funnel high up and closing to a point, its grey-blue walls banded and
// spinning, dust-coloured low down and pale at the top, lit by the sky on the side that faces you. Short
// streaks of wind run round the funnel, dust hangs in the gale, the air off it shivers. Every 16 seconds
// the spiral tightens: the walls spin faster, the streaks widen, then it eases.
// @part Spinning walls
// @part Wind streaks
// @part Caught dust
// @part Tightening
// @part Air shimmer
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail gale
// @palette {"corona.rim_cool":[0.25,0.6,0.75],"corona.rim_hot":[0.65,0.95,1.0],"corona.inner":[0.92,1.0,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.4,0.8,0.9],"core.fringe_hot":[0.85,0.98,1.0],"light.color":[0.65,0.95,1.0],"trail":[0.65,0.95,1.0]}
// @speed 2.0

const vec3 UG_AIR = vec3(0.68, 0.9, 0.97);
const vec3 UG_TEAL = vec3(0.2, 0.46, 0.58);
const vec3 UG_DEEP = vec3(0.08, 0.15, 0.22);
const vec3 UG_DUST = vec3(0.62, 0.5, 0.36);

float ug_tight(){ return part(3.0) > 0.5 ? rxCycle(16.0, 9.0, 11.0, 13.0, 15.5) : 0.0; }
float ug_tightI(){ return part(3.0) > 0.5 ? rxCycleInt(16.0, 9.0, 11.0, 13.0, 15.5) : 0.0; }
// the tornado's half width: a rope low down, a funnel high up; it leans a little as it turns
float ug_half(float y0){
  float L = uLength;
  float s = clamp(y0/max(L, 1.0), 0.0, 1.0);
  float open = smoothstep(0.12, 0.7, s)*(1.0 - 0.3*ug_tight());
  float close = pow(1.0 - smoothstep(0.7, 1.0, s), 0.8);
  float w = 0.28 + 1.0*open + 0.12*rxFlow(vec2(y0*0.12 - uTime*0.22, 3.0));
  // (update 29) the funnel narrows but its end stays a solid, rounded tip
  float e = 1.0 - clamp((L + 0.2 - y0)/0.9, 0.0, 1.0);
  return mix(0.62, w, close)*sqrt(max(1.0 - e*e, 0.0))*smoothstep(-0.2, 0.8, y0);
}
float ug_lean(float y){
  float s = clamp(y/max(uLength, 1.0), 0.0, 1.0);
  return 0.22*sin(uTime*0.35 + s*2.4)*s;
}
vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float d = rxOut(b - vec2(ug_lean(b.y), 0.0), L, ug_half(b.y));
  float tight = ug_tight();
  vec3 col = UG_AIR*(halo(d, 0.25)*0.4 + halo(d, 1.1)*0.16 + halo(d, 2.6)*0.04);
  // effect 5: air shimmer, faint wavering ripples of light just off it
  if (part(4.0) > 0.5 && uFx > 0.0){
    float w = rxFlow(vec2(b.x*1.6 + uTime*0.3, b.y*0.2 - uTime*0.16));
    col += UG_AIR*w*w*halo(d, 1.7 + tight)*smoothstep(0.1, 0.5, d)*0.14*uFx;
  }
  col += lvl(g)*UG_AIR*(1.0 - smoothstep(0.3, 1.6, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }

// effect 1: the walls. Round a real funnel: where you look across its middle they rush past, at its sides
// they turn away, the bands bending as round a barrel
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float bx = b.x - ug_lean(b.y);
  float half = ug_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.08 - px, half + 0.03 + px, abs(bx)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(bx/max(half, 0.01), -1.0, 1.0);
  float ang = asin(u);
  float s = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  float spin = part(0.0) > 0.5 ? uTime*0.45 + 1.1*ug_tightI() : 0.0;
  // the wall's own streaks, wound round it
  float climb = b.y*0.17;
  vec2 q = vec2(ang*0.8 - spin + climb*2.0, climb - uTime*0.08);
  float wa = rxFlow(q) + 0.5*rxFlow(q*vec2(2.0, 1.7) + 7.0);
  float band = smoothstep(0.45, 1.05, wa);
  vec3 wall = mix(UG_DEEP, mix(UG_TEAL, UG_AIR, smoothstep(0.35, 0.95, s)), band);
  // dust climbing out of the ground it tears at
  wall = mix(wall, UG_DUST*(0.45 + 0.75*band), (1.0 - smoothstep(0.08, 0.42, s))*0.8);
  // round: lit from the sky on its face, dark toward its sides
  float face = sqrt(max(1.0 - u*u, 0.0));
  vec3 col = wall*(0.55 + 1.1*face);
  col += UG_AIR*0.12*pow(1.0 - face, 3.0);
  col += UG_AIR*halo(max(b.y, 0.0), 0.5)*0.35;
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}

vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength;
  float aft = rxLife(0.2, 1.0, 0.6);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L + 1.0) > 9.0) return fx;
  float tight = ug_tight();
    // effect 2 and 4: fine dust drawn into long spirals round the funnel, the wind made visible; wider and
  // quicker as it tightens; never white
  if (part(1.0) > 0.5){
    float whirl = uTime*0.022 + 0.09*ug_tightI();
    for (int i = 0; i < 3; i++){
      float k = float(i);
      float rr = 1.1 + 0.4*k + 0.9*tight;
      fx += 0.85*xDust(ro, rd, rr, rr + 2.0, 1.1, k/3.0 + whirl, L*0.06, L*0.96, 0.1, k + 21.0, UG_AIR, UG_TEAL);
    }
    fx += 0.3*syMistSpin(ro, rd, 0.5, L*0.95, 1.0, 4.2 + tight, 0.03, 9.0, UG_TEAL*0.6, UG_DEEP);
  }
  // three great arms of wind winding up the funnel, turning with it
  if (part(0.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 6.5){
    for (int i = 0; i < 3; i++){
      float k = float(i), sa;
      float bWs = 0.0;
      float best = 1e3, bf = 0.0;
      float w0 = uTime*0.05 + 0.12*ug_tightI();
      vec3 prev = around(0.6, k/3.0 + w0, L*0.04);
      for (int j = 1; j <= 22; j++){
        float g = float(j)/22.0;
        vec3 q = around(0.6 + (3.2 + 0.8*tight)*g*g, k/3.0 + 1.3*g + w0, L*(0.04 + 0.92*g));
        float d = raySeg(ro, rd, prev, q, sa);
        { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(j) - 1.0 + sa)/22.0 - bf)*wN/bWs; }
        prev = q;
      }
      fx += (UG_AIR*halo(best, max(0.06, px))*0.55 + UG_TEAL*(halo(best, 0.35)*0.7 + halo(best, 1.1)*0.25))*smoothstep(0.0, 0.1, bf)*(1.0 - smoothstep(0.8, 1.0, bf));
    }
  }
  // dust thrown off the funnel in slow clouds, stones of light caught high in it, thin lightning in the storm
  if (part(1.0) > 0.5){
    for (int i = 0; i < 4; i++){
      float k = float(i);
      float y0 = L*(0.03 + 0.22*k);
      fx += 0.55*xPuffs(ro, rd, 1.0 + 0.5*k, 2.8 + 0.7*k, 0.5, k*0.27 + 0.02*uTime, y0, y0 + L*0.3, 1.2, k + 41.0, UG_DUST*0.9, UG_TEAL*0.7);
    }
    fx += vec3(0.0);
  }
  // effect 3: caught dust and grit, whirled round in the gale
  if (part(2.0) > 0.5){
    fx += gxKin(ro, rd, 0.0, 10.0, 3.20, 0.200, 0.40, 0.08, 0.96, 1.0, 8183.0, UG_DUST*1.3, UG_DUST*0.5);
    fx += gxKin(ro, rd, 0.0, 8.0, 2.20, 0.160, 0.30, 0.08, 0.96, -1.0, 8196.0, UG_AIR*0.8, UG_TEAL);
  }
  return fx*aft*uFx*hiltFade(ro, rd);
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
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(0.650, 0.950, 1.000);
const vec3 SX_DEEP = vec3(0.250, 0.600, 0.750);
const vec3 SX_PALE = vec3(0.920, 1.000, 1.000);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 0.9*syMistSpin(ro, rd, L*0.2, L*0.98, 1.5, 6.0, 0.035, 101.0, SX_PALE*0.3, SX_DEEP*0.4); a += 0.9*shStreaks(ro, rd, 2.8, L*0.08, L*0.95, 8.0, 2.6, 2.4, 0.5, 102.0, SX_PALE, SX_HOT*0.5);
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

// >>> update 52: this blade's great effects in the air
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  a += 1.25*syRibbon(ro, rd, px, 1.4, 4.2, 2.6, C*0.2, L*0.03, L*0.97, 0.3, 511.0, vec3(0.85, 1.0, 1.0), vec3(0.2, 0.55, 0.62));
  a += 1.0*syRibbon(ro, rd, px, 2.0, 3.4, -2.2, C*0.16 + 0.35, L*0.03, L*0.97, 0.22, 512.0, vec3(0.7, 0.92, 0.95), vec3(0.14, 0.42, 0.5));
  a += 1.35*xDust(ro, rd, 1.3, 4.6, 2.8, C*0.24, L*0.03, L*0.97, 1.0, 513.0, vec3(0.92, 0.95, 0.85), vec3(0.42, 0.6, 0.58));

  return base + a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<< update 52
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(4.0) > 0.5){
    a += 1.3*syPetalsAir(ro, rd, 10.0, 3.19, 0.038, 1584.0, vec3(0.920, 1.000, 1.000), vec3(0.650, 0.950, 1.000));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  if (part(4.0) > 0.5){
    col += vec3(0.0);
  }
  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol_m26(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.26*vec3(0.0);
  a += 1.25*gxGale(ro, rd, px, 16.0, 5.6, 9.0, vec3(0.920, 1.000, 1.000), vec3(0.650, 0.950, 1.000));
  a += 1.1*gxTumble(ro, rd, px, 9.0, 5.0, 2.0, 1.0, 0.3, 10.0, vec3(0.920, 1.000, 1.000), vec3(0.650, 0.950, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(46.0, 15.0, 23.0, 28.0, 37.0), L = uLength;
  a += 1.35*gxField(ro, rd, px, 16.0, m, 5.6, 1.3, 2.2, 0.7, 1.0, 0.3, 1107.0, vec3(0.920, 1.000, 1.000), vec3(0.650, 0.950, 1.000));
  a += m*1.4*gxGale(ro, rd, px, 16.0, 3.6, 1108.0, vec3(0.920, 1.000, 1.000), vec3(0.650, 0.950, 1.000));
  return base*1.0 + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

