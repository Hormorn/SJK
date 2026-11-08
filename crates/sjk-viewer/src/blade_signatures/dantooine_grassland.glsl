// Dantooine Grassland (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): the plains of Dantooine: long grass bending in slow waves up the blade, patches of sun crossing it, seed fluff lifting away on the wind.
// @part Tall grass
// @part Sun patches
// @part Seed fluff
// @raycache fast
// @trail growth
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.2,0.5,0.15],"corona.rim_hot":[0.55,0.9,0.4],"corona.inner":[0.88,1.0,0.8],"core.white":[0.97,1.0,0.93],"core.fringe_cool":[0.3,0.65,0.2],"core.fringe_hot":[0.7,0.95,0.55],"light.color":[0.5,0.85,0.35],"trail":[0.5,0.85,0.35]}
// @speed 1.0

const vec3 M = vec3(0.400, 0.750, 0.280);
const vec3 D = vec3(0.080, 0.260, 0.080);
const vec3 H = vec3(0.900, 1.000, 0.700);
const vec3 A = vec3(0.950, 0.880, 0.500);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.55;
  float top = L*rxLife(0.0, 0.45, 0.32);
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/max(w*1.0, 0.05), 0.0, 1.0);
  return w*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float yc = clamp(b.y, 0.0, L*rxLife(0.0, 0.45, 0.32));
  float d = max(length(vec2(b.x, b.y - yc)) - k_half(yc, b.x < 0.0 ? -1.0 : 1.0), 0.0);
  float T = uTime, C = uClock, y = b.y, s = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  vec3 col = M*(halo(d, 0.16)*0.4 + halo(d, 0.8)*0.2 + halo(d, 2.4)*0.035);
  col += lvl(g)*M*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.28;
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
    float wave = 2.0*sin(y*0.1 - T*0.4);
    col = mix(D, M, 0.5 + 0.5*sin(ang*9.0 + wave));
    if (part(1.0) > 0.5) col = mix(col, A, smoothstep(0.58, 0.8, kf(x*0.8, y*0.08 - T*0.09))*0.65);
  }
  col *= (0.45 + 0.7*nz)*1.35;
  col += M*pow(1.0 - nz, 3.0)*0.5;
  col = mix(col, H, (1.0 - smoothstep(0.0, 0.55, abs(u)))*0.6);
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
vec3 kLeaf(vec3 ro, vec3 rd, float px, float y, float a, float len, float curl, float w, vec3 hot, vec3 col){
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(0.5, a, y);
  for (int i = 1; i <= 6; i++){
    float f = float(i)/6.0;
    vec3 q = around(0.5 + len*f*0.75, a + 0.04*sin(uClock*0.5 + a*20.0)*f, y + len*f*(0.85 - curl*f));
    float d = raySeg(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(i) - 1.0 + s)/6.0 - bf)*wN/bWs; }
    prev = q;
  }
  float ww = w*(1.0 - bf*bf)*smoothstep(0.0, 0.12, bf) + 0.01;
  return hot*(1.0 - smoothstep(ww*0.6, ww + px, best))*0.85 + col*halo(best, ww*3.0)*0.42;
}
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  if (part(2.0) > 0.5) fx += gxKin(ro, rd, 1.0, 8.0, 2.80, 0.060, 0.20, 0.08, 0.96, 1.0, 7260.0, A, H*0.4);
  if (part(1.0) > 0.5) for (int i = 0; i < 8; i++){ float k = float(i); fx += kLeaf(ro, rd, px, L*(0.02 + (0.5 - 0.02)*fract(k*0.618)), k*0.382 + 0.0, 3.2*(0.7 + 0.5*h11(k*3.1)), 0.35, 0.05, M*1.2, A); }
  // effects rise with the blade as it lights and sink with it as it goes out
  float sa;
  raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, L + 4.0, 0.0), sa);
  float grownTop = (L + 6.0)*rxLife(0.0, 0.55, 0.32);
  fx *= 1.0 - smoothstep(grownTop - 4.0, grownTop, sa*(L + 4.0));
  // nothing over the point
  fx *= tipFade(ro, rd);
  return fx*aft*uFx*hiltFade(ro, rd);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(0.550, 0.900, 0.400);
const vec3 SX_DEEP = vec3(0.200, 0.500, 0.150);
const vec3 SX_PALE = vec3(0.880, 1.000, 0.800);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 1.0*xDust(ro, rd, 2.4, 1.2, 1.0, C*0.025, L*0.06, L*0.96, 0.36, 404.0, SX_PALE, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(2.0) > 0.5){
  a += 1.1*shStreaks(ro, rd, 2.71, L*0.05, L*0.95, 10.4, 3.0, 1.70, 0.4, 587.0, vec3(0.880, 1.000, 0.800), vec3(0.550, 0.900, 0.400));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.550, 0.900, 0.400)*wv*(halo(d, 0.9)*0.21 + halo(d, 2.6)*0.13)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  if (part(2.0) > 0.5){
    col += vec3(0.0);
  }
  return col;
}
