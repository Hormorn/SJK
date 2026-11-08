// Peridea Convergence: the meeting of the two halves of the Force at the edge of the galaxy, red
// that pulls and blue that pushes, and what is born where they meet. Its own body is split down its
// length near the hand, red on one side and blue on the other, the two colours swirling into each
// other as they climb until its upper part is a deep, imaginary violet with a void-dark heart. Two
// broad streams of energy, one red and one blue, spiral up round it from opposite sides and meet
// two thirds up, where space bends: a violet haze turns slowly round the upper blade and violet
// lightning crawls through it. Every 24 seconds the convergence: the streams surge, the violet
// blazes white, a burst of violet sparks is thrown out and the bent space swells, then settles.
// Lit, a red and a blue stream race up the blade and meet in a violet flash; let go, the violet
// collapses down into the emitter. A clash is a violet flash. Its trail is the convergence
// (`@trail convergence`). No figures, no rings.
// @part Split body
// @part Twin streams
// @part The imaginary
// @part Bent space
// @part Convergence
// @part Annihilation
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail convergence
// @palette {"corona.rim_cool":[0.3,0.05,0.6],"corona.rim_hot":[0.75,0.35,1.0],"corona.inner":[0.92,0.8,1.0],"core.white":[1.0,0.96,1.0],"core.fringe_cool":[0.55,0.15,0.9],"core.fringe_hot":[0.85,0.6,1.0],"light.color":[0.6,0.25,1.0],"trail":[0.6,0.25,1.0]}
// @speed 5.0

const vec3 PC_RED = vec3(1.0, 0.12, 0.18);
const vec3 PC_BLUE = vec3(0.15, 0.4, 1.0);
const vec3 PC_VIOLET = vec3(0.58, 0.16, 1.0);
const vec3 PC_VOID = vec3(0.04, 0.0, 0.08);
const vec3 PC_WHITE = vec3(1.0, 0.95, 1.0);

float pc_top(){ return (uLength + 0.4)*rxLife(0.0, 0.35, 0.7); }
float pc_after(){ return rxLife(0.25, 1.0, 0.6); }
// the convergence on its 24-second cycle: 0 to 1, smooth both ways
float pc_surge(){ return part(4.0) > 0.5 ? rxCycle(24.0, 15.0, 17.0, 18.5, 21.5)*pc_after() : 0.0; }
float pc_meet(){ return uLength*0.62; }
// how far up the blade it has become violet (0 at the split, 1 above the meeting)
float pc_mix(float y){ return smoothstep(uLength*0.2, pc_meet(), y); }
vec3 pc_hue(float y, float side){
  vec3 c = mix(PC_RED, PC_BLUE, smoothstep(-0.4, 0.4, side));
  return mix(c, PC_VIOLET, pc_mix(y));
}

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = pc_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half = rxHalf(clamp(b.y, 0.0, top), top, 0.6, 1.0);
    float d = rxOut(b, top, half);
    float side = clamp(b.x*2.0, -1.0, 1.0);
    float sg = pc_surge();
    vec3 hue = pc_hue(b.y, side);
    col = hue*halo(d, 0.55 + 0.3*sg)*0.95 + mix(hue, PC_VIOLET, 0.5)*halo(d, 1.6)*0.45 + PC_WHITE*halo(d, 0.15)*0.45;
    // effect 3 (with the imaginary): the upper blade's violet light pulsing slowly
    if (part(2.0) > 0.5) col += PC_VIOLET*halo(d, 1.0 + 0.8*sg)*pc_mix(b.y)*(0.35 + 0.25*sin(tCalm()*1.2) + 0.8*sg)*pc_after();
  }
  col += lvl(g)*mix(PC_RED, PC_BLUE, 0.5)*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.5;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*vec3(0.8, 0.5, 1.0)*0.25*(1.0 - smoothstep(0.3, 2.0, b.y)); }

