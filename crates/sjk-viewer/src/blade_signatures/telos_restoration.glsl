// Telos Restoration (rebuilt 3/11/2026): The Telos restoration, life returned to a burned world: a clean white blade with green rising up it in a slow year. Living shoots climb round it opening leaves as they go, pollen drifts off them, then they turn gold, wither and shed, and spring starts again.
// @part Clean light
// @part Climbing shoots
// @part Pollen
// @part Falling leaves
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail growth
// @palette {"corona.rim_cool":[0.099,0.473,0.143],"corona.rim_hot":[0.18,0.86,0.26],"corona.inner":[0.93,1,0.88],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.144,0.688,0.208],"core.fringe_hot":[0.93,1,0.88],"light.color":[0.18,0.86,0.26],"trail":[0.18,0.86,0.26]}
// @speed 1.0

const vec3 W = vec3(0.930, 1.000, 0.880);
const vec3 Gr = vec3(0.180, 0.860, 0.260);
const vec3 Dg = vec3(0.020, 0.200, 0.070);
const vec3 Au = vec3(1.000, 0.800, 0.320);
float te_top(){ return uLength*rxLife(0.0, 0.5, 0.32); }
float te_half(float y){
  float top = te_top();
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/1.2, 0.0, 1.0);
  return 0.46*sqrt(max(1.0 - e*e, 0.0));
}
// Spring climbs the blade in a 16 second year: 0 at the hand, 1 at the point, then it withers.
float te_year(){ return fract(uClock/16.0); }
float te_grow(){ float y = te_year(); return smoothstep(0.0, 0.5, y)*(1.0 - smoothstep(0.82, 1.0, y)); }
float te_wither(){ float y = te_year(); return smoothstep(0.66, 0.84, y)*(1.0 - smoothstep(0.9, 1.0, y)); }
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  float top = te_top();
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - te_half(yc), 0.0);
  float front = te_grow()*uLength;
  float lit = 1.0 - smoothstep(front - 2.0, front + 3.0, b.y);
  vec3 col = W*halo(d, 0.2)*0.6 + mix(W*0.45, Gr, lit)*(halo(d, 1.2)*0.5 + halo(d, 3.6)*0.18);
  col += lvl(g)*W*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.7*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = te_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  vec3 col = W;
  if (part(0.0) > 0.5){
    float front = te_grow()*L;
    float v = rxFlow(vec2(b.x*1.5, b.y*0.09 - uClock*0.1));
    float vein = 1.0 - smoothstep(0.0, 0.1, abs(v - 0.5));
    col = mix(W, mix(Gr, W, 0.2), vein*(1.0 - smoothstep(front - 2.0, front + 1.0, b.y)));
    col += Gr*exp(-pow((b.y - front)/1.2, 2.0))*(1.0 - te_wither())*0.6;
  }
  col *= 0.68 + 0.5*nz;
  col = mix(col, W, 0.45*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
// A climbing shoot: its line up the blade, how far it has grown, and leaves opening along it.
vec3 te_shoot(vec3 ro, vec3 rd, float px, float k){
  float L = uLength;
  float grown = te_grow(), w = te_wither();
  float len = grown*(0.72 + 0.28*h11(k*2.9));
  if (len < 0.02) return vec3(0.0);
  float y0 = 0.3 + 0.9*k, a0 = k/3.0 + 0.1;
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(0.7, a0, y0);
  for (int j = 1; j <= 16; j++){
    float f = float(j)/16.0;
    if (f - 1.0/16.0 > len) break;
    vec3 q = around(0.8 + 1.15*sin(f*9.0 + k*2.0)*sin(f*3.1416) + 0.25*f, a0 + 0.8*f + 0.02*sin(uClock*0.4 + k), mix(y0, L - 1.0, f));
    float dd = raySeg(ro, rd, prev, q, s);
    { float wN = bW(dd); bWs += wN; best = min(best, dd); bf += ((float(j) - 1.0 + s)/16.0 - bf)*wN/bWs; }
    prev = q;
  }
  float ww = mix(0.15, 0.05, bf)*(1.0 - 0.3*w);
  vec3 stem = mix(Gr, Au*0.75, w);
  vec3 col = mix(stem, W, 0.25)*(1.0 - smoothstep(ww*0.5, ww + px, best))*1.25 + stem*(halo(best, ww*3.0)*0.45 + halo(best, 0.7)*0.1);
  // the leaves
  for (int i = 0; i < 9; i++){
    float m = float(i);
    float f = (m + 0.5 + 0.3*h11(m*3.3 + k))/9.0;
    float open = smoothstep(f, f + 0.16, len)*(1.0 - 0.4*w);
    if (open <= 0.0) continue;
    vec3 at = around(0.8 + 1.15*sin(f*9.0 + k*2.0)*sin(f*3.1416) + 0.25*f, a0 + 0.8*f + 0.02*sin(uClock*0.4 + k), mix(y0, L - 1.0, f));
    float dir = m*2.4 + k;
    vec3 tip = at + vec3(sin(dir), 0.7 - 0.8*w, cos(dir))*(1.1 + 0.7*h11(m*5.1 + k))*open;
    vec3 mid = mix(at, tip, 0.55) + vec3(0.0, 0.14, 0.0);
    float s1, s2;
    float d1 = raySeg(ro, rd, at, mid, s1), d2 = raySeg(ro, rd, mid, tip, s2);
    float wl = d1 < d2 ? 0.3*open*smoothstep(0.0, 1.0, s1) : 0.3*open*(1.0 - s2*s2);
    float dl = min(d1, d2);
    vec3 leaf = mix(mix(Gr, W, 0.32), Au, w);
    col += leaf*(1.0 - smoothstep(wl*0.65, wl + px + 0.02, dl))*1.1 + leaf*halo(dl, 0.42)*0.14;
  }
  return col;
}
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.25, 1.0, 0.35)*(1.0 - smoothstep(0.92, 1.0, te_year()));
  if (aft <= 0.01) return fx;
  if (part(1.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 4.5) for (int i = 0; i < 3; i++) fx += te_shoot(ro, rd, px, float(i));
  if (part(2.0) > 0.5) fx += 1.25*gxKin(ro, rd, 1.0, 7.0, 2.60, 0.050, 0.16, 0.08, 0.96, 1.0, 8092.0, W, Gr);
  if (part(3.0) > 0.5) fx += te_wither()*1.3*shFlakes(ro, rd, 3.0, 0.5, L - 0.5, 16.0, 2.8, -0.6, 0.12, 0.12, 0.35, 0.5, 112.0, Au, Dg*3.0);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.32, 4.49, 0.014, 632.0, vec3(0.180, 0.860, 0.260)*0.24, vec3(0.099, 0.473, 0.143)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.180, 0.860, 0.260)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
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
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  a += 1.1*gxTumble(ro, rd, px, 8.0, 4.0, 2.0, 1.0, 0.26, 101.0, vec3(0.930, 1.000, 0.880), vec3(0.180, 0.860, 0.260));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

