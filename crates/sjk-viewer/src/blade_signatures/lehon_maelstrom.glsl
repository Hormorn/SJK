// Lehon Maelstrom: a tear in space above the Rakata's lost world, bound into a blade that draws
// everything in. Its own body is deep blue-black with a bright cyan edge, its surface flowing
// inward in slow spirals; round its upper half space itself swirls, streams of light spiralling
// slowly in toward the blade, and motes of light are caught in it, spiralling in and vanishing into the
// blade; a cold blue mist is drawn in along it. Every 20 seconds the maelstrom opens: the spiral
// widens and spins faster, the motes rush in, the blade's edge blazes and a flash bursts from where
// they fall in, then it closes. Lit, light spirals in to make it; let go, it spirals out of it and
// disperses. A clash is a blue-white flash with a burst of motes. Its trail is the vortex (`@trail
// vortex`). No figures.
// @part Inward body
// @part Swirling space
// @part Caught motes
// @part Drawn mist
// @part The maelstrom
// @part Stretched starlight
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail vortex
// @palette {"corona.rim_cool":[0.02,0.1,0.45],"corona.rim_hot":[0.2,0.65,1.0],"corona.inner":[0.75,0.92,1.0],"core.white":[0.92,0.98,1.0],"core.fringe_cool":[0.05,0.3,0.8],"core.fringe_hot":[0.45,0.8,1.0],"light.color":[0.2,0.5,1.0],"trail":[0.25,0.6,1.0]}
// @speed 4.5

const vec3 LM_CYAN = vec3(0.3, 0.85, 1.0);
const vec3 LM_BLUE = vec3(0.1, 0.3, 0.95);
const vec3 LM_DARK = vec3(0.01, 0.02, 0.1);
const vec3 LM_WHITE = vec3(0.92, 0.98, 1.0);

float lm_top(){ return (uLength + 0.4)*rxLife(0.0, 0.4, 0.8); }
float lm_after(){ return rxLife(0.3, 1.2, 0.6); }
float lm_open(){ return part(4.0) > 0.5 ? rxCycle(20.0, 11.0, 13.5, 15.5, 18.0)*lm_after() : 0.0; }
// how many seconds it has been open so far (update 16): what its faster turning adds, so the
// swirl speeds up smoothly as it opens instead of racing ahead (Creyon's video)
float lm_openI(){ return part(4.0) > 0.5 ? rxCycleInt(20.0, 11.0, 13.5, 15.5, 18.0) : 0.0; }
float lm_eye(){ return uLength*0.68; }

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = lm_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half = rxHalf(clamp(b.y, 0.0, top), top, 0.52, 1.0);
    float d = rxOut(b, top, half);
    float open = lm_open();
    float after = lm_after();
    col = LM_CYAN*halo(d, 0.45 + 0.3*open)*0.85 + LM_BLUE*halo(d, 1.4)*0.5 + LM_WHITE*halo(d, 0.12)*0.4;
    // effect 4: drawn mist, a cold blue mist drawn in along the blade
    if (part(3.0) > 0.5 && uFx > 0.0){
      float m = rxFlow(vec2(b.x*1.4 - clamp(b.x*2.0, -1.0, 1.0)*uTime*0.15, b.y*0.25 + uTime*0.06));
      col += LM_BLUE*smoothstep(0.5, 0.9, m)*halo(d, 1.6)*smoothstep(0.1, 0.5, d)*0.35*after*uFx;
    }
  }
  col += lvl(g)*LM_CYAN*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.5;
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.9*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core_d(vec3 c, vec2 b, float px){ return c*vec3(0.6, 0.85, 1.0)*0.3*(1.0 - smoothstep(0.3, 2.0, b.y)); }

