// Atollon Gathering: the energy of everything round it, lent to one blade. Pale sky-blue streams of
// energy curve in from all round and flow into the blade, motes of light drift slowly inward to
// it, and over a slow cycle its upper half fills with what it gathers, glowing brighter and
// pulsing as it gathers, until it lets it go in a soft wave of light down the blade and a gentle
// ring outward. Inside it light swirls slowly up. Nothing gathers on its tip: it all goes into
// the blade itself. No figures.
// @part Inflowing streams
// @part Drifting motes
// @part Gathering
// @part Release
// @part Inner swirl
// @part Gathered lights
// @raycache fast
// @trail starlight
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.3,0.6,0.95],"corona.rim_hot":[0.6,0.85,1.0],"corona.inner":[0.88,0.96,1.0],"core.white":[0.97,1.0,1.0],"core.fringe_cool":[0.45,0.75,1.0],"core.fringe_hot":[0.75,0.92,1.0],"light.color":[0.6,0.85,1.0],"trail":[0.6,0.85,1.0]}
// @speed 2.6

const vec3 GS_SKY = vec3(0.55, 0.82, 1.0);
const vec3 GS_PALE = vec3(0.85, 0.95, 1.0);
const vec3 GS_DEEP = vec3(0.2, 0.45, 0.9);
const vec3 GS_WHITE = vec3(0.98, 1.0, 1.0);

// The gathering on a 22-second cycle: how full the blade's upper half is (0 to 1, rising
// slowly over fourteen seconds) and the release (0 to 1 over its wave), smooth both ways.
float gs_full(out float release){
  release = -1.0;
  if (part(2.0) < 0.5) return 0.0;
  float c = mod((uTime*0.5) + uSeed*4.0, 22.0);
  float full = smoothstep(2.0, 16.0, c)*(1.0 - smoothstep(16.0, 17.5, c));
  if (part(3.0) > 0.5 && c >= 16.0 && c < 19.5) release = (c - 16.0)/3.5;
  return full;
}
float gs_half(float y){
  float tip = clamp((uLength + 0.3 - y)/2.0, 0.0, 1.0);
  return 0.5*sqrt(tip);
}

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float release;
  float full = gs_full(release);
  float top = uLength + 0.3;
  float y = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - y)) - gs_half(y), 0.0);
  float upper = smoothstep(uLength*0.4, uLength*0.65, b.y);
  // (update 16) a steady speed: one that followed the moment jumped by the blade's whole age
  float pulse = 0.5 + 0.5*sin(uTime*0.6);
  vec3 col = recol(g, GS_SKY)*0.55;
  // effect 3: the gathering, the blade's upper half glowing brighter and pulsing as it fills
  col += (GS_SKY*halo(d, 0.9 + 0.8*full) + GS_PALE*halo(d, 0.25)*0.6)*upper*full*(0.6 + 0.4*pulse);
  // effect 4: the release, a soft wave of light rolling down the blade from its upper half
  if (release >= 0.0){
    float wy = mix(uLength*0.85, -1.0, release);
    float fade = (1.0 - release)*smoothstep(0.0, 0.08, release);
    col += (GS_WHITE*halo(abs(b.y - wy), 1.2)*halo(d, 1.0) + GS_SKY*halo(abs(b.y - wy), 3.0)*halo(d, 2.5)*0.5)*fade;
  }
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.5; }

