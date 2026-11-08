// Exar Kun Temple (Legendary, rebuilt on Sol's glow and core 3/11/2026): The temple Exar Kun raised on Yavin 4 and bound his spirit to: a blade of black stone with burning lips and gold seams waking in slow waves. Six temple needles hang round it in the air striking it with lightning, his lost spirits wander round it as points going dark and bright, and souls climb past with long tails.
// @part Gold seams
// @part Temple needles
// @part Massassi haze
// @part Spirit fire
// @arcs Sith lightning
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail temple
// @palette {"corona.rim_cool":[0.231,0.099,0.539],"corona.rim_hot":[0.42,0.18,0.98],"corona.inner":[1.0,0.95,0.76],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.336,0.144,0.784],"core.fringe_hot":[1.0,0.95,0.76],"light.color":[0.42,0.18,0.98],"trail":[0.42,0.18,0.98]}
// @speed 1.0

const vec3 V = vec3(0.420, 0.180, 0.980);
const vec3 D = vec3(0.012, 0.006, 0.030);
const vec3 G = vec3(1.000, 0.720, 0.220);
const vec3 H = vec3(1.000, 0.950, 0.760);
const vec3 S = vec3(0.070, 0.050, 0.095);
float ex_top(){ return uLength*rxLife(0.0, 0.45, 0.32); }
float ex_half(float y){
  float top = ex_top();
  if (y > top) return 0.0;
  float w = 0.48 + 0.14*(1.0 - smoothstep(0.0, uLength*0.5, y));
  float e = 1.0 - clamp((top - y)/2.4, 0.0, 1.0);
  return w*sqrt(max(1.0 - e*e, 0.0));
}
float ex_wake(float y){ return pow(0.5 + 0.5*sin(y*0.3 - uClock*1.9), 3.0); }
float ex_stirs(){ return rxCycle(15.0, 7.0, 9.2, 11.2, 14.2); }
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  float st = ex_stirs();
  vec3 col = g*(0.66 + 0.35*st + 0.3*ex_wake(b.y) + 0.7*clamp(uClash, 0.0, 1.0));
  // the wide violet breath of the temple, and gold running in waves along the blade
  float top = ex_top();
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - ex_half(yc), 0.0);
  col += V*(halo(d, 1.7)*(0.16 + 0.16*st) + halo(d, 4.6)*(0.05 + 0.06*st));
  col += G*halo(d, 0.5)*ex_wake(b.y)*0.5;
  return col*tipFadeB(b)*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = ex_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0);
  vec3 col = D + V*0.1*rxFlow(vec2(b.x*1.5, b.y*0.08));
  float lip = smoothstep(0.7, 1.0, abs(u));
  float wake = ex_wake(b.y);
  if (part(0.0) > 0.5){
    float v = rxFlow(vec2(b.x*1.35 + b.y*0.04, b.y*0.05 - uClock*0.16));
    col += mix(G, H, wake)*(1.0 - smoothstep(0.0, 0.065, abs(v - 0.5)))*(0.9 + 2.2*wake);
    col += V*(1.0 - smoothstep(0.0, 0.2, abs(v - 0.5)))*0.3;
  }
  col = mix(col, mix(V*1.3, mix(G, H, wake), 0.25 + 0.75*wake), lip*0.9);
  col = mix(col, H, 0.5*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 ex_stonePos(float k, float s){
  float a = k/6.0 + uClock*0.05;
  float y0 = uLength*(0.06 + 0.125*k) + 1.5*sin(uClock*0.55 + k*2.1);
  float tall = 8.0 + 2.6*h11(k*3.7);
  return around(3.2 + 0.8*sin(k*2.4) + 0.55*sin(uClock*0.8 + k*1.7) + 0.55*s*sin(uClock*1.1 + k), a + 0.012*s*sin(uClock*0.9 + k*3.0), y0 + tall*s);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.2, 0.9, 0.35);
  if (aft <= 0.01) return fx;
  float st = ex_stirs();
  if (part(1.0) > 0.5) for (int i = 0; i < 6; i++){
    float k = float(i);
    vec3 cap = ex_stonePos(k, 1.0);
    float sn, dn = raySeg(ro, rd, ex_stonePos(k, 0.0), cap, sn);
    fx += (V*(halo(dn, 0.8)*0.5 + halo(dn, 2.0)*0.12) + G*halo(dn, 0.34)*0.42*sn)*smoothstep(0.0, 0.35, sn);
    float wn = mix(0.74, 0.05, pow(sn, 1.1));
    fx += mix(G, H, sn)*(halo(abs(dn - wn), 0.07)*0.7 + halo(abs(dn - wn), 0.26)*0.2)*smoothstep(0.03, 0.4, sn);
    float t, d = rayPt(ro, rd, cap, t);
    float pulse = 0.65 + 0.35*sin(uClock*1.3 + k*1.9);
    fx += (H*halo(d, 0.11)*1.5 + G*halo(d, 0.42)*0.8 + G*halo(d, 1.3)*0.16)*pulse*(1.0 + st);
    // Storm's beat: a strike leaves each stone's crown for the blade
    float cy = uClock*6.5 + k*0.618 + h11(k*5.1)*3.0;
    float m = floor(cy), q = fract(cy);
    if (h11(m*1.3 + k*7.1) > mix(0.62, 0.3, st)){
      float yy = cap.y - 1.5 - 2.5*h11(m*2.3 + k);
      float db = liveBolt(ro, rd, cap, around(2.0, k/6.0 + uClock*0.05, cap.y - 0.8), around(0.4, k/6.0 + uClock*0.05, yy), 0.45, m + k*3.1, px);
      fx += (mix(G, vec3(1.0), 0.5)*halo(db, max(0.04, px))*1.5 + V*halo(db, 0.22)*0.8)*pow(1.0 - q, 1.5);
    }
  }
  if (part(2.0) > 0.5) fx += 1.1*syMistSpin(ro, rd, 0.3, L*0.97, 1.2, 5.6, 0.05, 11.0, V*0.55, D*6.0);
  if (part(3.0) > 0.5){
    for (int i = 0; i < 4; i++){
      float k = float(i);
      float run = uClock*(0.26 + 0.05*h11(k)) + k*0.31;
      float cyc = floor(run), ph = fract(run);
      float life = sin(3.1416*ph);
      float y = L*(0.05 + 0.8*h11(k*2.3 + cyc*1.1));
      float a = h11(k*5.7 + cyc*0.7) + 0.05*ph;
      fx += (1.0 + 0.8*st)*life*rxTongue(ro, rd, y, a, (4.2 + 2.6*h11(k*3.1 + cyc))*(0.5 + 0.6*ph), 0.5, 0.62, k*3.0 + cyc, H, G*0.85, V*0.75);
    }
    fx += syInflow(ro, rd, px, 8.0, 5.6, L*0.12, L*0.9, 0.14, 13.0, H, G);
    // the temple's lost spirits, wandering points that go dark and bright
    fx += (0.7 + 0.4*st)*gxKin(ro, rd, 5.0, 4.0, 3.00, 0.040, 0.80, 0.08, 0.96, 1.0, 7299.0, G, V*0.8);
    // and souls climbing them, each with a long tail
    for (int i = 0; i < 2; i++){
      float k = float(i);
      fx += 1.15*rxComet(ro, rd, px, 2.1 + 0.6*k, L*0.08, L*0.72, 7.0 + 1.3*k, k*3.3 + 16.0, H, mix(G, V, 0.35));
    }
  }
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx <= 0.0 || part(1.0) < 0.5 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  for (int i = 0; i < 6; i++){
    float k = float(i), s;
    vec3 a = ex_stonePos(k, 0.0), c = ex_stonePos(k, 1.0);
    float d = raySeg(ro, rd, a, c, s);
    float w = mix(0.74, 0.05, pow(s, 1.1))*(0.8 + 0.42*vn(vec2(s*9.0 + k*3.0, k*1.7)));
    if (d > w + 0.3) continue;
    float cov = (1.0 - smoothstep(w - 0.1 - px, w + 0.05 + px, d))*smoothstep(0.0, 0.32, s);
    if (cov <= 0.0) continue;
    float fl = rxFlow(vec2(s*3.2 - uClock*0.12 + k*2.0, d*3.0 + k));
    float seam = (1.0 - smoothstep(0.0, 0.06, abs(fl - 0.5)))*smoothstep(0.05, 0.4, s);
    float edge = smoothstep(w*0.45, w, d);
    vec3 col = mix(S + V*0.16, mix(G, H, s), edge*(0.3 + 0.7*s)) + mix(G, H, 0.4)*seam*(1.0 - edge)*1.2;
    cov *= rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*growMask(ro, rd);
    if (cov > o.a) o = vec4(col, cov);
  }
  return o;
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
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
vec3 sig_vol_m26(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.58*gxKin(ro, rd, 3.0, 6.0, 2.60, 0.060, 0.20, 0.08, 0.96, -1.0, 9102.0, vec3(0.780, 0.500, 0.820), vec3(0.420, 0.180, 0.980));
  a += 1.05*gxBeads(ro, rd, px, 2.0, 3.2, 217.0, vec3(0.780, 0.500, 0.820), vec3(0.420, 0.180, 0.980));
  a += 1.3*gxLanterns(ro, rd, px, 8.0, 5.2, 17.0, vec3(0.780, 0.500, 0.820), vec3(0.420, 0.180, 0.980));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(49.0, 17.0, 25.0, 30.0, 39.0), L = uLength;
  a += m*1.45*gxHelix(ro, rd, px, 18.0, 3.2, 2.4, 0.06, 1112.0, vec3(0.780, 0.500, 0.820), vec3(0.420, 0.180, 0.980));
  a += m*1.1*gxLanterns(ro, rd, px, 8.0, 3.2, 1113.0, vec3(0.780, 0.500, 0.820), vec3(0.420, 0.180, 0.980));
  return base*1.0 + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