// effect 1: the split body, red on one side and blue on the other near the hand, the two swirling
// into each other up it into violet; effect 3: the imaginary's void-dark heart in its upper part
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = pc_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = rxHalf(clamp(b.y, 0.0, top), top, 0.6, 1.0);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float swirl = rxFlow(vec2(b.x*2.0 + b.y*0.25, b.y*0.3 - tCalm()*0.6));
  float side = clamp(b.x*2.0 + (swirl - 0.5)*1.6*pc_mix(b.y)*2.0, -1.0, 1.0);
  vec3 col = part(0.0) > 0.5 ? pc_hue(b.y, side) : PC_VIOLET;
  col = mix(col*1.1, PC_WHITE, (1.0 - smoothstep(0.0, 0.4, u))*0.55);
  if (part(2.0) > 0.5){
    float voidHeart = pc_mix(b.y)*(1.0 - smoothstep(0.0, 0.55, u))*(0.6 + 0.4*rxFlow(vec2(b.y*0.4 - tCalm()*0.5, 3.0)));
    col = mix(col, PC_VOID, voidHeart*0.75*(1.0 - pc_surge()));
    col += PC_VIOLET*halo(abs(u - 0.6), 0.15)*pc_mix(b.y)*0.6;
  }
  col = mix(col, PC_WHITE, 0.6*pc_surge()*pc_mix(b.y));
  col += PC_WHITE*halo(max(b.y, 0.0), 0.8)*0.5;
  col = mix(col, PC_WHITE, 0.5*clamp(uClash, 0.0, 1.0));
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t, f;
  float after = pc_after();
  float sg = pc_surge();
  if (fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  // effect 2: the twin streams, a red and a blue spiralling up from opposite sides and meeting
  // where space bends; lit, they race up to meet
  if (part(1.0) > 0.5){
    float race = uAge < 2000.0 ? smoothstep(0.0, 1.0, uAge) : 1.0 - smoothstep(0.0, 0.5, uAge - 2000.0);
    float meet = pc_meet()*max(race, 0.05);
    for (int i = 0; i < 2; i++){
      float k = float(i);
      vec3 c = k < 0.5 ? PC_RED : PC_BLUE;
      fx += (1.2 + 0.8*sg)*xBeads(ro, rd, 3.2, 0.5, 0.75, k*0.5 + 0.0032*tCalm(), L*0.04, meet, 1.3, k + 7.0, mix(c, PC_WHITE, 0.4), c)*race;
    }
  }
  // effect 4: bent space, a violet haze turning slowly round the upper blade
  if (part(3.0) > 0.5 && after > 0.01) fx += (1.2 + 1.4*sg)*syMistSpin(ro, rd, pc_meet() - 6.0, L + 0.2, 2.0, 4.6 + sg, 0.02, 22.0, PC_VIOLET, PC_VOID + PC_VIOLET*0.15)*after;
  // effect 5: violet lightning crawling through the bent space
  if (part(3.0) > 0.5 && after > 0.01){
    for (int i = 0; i < 3; i++){
      float k = float(i);
      float cyc = uTime*(0.2 + 0.08*k) + h11(k*5.1);
      float n = floor(cyc), q = fract(cyc);
      if (q > 0.5) continue;
      float life = smoothstep(0.0, 0.06, q)*(1.0 - smoothstep(0.25, 0.5, q));
      float a = h11(n*2.3 + k);
      vec3 pa = around(1.0, a, pc_meet() - 2.0 + 6.0*h11(n*3.1 + k));
      vec3 pc = around(3.5 + h11(n*4.7 + k), a + 0.15, min(pa.y + 3.0, L - 0.5));
      float d = rxBolt(ro, rd, pa, pc, n*6.1 + k, 0.35, 0.5, f);
      fx += (PC_WHITE*halo(d, max(0.04, px))*1.3 + PC_VIOLET*halo(d, 0.35)*0.6)*life*(1.0 + sg)*roomAt(pc)*after;
    }
  }
  // effect 6: the convergence's burst of violet sparks from where the streams meet
  {
    float c = mod(uClock + uSeed*7.0, 24.0);
    float q = (c - 17.0)/1.8;
    if (part(4.0) > 0.5) fx += rxSparkShower(ro, rd, px, vec3(0.0, pc_meet(), 0.0), q, 24.0, 6.5, floor((uTime + uSeed*7.0)/24.0), PC_WHITE, PC_VIOLET)*after;
  }
  // effects 7 and 8: lit, the meeting flash; let go, the violet collapsing into the emitter
  if (uAge < 1.6){
    float fl = exp(-pow((uAge - 1.0)/0.18, 2.0));
    vec3 at = vec3(0.0, pc_meet(), 0.0);
    fx += (PC_WHITE*halo(rayPt(ro, rd, at, t), 1.0)*1.6 + PC_VIOLET*halo(rayPt(ro, rd, at, t), 3.5)*0.5)*fl;
  }
  if (uAge >= 2000.0 && uAge < 2000.8){
    float q = (uAge - 2000.0)/0.8;
    vec3 at = vec3(0.0, pc_meet()*(1.0 - q), 0.0);
    fx += (PC_WHITE*halo(rayPt(ro, rd, at, t), 0.6)*1.4 + PC_VIOLET*halo(rayPt(ro, rd, at, t), 2.0)*0.5)*(1.0 - q);
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += (PC_WHITE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 1.0)*1.3 + PC_VIOLET*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 3.0)*0.5)*cl;
  return fx*uFx;
}


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(5.0) > 0.5) fx += x14_peridea_convergence(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  return fx*hiltFade(ro, rd);
}