// effect 5: the inner swirl, light swirling slowly up inside it, brighter in its upper half as
// it fills
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = uLength + 0.3;
  if (b.y < -0.2 || b.y > top) return o;
  float half = gs_half(b.y);
  float x = abs(b.x);
  float cov = fillD(x - half, px)*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float release;
  float full = gs_full(release);
  vec3 col = mix(GS_PALE*1.05, GS_SKY, smoothstep(0.3, 1.0, u));
  if (part(4.0) > 0.5){
    float sw = 0.5 + 0.5*sin(b.x*5.0 + b.y*0.6 - (uTime*0.4)*1.4 + 2.0*sin(b.y*0.15 - (uTime*0.4)*0.5));
    col = mix(col, GS_DEEP*1.1, pow(sw, 3.0)*0.35);
    col += GS_WHITE*pow(sw, 10.0)*0.4;
  }
  float upper = smoothstep(uLength*0.4, uLength*0.65, b.y);
  col = mix(col, GS_WHITE, upper*full*0.45);
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// effects 1 and 2: streams of energy curving in from all round and flowing into the blade's
// upper half (light running along them inward), and motes drifting slowly in to it
// The 3D structure round it (1/11/2026, Creyon: the lower tiers filled out so they are never
// plain, each its own shape): the gathering: countless motes of pale light drifting in from far round it and falling into the blade, thin rays of light reaching in after them, a pale haze drawn in round the emitter. Lit, it grows in; let go, it fades.
vec3 cx_more(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  float aft = rxLife(0.2, 1.0, 0.6);
  if (part(0.0) < 0.5 || aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  {

    fx += 1.2*syMistSpin(ro, rd, 0.3, L*0.95, 6.0, 1.5, 0.03, 3.0, GS_PALE, GS_SKY*0.4);
    float t, s;
    for (int i = 0; i < 24; i++){
      float k = float(i);
      float cyc = tCalm()*(0.15 + 0.08*h11(k*2.3)) + h11(k*7.1);
      float n = floor(cyc), q = fract(cyc);
      float a = h11(n*3.7 + k);
      float r = mix(5.5 + 0.8*h11(n*2.9 + k), 0.5, q*q);
      vec3 p = around(r, a, L*(0.1 + 0.8*h11(n*5.1 + k)));
      float life = smoothstep(0.0, 0.25, q)*(1.0 - smoothstep(0.9, 1.0, q));
      fx += (GS_WHITE*halo(rayPt(ro, rd, p, t), max(0.06, px))*1.4 + GS_SKY*halo(rayPt(ro, rd, p, t), 0.35)*0.3)*life*roomAt(p);
      if (i < 8){
        vec3 tail = around(r + 1.8*(1.0 - q), a, p.y);
        float d = raySeg(ro, rd, p, tail, s);
        fx += GS_PALE*halo(d, max(0.03, px))*(1.0 - s)*life*0.6*roomAt(tail);
      }
    }
  }
  return fx*aft;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, s, t;
  if (fxNear(ro, rd, 0.0, L + 2.0) > 8.5) return fx + cx_more(ro, rd, px)*uFx;
  float release;
  float full = gs_full(release);
  if (part(0.0) > 0.5){
    for (int i = 0; i < 5; i++){
      float k = float(i);
      float cyc = (uTime*0.4)*0.09 + h11(k*3.3);
      float n = floor(cyc), q = fract(cyc);
      float life = smoothstep(0.0, 0.2, q)*(1.0 - smoothstep(0.75, 1.0, q));
      float a = h11(n*2.1 + k*1.7);
      float yIn = L*(0.5 + 0.4*h11(n*4.7 + k));
      float bWs = 0.0;
      float best = 1e3, bf = 0.0;
      vec3 prev = around(7.0, a, yIn - 3.0 + 6.0*h11(n + k));
      for (int j = 1; j <= 16; j++){
        float f = float(j)/16.0;
        float e = f*f*(3.0 - 2.0*f);
        vec3 p = around(mix(7.0, 0.55, e), a + 0.18*e, mix(yIn - 3.0 + 6.0*h11(n + k), yIn, e));
        float dd = raySeg(ro, rd, prev, p, s);
        { float wN = bW(dd); bWs += wN; best = min(best, dd); bf += ((float(j) - 1.0 + s)/16.0 - bf)*wN/bWs; }
        prev = p;
      }
      float run = pow(0.5 + 0.5*sin(bf*14.0 - (uTime*0.4)*3.0 - k), 6.0);
      float taper = smoothstep(0.0, 0.35, bf);
      fx += (GS_PALE*halo(best, max(0.06, px))*(0.4 + 1.0*run) + GS_SKY*halo(best, 0.35)*0.35)*life*taper*(0.6 + 0.6*full);
    }
  }
  if (part(1.0) > 0.5){
    for (int i = 0; i < 18; i++){
      float k = float(i);
      float cyc = (uTime*0.4)*(0.1 + 0.07*h11(k*2.9)) + h11(k*5.3);
      float n = floor(cyc), q = fract(cyc);
      float e = q*q;
      vec3 p = around(mix(6.0 + 1.5*h11(n + k), 0.6, e), h11(n*3.1 + k) + 0.1*q, L*(0.35 + 0.6*h11(n*6.7 + k)) + (1.0 - e)*1.5);
      float d = rayPt(ro, rd, p, t);
      float life = smoothstep(0.0, 0.2, q)*(1.0 - smoothstep(0.85, 1.0, q));
      fx += (GS_WHITE*halo(d, max(0.06, px))*1.3 + GS_SKY*halo(d, 0.3)*0.35)*life*roomAt(p);
    }
  }
  // the release: a soft flare of gathered light along the blade (no ring)
  if (release >= 0.0){
    float d = fxNear(ro, rd, L*0.2, L*0.8);
    fx += (GS_PALE*halo(d, 0.5 + 1.5*release)*0.9 + GS_SKY*halo(d, 2.0 + 2.5*release)*0.3)*sin(3.1416*min(release*2.0, 1.0))*(1.0 - release)*roomFade(d);
  }
  return (fx*uFx) + cx_more(ro, rd, px)*uFx;
}

// ---- update 13: its floating layer (tools/sw_extras.py) ----
const vec3 XM = vec3(0.6, 0.85, 1.0);
const vec3 XH = vec3(0.86, 0.948, 1.0);
const vec3 XA = vec3(0.6, 0.85, 1.0);
const vec3 XD = vec3(0.18, 0.255, 0.3);
vec3 sw_extra(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  fx += 1.0*shStrands(ro, rd, 1.25, 1.0, uLength - 0.6, 3.0, 0.02, 0.03, 0.14, 2.5, 81.0, XH, XM);
  for (int ci = 0; ci < 2; ci++){
    float k = float(ci);
    fx += 1.0*rxComet(ro, rd, px, 2.6 + 0.8*k, uLength*(0.12 + 0.22*k), uLength*0.35, 11.0 + 3.0*k, k + 82.0, XH, XA);
  }
  fx += 1.0*shFlakes(ro, rd, 2.4, 1.5, uLength - 0.6, 12.0, 2.8, -0.7, 0.02, 0.16, 0.5, 0.45, 83.0, XH, XA);
  return fx;
}

vec3 sig_vol_raw(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(5.0) > 0.5) fx += gs_lights(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  return fx*hiltFade(ro, rd);
}

// Gathered lights (update 19): large soft spheres of the sky's light drift in from far round the
// blade, each with a slow bright heart, climbing a long curve; each is drawn out toward the blade as
// it nears, a thread of light joining them, and the blade flares where it pours in.
vec3 gs_lights(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return acc;
  float t, s;
  for (int i = 0; i < 6; i++){
    float k = float(i);
    float cyc = uClock*(0.09 + 0.03*h11(k*3.7)) + h11(k*6.1);
    float n = floor(cyc), q = fract(cyc);
    float e = q*q*(3.0 - 2.0*q);
    float a = h11(n*2.9 + k) + 0.55*e;
    float y0 = uLength*(0.1 + 0.5*h11(n*5.3 + k));
    vec3 c = around(mix(6.4, 0.6, e), a, y0 + 8.0*e);
    float R = (0.55 + 0.5*h11(n*1.7 + k))*(1.0 - 0.55*e);
    float life = smoothstep(0.0, 0.14, q)*(1.0 - smoothstep(0.88, 1.0, q));
    float d = rayPt(ro, rd, c, t);
    float br = 0.8 + 0.2*sin(uClock*2.3 + k*2.0);
    acc += (GS_WHITE*halo(d, max(R*0.42, px))*1.25 + GS_SKY*halo(d, R*1.5)*0.55 + GS_DEEP*halo(d, R*3.6)*0.16)*br*life*roomFade(mix(6.4, 0.6, e));
    // the thread to the blade, once it is near
    float near = smoothstep(0.45, 0.8, e)*life;
    if (near > 0.01){
      float dt = raySeg(ro, rd, c, around(0.45, a + 0.05, c.y + 1.5), s);
      acc += (GS_PALE*halo(dt, max(0.05, px))*0.9 + GS_SKY*halo(dt, 0.25)*0.3)*near*sin(3.1416*clamp(s, 0.0, 1.0));
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
// (update 28, Creyon: its effects stopped short of the tip) the gathering reaches the point
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_raw(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength;
  vec3 up = vec3(0.0);
  for (int i = 0; i < 3; i++){
    float k = float(i);
    up += 1.1*xBeads(ro, rd, 2.6 - 0.4*k, 0.95, 0.5, k/3.0 + 0.01*uTime, L*0.5, L*0.99, 0.06, k + 3.0, GS_WHITE, GS_SKY);
  }
  up += 1.2*gxKin(ro, rd, 3.0, 9.0, 3.20, 0.080, 0.12, 0.08, 0.96, -1.0, 7078.0, GS_WHITE, GS_SKY);
  float s;
  raySeg(ro, rd, vec3(0.0), vec3(0.0, L + 4.0, 0.0), s);
  return fx + up*rxLife(0.3, 1.0, 0.4)*uFx*tipFade(ro, rd);
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
const vec3 SX_HOT = vec3(0.600, 0.850, 1.000);
const vec3 SX_DEEP = vec3(0.300, 0.600, 0.950);
const vec3 SX_PALE = vec3(0.880, 0.960, 1.000);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 0.0, 7.0, 4.00, 0.060, 0.25, 0.08, 0.96, -1.0, 7091.0, vec3(1.0), SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(5.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.54, 4.95, -0.011, 519.0, vec3(0.600, 0.850, 1.000)*0.24, vec3(0.300, 0.600, 0.950)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.600, 0.850, 1.000)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
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
  a += 1.58*gxKin(ro, rd, 5.0, 4.0, 3.40, 0.030, 0.80, 0.08, 0.96, 1.0, 9034.0, vec3(0.880, 0.960, 1.000), vec3(0.600, 0.850, 1.000));
  a += 1.15*gxSwarm(ro, rd, px, 11.0, 4.6, 31.0, vec3(0.880, 0.960, 1.000), vec3(0.600, 0.850, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

