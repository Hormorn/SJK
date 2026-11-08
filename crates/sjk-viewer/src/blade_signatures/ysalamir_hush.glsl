// Ysalamir Hush (Legendary, 3/11/2026): The forest of Myrkr, where the ysalamir silence the Force: a slim blade of living scales with two olbio branches winding round it, thick with bark, moss, hanging strands and glowing fronds that breathe. Rivers of fireflies circle it, a green aurora hangs in the air and a dark hush climbs the blade.
// @part Scaled hide
// @part Olbio branches
// @part Firefly rivers
// @part Moss aurora
// @wisps Slow breath
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail hush
// @palette {"corona.rim_cool":[0.385,0.44,0.303],"corona.rim_hot":[0.7,0.8,0.55],"corona.inner":[0.9,0.94,0.8],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.56,0.64,0.44],"core.fringe_hot":[0.9,0.94,0.8],"light.color":[0.7,0.8,0.55],"trail":[0.7,0.8,0.55]}
// @speed 1.0

const vec3 M = vec3(0.420, 0.720, 0.360);
const vec3 Dk = vec3(0.030, 0.085, 0.040);
const vec3 A = vec3(0.680, 0.980, 0.480);
const vec3 Am = vec3(1.000, 0.700, 0.240);
const vec3 Bk = vec3(0.110, 0.070, 0.042);
const vec3 H = vec3(0.920, 1.000, 0.800);
float ys_top(){ return uLength*rxLife(0.0, 0.5, 0.32); }
float ys_gap(float y){
  float L = uLength, g = fract(uClock*0.1)*(L + 10.0) - 5.0;
  return 1.0 - smoothstep(1.4, 3.6, abs(y - g));
}
float ys_half(float y){
  float top = ys_top();
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/2.2, 0.0, 1.0);
  return (0.5 + 0.07*sin(y*0.8 - uClock*1.5))*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  float gap = ys_gap(b.y);
  vec3 col = g*(0.62 - 0.5*gap + 0.7*clamp(uClash, 0.0, 1.0));
  float top = ys_top(), yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - ys_half(yc), 0.0);
  col += M*(halo(d, 0.9)*0.3*(1.0 - gap) + halo(d, 3.2)*0.07);
  // light coming down through the canopy: long soft shafts leaning across it
  float shaft = rxFlow(vec2(b.y*0.3 + b.x*0.34 - uClock*0.05, 4.0));
  col += mix(A, Am, 0.3)*shaft*shaft*shaft*halo(d, 4.2)*0.42;
  return col*tipFadeB(b)*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = ys_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  vec3 col = M;
  if (part(0.0) > 0.5){
    float ripple = 0.5 + 0.5*sin(b.y*0.8 - uClock*1.5);
    float row = floor(b.y*2.3);
    vec2 q = fract(vec2(asin(u)*1.05 + 0.5*row, b.y*2.3)) - 0.5;
    float sc = length(q*vec2(1.0, 0.85));
    col = mix(mix(H, A, 0.25 + 0.5*ripple), Dk*2.0, smoothstep(0.2, 0.55, sc)*(1.15 - 0.45*ripple));
    col = mix(col, Am*0.85, 0.3*smoothstep(0.55, 0.85, rxFlow(vec2(b.x*1.1, b.y*0.09 - uClock*0.16))));
  }
  col *= 0.5 + 0.7*nz;
  float gap = ys_gap(b.y);
  col = mix(col, Dk*0.55, gap*0.9);
  col += A*smoothstep(0.7, 1.0, abs(u))*(0.3 + 0.5*gap);
  col = mix(col, H, 0.45*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
// The olbio branch, wound round the blade; f from the hand to the point.
vec3 ys_wood(float f){
  float L = uLength;
  return around(1.3 + 0.26*sin(f*7.0 - uClock*0.5), 1.5*f + 0.05*sin(f*6.0 + uClock*0.3) + 0.03*uClock, mix(0.5, L - 1.4, f));
}
// The second branch, winding the other way and further out.
vec3 ys_wood2(float f){
  float L = uLength;
  return around(2.15 + 0.3*sin(f*6.0 + uClock*0.4), 0.5 - 1.15*f + 0.05*sin(f*5.0 - uClock*0.3) - 0.02*uClock, mix(0.4, L - 2.2, f));
}
float ys_trace2(vec3 ro, vec3 rd, out float at){
  float bWs = 0.0;
  float best = 1e3, s;
  at = 0.0;
  vec3 prev = ys_wood2(0.0);
  for (int j = 1; j <= 22; j++){
    float f = float(j)/22.0;
    vec3 q = ys_wood2(f);
    float d = raySeg(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); at += (f - (1.0 - s)/22.0 - at)*wN/bWs; }
    prev = q;
  }
  return best;
}
// A broad glowing frond leaving a branch, opening and closing as if it breathed.
vec3 ys_frond(vec3 ro, vec3 rd, float px, vec3 at, float k){
  float open = 0.7 + 0.3*sin(uClock*0.9 + k*2.1);
  float dir = k*2.4 + 0.25*sin(uClock*0.6 + k);
  vec3 tip = at + vec3(sin(dir), 0.5 + 0.35*sin(uClock*0.8 + k), cos(dir))*(1.15 + 0.7*h11(k*5.1))*open;
  vec3 mid = mix(at, tip, 0.5) + vec3(0.0, 0.2, 0.0);
  float s1, s2;
  float d1 = raySeg(ro, rd, at, mid, s1), d2 = raySeg(ro, rd, mid, tip, s2);
  float wl = d1 < d2 ? 0.26*smoothstep(0.0, 1.0, s1) : 0.26*(1.0 - s2*s2);
  float dl = min(d1, d2);
  return mix(M, A, 0.75)*(1.0 - smoothstep(wl*0.25, wl + px + 0.09, dl))*0.8 + M*halo(dl, 0.55)*0.22;
}
float ys_trace(vec3 ro, vec3 rd, out float at){
  float bWs = 0.0;
  float best = 1e3, s;
  at = 0.0;
  vec3 prev = ys_wood(0.0);
  for (int j = 1; j <= 28; j++){
    float f = float(j)/28.0;
    vec3 q = ys_wood(f);
    float d = raySeg(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); at += (f - (1.0 - s)/28.0 - at)*wN/bWs; }
    prev = q;
  }
  return best;
}
vec3 sig_vol_e3(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.3, 1.1, 0.35);
  if (aft <= 0.01) return fx;
  if (part(1.0) > 0.5){
    float t;
    // moss lights along the branch: clusters of soft glow
    for (int i = 0; i < 11; i++){
      float k = float(i);
      vec3 p = ys_wood((k + 0.5)/11.0) + vec3(0.18*sin(k*5.1), 0.18*cos(k*3.3), 0.18*cos(k*5.1));
      float d = rayPt(ro, rd, p, t);
      float br = 0.45 + 0.55*pow(0.5 + 0.5*sin(uClock*2.2 - k*1.1), 2.0);
      fx += (H*halo(d, 0.1)*1.3 + A*halo(d, 0.3)*1.1 + M*(halo(d, 0.85)*0.55 + halo(d, 2.0)*0.13))*br;
    }
    if (fxNear(ro, rd, 0.0, L) < 6.0){
      for (int i = 0; i < 12; i++){
        float k = float(i);
        vec3 p0 = mod(k, 2.0) < 0.5 ? ys_wood((k + 0.5)/12.0) : ys_wood2((k + 0.5)/12.0);
        if (rayPtRaw(ro, rd, p0, t) > 3.7) continue;
        fx += ys_frond(ro, rd, px, p0, k);
      }
      float at2, d2 = ys_trace2(ro, rd, at2);
      fx += M*(halo(d2, 0.45)*0.26 + halo(d2, 1.2)*0.07);
      float at, d = ys_trace(ro, rd, at);
      fx += M*(halo(d, 0.5)*0.3 + halo(d, 1.3)*0.08);
      // moss hanging from the branch in long pale strands, stirring
      for (int i = 0; i < 18; i++){
        float k = float(i), s;
        vec3 top = ys_wood((k + 0.5)/18.0);
        if (rayPtRaw(ro, rd, top + vec3(0.0, -1.6, 0.0), t) > 3.2) continue;
        vec3 bot = top + vec3(0.65*sin(uClock*1.1 + k*2.1), -1.5 - 1.7*h11(k*3.7), 0.65*cos(uClock*0.9 + k*1.3));
        float dm = raySeg(ro, rd, top, bot, s);
        fx += (mix(H, A, s)*halo(dm, max(0.045, px))*1.15 + M*halo(dm, 0.24)*0.42)*(1.0 - s)*(1.0 - s)*(0.6 + 0.4*sin(uClock*0.8 + k*2.7));
      }
    }
  }
  if (part(2.0) > 0.5){
    float t;
    for (int i = 0; i < 14; i++){
      float k = float(i);
      // held out at the edge of the bubble: they drift in and are pushed back
      float rr = 3.0 + 1.1*sin(uClock*0.6 + k*1.9) + 0.6*h11(k*3.1);
      vec3 p = around(rr, h11(k*7.7) + 0.05*uClock + 0.05*sin(uClock*0.5 + k), L*(0.06 + 0.88*h11(k*2.3)) + 1.5*sin(uClock*0.5 + k*2.7));
      float d = rayPt(ro, rd, p, t);
      float blink = pow(0.5 + 0.5*sin(uClock*2.3 + k*4.3), 3.0);
      fx += (mix(Am, H, 0.4)*halo(d, max(0.075, px))*2.0 + Am*(halo(d, 0.34)*0.8 + halo(d, 1.0)*0.16))*(0.25 + 0.75*blink);
    }
  }
  if (part(2.0) > 0.5){
    // two living streams of fireflies winding round it, the swarm the ysalamir hides in
    fx += 1.5*xBeads(ro, rd, 2.9, 3.2, 1.6, uClock*0.13, L*0.04, L*0.95, 0.09, 91.0, H, Am);
    fx += 1.35*xBeads(ro, rd, 3.5, 3.0, -1.4, uClock*0.1 + 0.5, L*0.04, L*0.95, 0.08, 93.0, mix(Am, H, 0.3), A);
  }
  if (part(3.0) > 0.5){
    fx += 1.25*syMistSpin(ro, rd, 0.3, L*0.97, 1.4, 5.2, -0.03, 94.0, A*0.24, M*0.16);
    // the great trees' leaves coming down through it, pale and slow, and amber pollen going up
    fx += 1.5*shFlakes(ro, rd, 4.3, 0.5, L - 0.4, 15.0, 2.9, -0.55, 0.05, 0.13, 0.6, 0.0, 95.0, A, M*0.6);
    fx += 1.3*shFlakes(ro, rd, 2.8, 0.5, L - 0.4, 12.0, 3.1, -0.4, -0.04, 0.1, 0.5, 0.0, 96.0, H, A*0.5);
    fx += 1.3*gxKin(ro, rd, 1.0, 6.0, 2.40, 0.040, 0.22, 0.08, 0.96, -1.0, 8261.0, Am + H*0.2, Am*0.5);
  }
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  float L = uLength;
  if (uFx <= 0.0 || part(1.0) < 0.5 || fxNear(ro, rd, 0.0, L) > 16.0) return o;
  float at2, d2 = ys_trace2(ro, rd, at2);
  float w2 = mix(0.24, 0.07, at2)*(0.86 + 0.28*vn(vec2(at2*34.0, 7.0)));
  float c2 = 1.0 - smoothstep(w2 - 0.03 - px, w2 + 0.03 + px, d2);
  if (c2 > 0.0){
    float m2 = smoothstep(0.55, 0.72, rxFlow(vec2(at2*26.0 + 9.0, clamp(d2/w2, 0.0, 1.0)*1.3)));
    o = vec4(mix(Bk*(0.6 + 1.5*smoothstep(0.3, 0.75, rxFlow(vec2(at2*50.0, 4.0)))), mix(M, A, m2)*0.8, m2), c2);
  }
  float at, d = ys_trace(ro, rd, at);
  float w = mix(0.36, 0.13, at)*(0.86 + 0.28*vn(vec2(at*40.0, 3.0)));
  float cov = 1.0 - smoothstep(w - 0.03 - px, w + 0.03 + px, d);
  if (cov <= 0.0){ o.a *= uFx*rxLife(0.3, 1.1, 0.35)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd); return o; }
  float side = clamp(d/w, 0.0, 1.0);
  float bark = rxFlow(vec2(at*62.0, side*2.4));
  vec3 col = mix(Bk*0.55, Bk*2.3, smoothstep(0.3, 0.75, bark))*(1.0 - 0.6*side*side);
  float moss = smoothstep(0.58, 0.74, rxFlow(vec2(at*30.0 + 5.0, side*1.3)));
  col = mix(col, mix(M, A, moss)*0.8, moss);
  o = vec4(mix(o.rgb, col, cov), max(o.a, cov));
  o.a *= uFx*rxLife(0.3, 1.1, 0.35)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return o;
}
vec3 sig_glow_e3(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e3(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the forest's spirits, drifting through the silence

  if (part(2.0) > 0.5) a += 1.5*vec3(0.0);
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_e3(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  if (part(3.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol_m26(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.5*gxKin(ro, rd, 6.0, 5.0, 2.00, 0.040, 0.40, 0.08, 0.96, 1.0, 9459.0, H, mix(A, Am, 0.3));
  a += 1.3*gxSwarm(ro, rd, px, 12.0, 4.8, 21.0, vec3(0.600, 0.920, 0.500), vec3(0.260, 0.620, 0.300));
  a += 1.05*gxTumble(ro, rd, px, 8.0, 4.8, 1.0, 1.0, 0.34, 22.0, vec3(0.600, 0.920, 0.500), vec3(0.260, 0.620, 0.300));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(55.0, 20.0, 30.0, 37.0, 47.0), L = uLength;
  a += m*1.35*gxSwarm(ro, rd, px, 12.0, 3.4, 1116.0, vec3(0.600, 0.920, 0.500), vec3(0.260, 0.620, 0.300));
  return base*(1.0 - 0.75*m) + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

