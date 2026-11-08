// Kathol Rift (new 2/11/2026, update 30): the Kathol Rift, the strange storm of space where the Aing-Tii monks walk
// through time. No tube of light at all: the blade is one long ribbon, twisted like a Mobius band, forever turning
// on itself. One face is deep space with slow stars in it; the other is liquid gold. Where it turns edge-on it
// pinches thin and burns with every colour. Time is wrong around it: the twist runs on, slows, stops and runs
// backward; ghosts of the ribbon as it was a moment ago hang beside it; two streams of light wind round it the
// opposite way; motes fall upward and then back down. Lit, the ribbon unscrews out of the hilt; let go, it winds back.
// @part Twisted ribbon
// @part Space and gold
// @part Echoes of time
// @part Flow streams
// @part Falling upward
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail kathol
// @palette {"corona.rim_cool":[0.3,0.1,0.65],"corona.rim_hot":[0.95,0.72,0.3],"corona.inner":[1.0,0.9,0.65],"core.white":[1.0,0.97,0.88],"core.fringe_cool":[0.3,0.12,0.8],"core.fringe_hot":[1.0,0.75,0.3],"light.color":[0.75,0.5,0.85],"trail":[0.85,0.62,0.95]}
// @speed 1.0

const vec3 KA_NIGHT = vec3(0.02, 0.015, 0.075);
const vec3 KA_TEAL = vec3(0.05, 0.42, 0.55);
const vec3 KA_VIOLET = vec3(0.42, 0.16, 0.95);
const vec3 KA_GOLD = vec3(1.0, 0.68, 0.18);
const vec3 KA_PALE = vec3(1.0, 0.93, 0.72);

