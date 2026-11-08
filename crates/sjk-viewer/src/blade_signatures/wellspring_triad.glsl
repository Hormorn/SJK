// Wellspring Triad (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): the source of the Force the Priestesses guard: three living streams, gold, silver and sea-green, twist round each other up a dark blade and flare white where they cross. Out in the air the same three streams turn in slow beads of light.
// @part Three streams
// @part Where they meet
// @part Beads of life
// @part Living light
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail starfall
// @palette {"corona.rim_cool":[0.65,0.5,0.2],"corona.rim_hot":[1.0,0.85,0.45],"corona.inner":[1.0,0.97,0.88],"core.white":[1.0,0.99,0.95],"core.fringe_cool":[0.9,0.75,0.4],"core.fringe_hot":[1.0,0.92,0.7],"light.color":[1.0,0.9,0.65],"trail":[1.0,0.9,0.6]}
// @speed 1.0

const vec3 M = vec3(0.140, 0.130, 0.100);
const vec3 D = vec3(0.030, 0.030, 0.030);
const vec3 H = vec3(1.000, 0.970, 0.850);
const vec3 A = vec3(1.000, 0.740, 0.280);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.72;
  float top = L*rxLife(0.0, 0.45, 0.32);
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/max(w*1.0, 0.05), 0.0, 1.0);
  return w*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float yc = clamp(b.y, 0.0, L*rxLife(0.0, 0.45, 0.32));
  float d = max(length(vec2(b.x, b.y - yc)) - k_half(yc, b.x < 0.0 ? -1.0 : 1.0), 0.0);
  float T = uTime, C = uClock, y = b.y, s = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  vec3 col = vec3(1.0, 0.85, 0.5)*(halo(d, 0.16)*0.6 + halo(d, 0.8)*0.3 + halo(d, 2.4)*0.06);
  col += lvl(g)*vec3(1.0, 0.85, 0.5)*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.28;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.6*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 k_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = k_half(b.y, b.x < 0.0 ? -1.0 : 1.0);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0);
  float ang = asin(u), nz = sqrt(max(1.0 - u*u, 0.0));
  float x = b.x, y = b.y, s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float T = uTime, C = uClock;
  vec3 col = M;
  if (part(0.0) > 0.5){
    float tw = y*0.22 - T*0.4;
    float a1 = sin(ang*1.0 + tw), a2 = sin(ang*1.0 + tw + 2.09), a3 = sin(ang*1.0 + tw + 4.19);
    col = mix(D, M, kf(x*2.0, y*0.14));
    float w1 = smoothstep(0.84, 1.0, a1), w2 = smoothstep(0.84, 1.0, a2), w3 = smoothstep(0.84, 1.0, a3);
    col += A*w1*1.25 + vec3(0.8, 0.86, 1.0)*w2*1.15 + vec3(0.2, 0.95, 0.72)*w3*1.15;
    if (part(1.0) > 0.5) col += H*smoothstep(0.6, 1.1, min(w1 + w2, 1.0)*0.5 + min(w2 + w3, 1.0)*0.5)*0.0;
  }
  col *= (0.7 + 0.4*nz)*1.5;
  col += vec3(1.0, 0.85, 0.5)*pow(1.0 - nz, 3.0)*0.9;
  col = mix(col, H, (1.0 - smoothstep(0.0, 0.55, abs(u)))*0.7);
  col += H*halo(max(b.y, 0.0), 0.5)*0.5;
  col = mix(col, H, 0.45*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec4 sig_solid(vec4 o, vec2 b, float px){
  vec4 r = k_solid(o, b, px);
  float j = smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, r.rgb, j), mix(o.a, r.a, j));
}
float kMoment(){ return rxCycle(26.0, 15.0, 17.2, 19.2, 22.2); }
vec3 kVine(vec3 ro, vec3 rd, float px, float r0, float r1, float y0, float y1, float turns, float a0, float wob, float w, vec3 hot, vec3 col, float k){
  if (fxNear(ro, rd, min(y0, y1), max(y0, y1)) > max(r0, r1) + wob + 1.2) return vec3(0.0);
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(r0, a0, y0);
  for (int i = 1; i <= 26; i++){
    float f = float(i)/26.0;
    vec3 q = around(mix(r0, r1, f) + wob*sin(f*9.0 + uClock*0.7 + k), a0 + turns*f, mix(y0, y1, f));
    float d = raySeg(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(i) - 1.0 + s)/26.0 - bf)*wN/bWs; }
    prev = q;
  }
  float pulse = pow(0.5 + 0.5*sin(bf*11.0 - uClock*1.3 + k), 4.0);
  float ww = mix(w, w*0.3, bf);
  return (hot*halo(best, max(ww, px))*(0.6 + 0.9*pulse) + col*halo(best, ww*4.5)*(0.35 + 0.5*pulse))*smoothstep(0.0, 0.06, bf)*(1.0 - smoothstep(0.82, 1.0, bf));
}
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  if (part(2.0) > 0.5){ fx += kVine(ro, rd, px, 1.0, 2.1, 0.3, L - 0.3, 1.7, 0.02*C, 0.2, 0.09, H, A, 1.0);
   fx += kVine(ro, rd, px, 1.0, 2.1, 0.3, L - 0.3, 1.7, 0.333 + 0.02*C, 0.2, 0.09, H, vec3(0.8, 0.86, 1.0), 3.0);
   fx += kVine(ro, rd, px, 1.0, 2.1, 0.3, L - 0.3, 1.7, 0.667 + 0.02*C, 0.2, 0.09, H, vec3(0.2, 0.95, 0.72), 5.0);
   fx += 1.2*xBeads(ro, rd, 1.5, 1.9, 1.3, 0.1 + 0.008*T, L*0.03, L*0.97, 0.08, 3.0, H, A);
   fx += 1.2*xBeads(ro, rd, 1.5, 1.9, 1.3, 0.433 + 0.008*T, L*0.03, L*0.97, 0.08, 5.0, H, vec3(0.8, 0.86, 1.0)); }
  if (part(3.0) > 0.5){ fx += (0.35 + 0.6*mom)*rxAuraShell(ro, rd, -0.3, L + 0.3, 2.8 + 0.8*mom, 5.0, A*0.7, vec3(0.2, 0.95, 0.72)*0.6); fx += 1.2*rxBorn(ro, rd, px, 18.0, 0.5, L - 0.5, 0.5, 2.6, 2.4, 0.05, 15.0, H, A); }
  if (part(1.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 7.5){
    // the fountain only this blade has: what climbs the three streams spills over high on the blade and
    // falls back all round it as long arcs of bright drops, never above its point
    for (int i = 0; i < 54; i++){
      float k = float(i);
      float q = fract(C*0.09 + h11(k*2.3));
      float a = k/54.0*3.0 + 0.012*C;
      float out_ = 3.6 + 1.2*h11(k*3.7);
      vec3 p = around(0.9 + out_*q, a, L*(0.9 + 0.08*sin(3.1416*min(q*2.2, 1.0))) - L*0.86*q*q);
      float d = rayPt(ro, rd, p, t);
      float w3 = mod(k, 3.0);
      vec3 hue = w3 < 0.5 ? A : (w3 < 1.5 ? vec3(0.8, 0.86, 1.0) : vec3(0.2, 0.95, 0.72));
      fx += (H*halo(d, max(0.045, px))*1.3 + hue*halo(d, 0.2)*0.6)*smoothstep(0.0, 0.1, q)*(1.0 - smoothstep(0.85, 1.0, q));
    }
  }
  // effects rise with the blade as it lights and sink with it as it goes out
  float sa;
  raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, L + 4.0, 0.0), sa);
  float grownTop = (L + 6.0)*rxLife(0.0, 0.55, 0.32);
  fx *= 1.0 - smoothstep(grownTop - 4.0, grownTop, sa*(L + 4.0));
  // nothing over the point
  fx *= tipFade(ro, rd);
  uTurnFx += fx*aft*uFx*hiltFade(ro, rd);
  return vec3(0.0);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(1.000, 0.850, 0.450);