// effect 1: the inward body, deep blue-black flowing inward in slow spirals, a bright cyan edge
// that blazes when the maelstrom opens
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = lm_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = rxHalf(clamp(b.y, 0.0, top), top, 0.52, 1.0);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float open = lm_open();
  vec3 col = mix(LM_DARK, LM_BLUE*0.7, smoothstep(0.4, 0.9, u));
  if (part(0.0) > 0.5){
    float spiral = 0.5 + 0.5*sin(b.y*0.9 + u*5.0*clamp(b.x*2.0, -1.0, 1.0) + uTime*0.35 + 0.6*lm_openI());
    col += LM_BLUE*pow(spiral, 5.0)*0.6*(1.0 - u*0.5);
  }
  col += LM_CYAN*smoothstep(0.8, 1.0, u)*(1.0 + 1.2*open);
  col += LM_WHITE*halo(abs(b.y - lm_eye()), 1.0)*open*0.8*(1.0 - u);
  col += LM_WHITE*halo(max(b.y, 0.0), 0.8)*0.6;
  col = mix(col, LM_WHITE, 0.55*clamp(uClash, 0.0, 1.0));
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// effect 3: caught motes, points of light spiralling in toward the blade's upper half and
// vanishing into it; rushing in when the maelstrom opens
vec3 lm_motes(vec3 ro, vec3 rd, float px, float open){
  vec3 acc = vec3(0.0);
  float L = uLength, t;
  if (fxNear(ro, rd, 0.0, L + 3.0) > 8.0) return acc;
  for (int i = 0; i < 14; i++){
    float k = float(i);
    float q = fract((uTime + 1.2*lm_openI())*(0.035 + 0.025*h11(k*2.9)) + h11(k*5.3));
    float e = q*q;
    vec3 p = around(mix(6.5 + 1.5*h11(k), 0.5, e), h11(k*3.1) + 0.9*e, lm_eye() + (h11(k*6.7) - 0.5)*8.0*(1.0 - e));
    float tw = 0.7 + 0.3*sin(uClock*2.6 + k);
    acc += (LM_WHITE*halo(rayPt(ro, rd, p, t), max(0.06, px))*1.4 + LM_CYAN*halo(rayPt(ro, rd, p, t), 0.3)*0.4)*tw*smoothstep(0.0, 0.15, q)*(1.0 - smoothstep(0.85, 1.0, q))*roomAt(p);
  }
  return acc;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float after = lm_after();
  float open = lm_open();
  float t;
  if (part(2.0) > 0.5 && after > 0.01) fx += lm_motes(ro, rd, px, open)*after;
  // effect 2: swirling space (1/11/2026, its own shape, and no rings): broad streams of light drawn
  // in from far round the blade, each curving in as it climbs and pouring into the blade, white
  // where it enters, cyan through its body; wider and faster when the maelstrom opens
  if (part(1.0) > 0.5 && after > 0.01 && fxNear(ro, rd, 0.0, uLength) < 9.0){
    fx += (1.2 + 0.8*open)*rxMistSpinP(ro, rd, 0.3, uLength*0.95, 2.5, 5.5, 0.008*(uTime + lm_openI()), 160.0, LM_BLUE, LM_DARK + LM_BLUE*0.15)*after;
    for (int i = 0; i < 9; i++){
      float k = float(i);
      float y0 = uLength*(0.08 + 0.6*h11(k*4.3));
      fx += (1.0 + 0.6*open)*rxRibbon(ro, rd, 5.0 + 1.5*h11(k*2.9), 0.4, -0.3 - 0.3*h11(k*8.1), h11(k*6.7) + 0.007*(uTime + lm_openI()), y0, min(y0 + uLength*0.3, uLength*0.9), 0.9, k + 160.0, LM_WHITE, LM_CYAN)*after;
    }
  }
  // the eye (1/11/2026, a little more for Creyon): where the streams pour in, the blade glows
  // white-blue, breathing slowly, and the vortex round it is lit from within as the light falls in
  if (part(1.0) > 0.5 && after > 0.01){
    float br = 0.75 + 0.25*sin(uClock*1.3);
    vec3 eye = vec3(0.0, lm_eye(), 0.0);
    fx += (LM_WHITE*halo(rayPt(ro, rd, eye, t), 0.9)*0.9 + LM_CYAN*halo(rayPt(ro, rd, eye, t), 3.0)*0.35)*br*after;
  }
  // the flash where they fall in, at the maelstrom's height
  float fl = rxMoment(20.0, 15.0, 1.5);
  if (fl >= 0.0 && part(4.0) > 0.5) fx += (LM_WHITE*halo(rayPt(ro, rd, vec3(0.0, lm_eye(), 0.0), t), 0.8 + 2.0*fl)*1.3 + LM_CYAN*halo(rayPt(ro, rd, vec3(0.0, lm_eye(), 0.0), t), 3.0)*0.4)*(1.0 - fl)*(1.0 - fl)*after;
  // effects 7 and 8: lit, light spirals in to make it; let go, it spirals out and disperses
  if (uAge < 1.4 || (uAge >= 2000.0 && uAge < 2001.2)){
    float q = uAge < 1.4 ? 1.0 - clamp(uAge/1.2, 0.0, 1.0) : clamp((uAge - 2000.0)/1.1, 0.0, 1.0);
    for (int i = 0; i < 16; i++){
      float k = float(i);
      vec3 p = around(0.5 + 5.0*q*(0.5 + 0.5*h11(k*4.1)), h11(k*6.3) + q*0.8, uLength*h11(k*2.9));
      fx += (LM_WHITE*halo(rayPt(ro, rd, p, t), max(0.07, px))*1.4)*sin(3.1416*clamp(q, 0.0, 1.0))*roomAt(p);
    }
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += LM_WHITE*halo(rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), t), 1.0)*cl*1.4;
  return fx*uFx;
}


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(5.0) > 0.5) fx += x14_lehon_maelstrom(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  return fx*hiltFade(ro, rd);
}