float ka_life(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, 0.5, uAge - 2000.0);
  return smoothstep(0.0, 0.6, uAge);
}
// the twist's own time: it runs on, slows, stops and runs back a little
float ka_time(float t){ return t*0.85 + 1.9*sin(t*0.37) + 0.9*sin(t*0.61 + 1.0); }
// the ribbon's turn at y at time t; lit, it spins fast and settles; let go, it winds back
float ka_phi(float y, float t){
  float wind = uAge >= 2000.0 ? 7.0*smoothstep(0.0, 0.5, uAge - 2000.0) : 7.0*(1.0 - smoothstep(0.0, 0.8, uAge));
  return y*(0.36 + 0.05*sin(t*0.13)) - ka_time(t) + wind;
}
float ka_half(float y, float top, float phi){
  float e = 1.0 - clamp((top - y)/0.9, 0.0, 1.0);
  float end = sqrt(max(1.0 - e*e, 0.0));
  return (0.86 + 0.09*cos(phi))*end*mix(0.55, 1.0, smoothstep(0.0, 1.2, y));
}
vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = (uLength + 0.2)*ka_life();
  if (top < 0.15) return vec3(0.0);
  float yc = clamp(b.y, 0.0, top);
  float phi = ka_phi(yc, uClock);
  float d = max(length(vec2(b.x, b.y - yc)) - ka_half(yc, top, phi), 0.0);
  float c = cos(phi);
  vec3 tone = mix(KA_VIOLET, KA_GOLD, 0.5 + 0.5*c/(abs(c) + 0.2));
  vec3 col = tone*(halo(d, 0.2)*0.6 + halo(d, 0.9)*0.2 + halo(d, 2.4)*0.07);
  col += lvl(g)*tone*(1.0 - smoothstep(0.3, 1.2, abs(b.y)))*0.25;
  // echoes: the ribbon as it was a moment ago, hanging beside it, clear as a breath on glass
  if (part(2.0) > 0.5 && uFx > 0.0){
    float show = rxLife(0.5, 1.5, 0.3)*(0.55 + 0.45*smoothstep(-0.2, 0.7, sin(uClock*0.29)));
    for (int i = 0; i < 2; i++){
      float k = float(i) + 1.0;
      float dt = uClock - k*0.85;
      float ph = ka_phi(yc, dt);
      float off = (k*0.7 + 0.25*sin(yc*0.21 + dt*0.5))*(i == 0 ? 1.0 : -1.0);
      float de = abs(abs(b.x - off) - ka_half(yc, top, ph));
      float hue = fract(yc*0.012 + ph*0.05 + 0.13*k);
      col += hsv(hue, 0.65, 1.0)*(halo(de, 0.16)*0.2 + halo(de, 0.45)*0.08)*show*(1.0 - smoothstep(top - 1.5, top, b.y))*smoothstep(0.0, 3.0, b.y)/k;
    }
  }
  return col*roomFade(abs(b.x))*(1.0 + 0.7*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float top = (uLength + 0.2)*ka_life();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.2) return o;
  float phi = part(0.0) > 0.5 ? ka_phi(b.y, uClock) : 0.0;
  float half = ka_half(b.y, top, phi);
  float cov = (1.0 - smoothstep(half - 0.04 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half)*smoothstep(-0.25, 0.1, b.y);
  if (cov <= 0.0) return o;
  // (update 30, Creyon: odd, cheap) one blade: two ribbons wound round it, night and gold, sliding along each other
  float wrap = phi + asin(clamp(b.x/max(half, 0.01), -1.0, 1.0))*1.15;
  float c = cos(wrap), s0 = sin(wrap);
  // which face you see, soft where it turns edge-on
  float face = 0.5 + 0.5*c/(abs(c) + 0.28);
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0);
  // round the band: it tilts toward you on one side as it turns
  float lit = 0.62 + 0.55*sqrt(max(1.0 - u*u, 0.0));
  vec3 col = mix(vec3(0.62, 0.3, 0.95), KA_GOLD, face);
  if (part(1.0) > 0.5){
    // the night face: deep space, slow clouds, small round stars carried along the band
    vec2 q = vec2(u*1.1, b.y*0.09 - ka_time(uClock)*0.13);
    float n = rxFlow(q + 0.7*rxFlow(q*0.8 + 5.0));
    vec3 night = mix(KA_NIGHT, mix(KA_VIOLET*0.75, KA_TEAL, smoothstep(0.35, 0.8, rxFlow(q*1.4 + 2.0))), smoothstep(0.42, 0.95, n));
    vec2 sc = vec2(u*2.0 + 3.0, b.y*0.8 - ka_time(uClock)*1.0);
    vec2 si = floor(sc);
    vec2 sp = si + 0.2 + 0.6*h22(si);
    night += vec3(0.9, 0.95, 1.0)*halo(length(sc - sp), 0.13)*step(0.6, h21(si + 2.0))*(0.75 + 0.25*sin(uClock*1.4 + h21(si)*17.0));
    // the gold face: thick liquid, slow bright currents folding over each other
    vec2 g0 = vec2(u*1.6 + 0.5*sin(b.y*0.15 + uClock*0.3), b.y*0.16 + ka_time(uClock)*0.42);
    float gf = rxFlow(g0 + 0.9*rxFlow(g0*0.75 + 9.0));
    vec3 gold = mix(vec3(0.92, 0.52, 0.1), KA_GOLD*1.25, smoothstep(0.2, 0.6, gf));
    gold = mix(gold, KA_PALE*1.15, smoothstep(0.58, 0.9, gf));
    col = mix(night, gold, face);
  }
  col *= lit;
  // edge-on, it burns with every colour
  float edgeOn = 1.0 - smoothstep(0.0, 0.3, abs(c));
  col = mix(col, hsv(fract(b.y*0.03 - uClock*0.12), 0.72, 1.0)*1.25, edgeOn*0.6);
  col = mix(col, KA_PALE*1.1, (1.0 - smoothstep(0.0, 0.42, abs(u)))*(0.22 + 0.3*face));
  // a thin bright line along both its edges, soft, the colour of the side it borders
  col += mix(KA_VIOLET*1.2, KA_PALE, face)*(1.0 - smoothstep(0.0, 0.17, half - abs(b.x)))*0.75;
  col += KA_PALE*halo(max(b.y, 0.0), 0.5)*0.4;
  col = mix(col, KA_PALE, 0.5*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength, t;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.4, 1.3, 0.3);
  float T = ka_time(uClock);
  // two streams winding round it against the ribbon's turn
  if (part(3.0) > 0.5){
    for (int i = 0; i < 2; i++){
      float k = float(i);
      vec3 a = k < 0.5 ? KA_GOLD : KA_VIOLET;
      fx += 1.5*xDust(ro, rd, 1.7, 2.3, 1.0, k*0.5 + 0.06*T, L*0.04, L*0.98, 0.34, k + 41.0, mix(a, vec3(1.0), 0.5), a);
      fx += 0.7*xPuffs(ro, rd, 1.7, 2.3, 1.0, k*0.5 + 0.06*T, L*0.04, L*0.98, 0.42, k + 47.0, a, a*0.4);
      fx += 1.2*xBeads(ro, rd, 2.0, 2.4, 1.0, k*0.5 + 0.06*T, L*0.04, L*0.98, 0.1, k + 43.0, vec3(1.0), a);
    }
    fx += 0.5*syMistSpin(ro, rd, L*0.05, L*0.98, 1.2, 4.0, -0.03, 45.0, KA_VIOLET*0.42, KA_TEAL*0.25);
  }
  // the rift itself: slow bright comets of time chasing round it, gold and violet flame up it, pale beams in the air
  for (int i = 0; i < 3; i++){
    float k = float(i);
    vec3 a = i == 1 ? KA_VIOLET : KA_GOLD;
    fx += 1.2*rxComet(ro, rd, px, 2.4 + 0.8*k, L*(0.1 + 0.26*k), L*0.4, 9.0 + 3.0*k, k + 61.0, mix(a, vec3(1.0), 0.5), a);
  }
  fx += 0.8*shFire(ro, rd, 1.25, 0.3, L, 1.6, 0.5, 5.0, KA_VIOLET*0.06, KA_VIOLET*0.5, KA_GOLD);
  fx += 0.7*rxRays(ro, rd, px, 6.0, L*0.12, L*0.95, 3.8, 63.0, KA_PALE*0.45, KA_VIOLET*0.3);
  fx += 0.9*syMistSpin(ro, rd, L*0.05, L*0.98, 1.4, 5.0, 0.03, 65.0, KA_GOLD*0.3, KA_VIOLET*0.32);
  // motes that fall upward, stop, and fall back: time going both ways
  if (part(4.0) > 0.5){
    for (int i = 0; i < 26; i++){
      float k = float(i);
      float tt = uClock*(0.1 + 0.08*h11(k*2.3)) + h11(k*6.7)*6.2832;
      float y = L*(0.5 + 0.47*sin(tt + 0.9*sin(tt*2.0)));
      vec3 p = around(1.4 + 2.8*h11(k*3.7), h11(k*5.1) + 0.05*T*(k < 13.0 ? 1.0 : -1.0), y);
      float d = rayPt(ro, rd, p, t);
      vec3 hue = hsv(fract(h11(k*9.1) + y*0.01), 0.5, 1.0);
      fx += hue*(halo(d, max(0.05, px))*1.3 + halo(d, 0.22)*0.32);
    }
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += KA_PALE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 1.1)*cl;
  return fx*aft*uFx*hiltFade(ro, rd);
}
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
const vec3 SX_HOT = vec3(0.950, 0.720, 0.300);
const vec3 SX_DEEP = vec3(0.300, 0.100, 0.650);
const vec3 SX_PALE = vec3(1.000, 0.900, 0.650);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 3.0, 8.0, 3.40, 0.080, 0.20, 0.08, 0.96, -1.0, 7533.0, vec3(1.0, 0.95, 0.8), SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(4.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.36, 4.46, -0.015, 741.0, vec3(0.950, 0.720, 0.300)*0.24, vec3(0.300, 0.100, 0.650)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.950, 0.720, 0.300)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(4.0) > 0.5){
    a += 1.4*vec3(0.0);
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
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.24*gxKin(ro, rd, 5.0, 4.0, 3.60, 0.040, 0.80, 0.08, 0.96, 1.0, 9187.0, vec3(1.000, 0.900, 0.650), vec3(0.950, 0.720, 0.300));
  a += 1.2*gxFlares(ro, rd, px, 6.0, 5.0, 56.0, vec3(1.000, 0.900, 0.650), vec3(0.950, 0.720, 0.300));
  a += 1.0*gxVortex(ro, rd, px, 10.0, 4.6, 57.0, vec3(1.000, 0.900, 0.650), vec3(0.950, 0.720, 0.300));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