const vec3 SX_DEEP = vec3(0.650, 0.500, 0.200);
const vec3 SX_PALE = vec3(1.000, 0.970, 0.880);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 1.1*shMotes(ro, rd, 1.9, 0.5, L - 0.5, 12.0, 2.6, 0.9, 0.02, 0.05, 0.55, 0.0, 41.0, SX_PALE, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34

// >>> update 52: this blade's great effects in the air
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  a += 1.3*syArcs(ro, rd, px, 6.0, 3.0, 5.0, 0.3, 531.0, vec3(1.0, 0.98, 0.85), vec3(1.0, 0.75, 0.22));
  a += 1.15*xBeads(ro, rd, 2.3, 2.9, 1.5, C*0.16, L*0.04, L*0.95, 0.09, 532.0, vec3(1.0), vec3(1.0, 0.78, 0.3));
  a += 1.15*xBeads(ro, rd, 2.9, 2.3, -1.5, C*0.14 + 0.33, L*0.04, L*0.95, 0.09, 533.0, vec3(1.0), vec3(0.4, 0.9, 0.7));
  a += 1.15*xBeads(ro, rd, 3.4, 3.0, 1.1, C*0.11 + 0.66, L*0.04, L*0.95, 0.08, 534.0, vec3(1.0), vec3(0.45, 0.7, 1.0));
  a += 0.8*syMistSpin(ro, rd, 0.3, L*0.96, 1.3, 4.6, 0.05, 535.0, vec3(0.3, 0.24, 0.1), vec3(0.04, 0.05, 0.05));
  return base + a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<< update 52
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