// effect 6 (1/11/2026, update 14): stretched starlight, stars far out round the maelstrom pulled
// toward it, drawn out into long thin threads of light as they fall faster and faster, and gone
// into the blade at its eye
vec3 x14_lehon_maelstrom(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return acc;
  float s;
  for (int i = 0; i < 8; i++){
    float k = float(i);
    float q = fract(uTime*(0.05 + 0.02*h11(k*2.1)) + h11(k*6.3));
    float e = q*q;
    float e2 = max(e - (0.03 + 0.25*e), 0.0);
    float a0 = h11(k*3.9 + floor(uTime*(0.05 + 0.02*h11(k*2.1)) + h11(k*6.3)));
    float off = (h11(k*5.1) - 0.5)*10.0;
    vec3 head = around(mix(7.0, 0.5, e), a0 + 0.6*e, lm_eye() + off*(1.0 - e));
    vec3 tail = around(mix(7.0, 0.5, e2), a0 + 0.6*e2, lm_eye() + off*(1.0 - e2));
    float d = raySeg(ro, rd, tail, head, s);
    float life = smoothstep(0.0, 0.1, q)*(1.0 - smoothstep(0.93, 1.0, q));
    acc += (vec3(0.92, 0.98, 1.0)*halo(d, max(0.035 + 0.04*s, px))*(0.4 + 1.2*s) + vec3(0.3, 0.85, 1.0)*halo(d, 0.25)*0.25)*life*roomFade(mix(7.0, 0.5, e));
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
vec3 sig_glow_d(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(0.200, 0.650, 1.000);
const vec3 SX_DEEP = vec3(0.020, 0.100, 0.450);
const vec3 SX_PALE = vec3(0.750, 0.920, 1.000);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  
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
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){ return sig_glow_d(g, b, r, px)*0.5; }
vec3 sig_core(vec3 c, vec2 b, float px){ return sig_core_d(c, b, px)*0.6; }
// <<< dim

// >>> update 52: this blade's great effects in the air
vec3 sig_vol_e3(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 prevT = uTurnFx;
  vec3 base = sig_vol_p1(ro, rd, px)*0.45;
  uTurnFx = prevT + (uTurnFx - prevT)*0.45;
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  a += 1.3*xDust(ro, rd, 1.5, 4.4, 2.2, C*0.16, L*0.05, L*0.97, 1.0, 411.0, vec3(0.85, 1.0, 1.0), vec3(0.15, 0.7, 1.0));
  a += 1.0*syMistSpin(ro, rd, 0.3, L*0.97, 1.3, 5.2, 0.09, 413.0, vec3(0.05, 0.28, 0.5), vec3(0.01, 0.06, 0.16));
  for (int i = 0; i < 6; i++){
    float k = float(i);
    float run = C*(0.5 + 0.08*h11(k + 414.0)) + k*0.29, cyc = floor(run), ph = fract(run);
    float y = L*((k + h11(k*2.3 + cyc*1.3))/6.0)*0.86 + 0.3;
    a += 1.0*sin(3.1416*ph)*syLick(ro, rd, px, y, h11(k*5.7 + cyc*0.7 + 414.0) + 0.08*ph, 1.6 + 1.8*h11(k*3.1 + cyc), 2.0 + 2.5*h11(k*4.3 + cyc), 0.42, k*3.0 + cyc + 414.0, vec3(0.9, 1.0, 1.0), vec3(0.2, 0.75, 1.0), vec3(0.03, 0.2, 0.5));
  }

  uTurnFx += a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return base;
}
// <<< update 52
vec3 sig_glow_e3(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_e3(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  // the great arms of the maelstrom turning slowly about its eye
  if (part(4.0) > 0.5){
    vec2 e0 = vec2(b.x, (b.y - lm_eye())*0.5);
    float r0 = length(e0), th = atan(e0.y, e0.x);
    float arm = pow(0.5 + 0.5*sin(2.0*th + 3.0*log(r0 + 0.3) - C*0.45), 2.4);
    col += mix(LM_CYAN, LM_BLUE*1.1, smoothstep(0.5, 4.0, r0))*arm*(1.0 - smoothstep(1.2, 6.0, r0))*smoothstep(0.3, 1.0, r0)*0.75*on;
  }

  return col;
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e3(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(2.0) > 0.5) a += 1.3*syBubbles(ro, rd, 10.0, 4.2, 0.06, 131.0, LM_WHITE, LM_CYAN);
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}

// >>> what this blade has in the air of its own
vec3 sig_vol_r24(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  // its great moment: what floats round it is drawn in and tightens round the eye, turning ever closer, then lets go
  float mo = rxCycle(52.0, 18.0, 27.0, 32.0, 41.0);
  if (mo > 0.0){
    float turn = rxCycleInt(52.0, 18.0, 27.0, 32.0, 41.0)*0.09, eye = L*0.58;
    for (int i = 0; i < 16; i++){
      float k = float(i), h0 = h11(k*3.1 + 5.0), h1 = h11(k*5.7 + 1.0), h2 = h11(k*8.3 + 2.0);
      float e0 = smoothstep(h0*0.3, 0.7 + h0*0.3, mo);
      vec3 p = around(mix(4.6 + 1.6*h1, 0.95 + 0.7*h2, e0), h2 + uTime*0.04 + turn*(1.0 + h1), mix(L*(0.1 + 0.82*h1), eye + (h0 - 0.5)*7.0, e0));
      a += 1.45*gxPiece(ro, rd, px, p, 0.26 + 0.24*h0, k + 70.0, 0.0, vec3(0.8, 0.96, 1.0), vec3(0.2, 0.55, 1.0))*smoothstep(0.0, 0.2, mo);
    }
    float tm, de = rayPtRaw(ro, rd, vec3(0.0, eye, 0.0), tm);
    a += (vec3(0.85, 0.97, 1.0)*halo(de, 1.2)*0.7 + vec3(0.2, 0.55, 1.0)*halo(de, 3.4)*0.42)*mo*mo;
    a += mo*1.25*gxVortex(ro, rd, px, 14.0, 6.0, 71.0, vec3(0.8, 0.96, 1.0), vec3(0.2, 0.55, 1.0));
  }
  a += 1.5*vec3(0.0);
  a += 1.3*gxVortex(ro, rd, px, 16.0, 6.0, 12.0, vec3(0.750, 0.920, 1.000), vec3(0.200, 0.650, 1.000));
  a += 1.2*gxTumble(ro, rd, px, 11.0, 4.6, 0.0, 0.0, 0.42, 13.0, vec3(0.750, 0.920, 1.000), vec3(0.200, 0.650, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> how it answers: as it lights, in a swing, in a clash
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_r24(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float ig = uAge < 2000.0 ? 1.0 - smoothstep(0.3, 3.0, uAge) : 0.0, sw = clamp(uSwing, 0.0, 1.0);
  a += ig*1.2*gxVortex(ro, rd, px, 14.0, 6.0, 317.0, vec3(0.80, 0.96, 1.00), vec3(0.20, 0.55, 1.00));
  a += 1.35*gxBurst(ro, rd, px, 11.0, 5.0, 0.0, 0.3, 318.0, vec3(0.85, 0.97, 1.00), vec3(0.20, 0.55, 1.00));
  return base + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