// effect 6 (1/11/2026, update 14): annihilation, light and its opposite: a white mote and a
// violet one spiral in toward each other beside the blade, closer and faster, and meet in a soft
// white flash that fades; then another pair somewhere else
vec3 x14_peridea_convergence(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return acc;
  float t;
  for (int i = 0; i < 3; i++){
    float k = float(i);
    float cyc = uTime/(7.0 + 2.0*k) + h11(k*4.4);
    float n = floor(cyc), q = fract(cyc);
    vec3 m = around(2.6 + 1.5*h11(n*2.2 + k), h11(n*3.9 + k), uLength*(0.25 + 0.6*h11(n*6.6 + k)));
    float close = smoothstep(0.0, 0.8, q);
    for (int j = 0; j < 2; j++){
      float sg = float(j)*2.0 - 1.0;
      float ang = 6.2832*(1.0 - close)*1.5 + h11(n + k)*6.28;
      vec3 p = m + vec3(cos(ang), 0.35*sin(ang*1.3), sin(ang))*1.8*(1.0 - close)*sg;
      float d = rayPt(ro, rd, p, t);
      vec3 col = mix(vec3(1.0, 0.95, 0.85), vec3(0.6, 0.35, 1.0), float(j));
      acc += (col*halo(d, max(0.08, px))*1.8 + col*halo(d, 0.5)*0.25)*smoothstep(0.0, 0.1, q)*(1.0 - smoothstep(0.78, 0.8, q));
    }
    if (q > 0.8){
      float f = (q - 0.8)/0.2;
      float d = rayPt(ro, rd, m, t);
      acc += (vec3(1.0)*halo(d, 0.2 + 0.8*f)*2.0 + vec3(0.7, 0.5, 1.0)*halo(d, 1.5 + 2.0*f)*0.4)*(1.0 - f)*(1.0 - f);
    }
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
const vec3 SX_HOT = vec3(0.750, 0.350, 1.000);
const vec3 SX_DEEP = vec3(0.300, 0.050, 0.600);
const vec3 SX_PALE = vec3(0.920, 0.800, 1.000);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 3.0, 7.0, 3.60, 0.050, 0.25, 0.08, 0.96, 1.0, 7884.0, vec3(1.0), vec3(1.0, 0.15, 0.2)); a += gxKin(ro, rd, 3.0, 7.0, 3.60, 0.050, 0.25, 0.08, 0.96, -1.0, 7897.0, vec3(1.0), vec3(0.2, 0.35, 1.0));
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(5.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.54, 4.66, -0.014, 759.0, vec3(0.750, 0.350, 1.000)*0.24, vec3(0.300, 0.050, 0.600)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.750, 0.350, 1.000)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
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
  a += 1.54*vec3(0.0);
  a += 1.2*gxPlanets(ro, rd, px, 3.0, 5.0, 45.0, vec3(0.920, 0.800, 1.000), vec3(0.750, 0.350, 1.000));
  a += 1.0*gxMeteors(ro, rd, px, 3.0, 4.6, 46.0, vec3(0.920, 0.800, 1.000), vec3(0.750, 0.350, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

