// Sarlacc Pit (new 3/11/2026, one of the last 28): The Great Pit of Carkoon: a ribbed pink throat swallowing downward into pale desert. Tendrils reach out of the low end of it and sand blows round the top.
// @part Throat
// @part Reaching tendrils
// @part Blown sand
// @raycache fast
// @trail plague
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.506,0.248,0.253],"corona.rim_hot":[0.92,0.45,0.46],"corona.inner":[1,0.86,0.72],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.736,0.36,0.368],"core.fringe_hot":[1,0.86,0.72],"light.color":[0.92,0.45,0.46],"trail":[0.92,0.45,0.46]}
// @speed 1.0

const vec3 M = vec3(0.740, 0.560, 0.320);
const vec3 D = vec3(0.200, 0.090, 0.080);
const vec3 H = vec3(1.000, 0.860, 0.720);
const vec3 A = vec3(0.920, 0.450, 0.460);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.6;
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
  vec3 col = M*(halo(d, 0.16)*0.2 + halo(d, 0.8)*0.1 + halo(d, 2.4)*0.02);
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
    float w = 0.5 + 0.5*sin(y*2.1 + 0.8*sin(ang*2.0) + T*0.9);
    col = mix(D*1.5, A, 0.25 + 0.45*w);
    col = mix(col, M, smoothstep(0.35, 0.7, s)*0.75);
  }
  col *= (0.45 + 0.7*nz)*1.15;
  col += M*pow(1.0 - nz, 3.0)*0.3;
  col = mix(col, H, (1.0 - smoothstep(0.0, 0.55, abs(u)))*0.0);
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
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  if (part(1.0) > 0.5) for (int i = 0; i < 2; i++){ float k = float(i); fx += kVine(ro, rd, px, 0.85, 1.6, L*0.04, L*(0.42 + 0.2*k), 0.7, 0.5*k + C*0.018, 0.35, 0.08, A, D*2.0, 181.0 + k); }
  if (part(2.0) > 0.5) fx += 1.0*xDust(ro, rd, 2.5, 0.9, 0.8, C*0.02, L*0.45, L*0.96, 0.32, 183.0, M, D);
  // effects rise with the blade as it lights and sink with it as it goes out
  float sa;
  raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, L + 4.0, 0.0), sa);
  float grownTop = (L + 6.0)*rxLife(0.0, 0.55, 0.32);
  fx *= 1.0 - smoothstep(grownTop - 4.0, grownTop, sa*(L + 4.0));
  // nothing over the point
  fx *= tipFade(ro, rd);
  return fx*aft*uFx*hiltFade(ro, rd);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(2.0) > 0.5){
  a += 1.4*shFlakes(ro, rd, 3.25, 0.5, L - 0.5, 14.1, 2.8, 0.29, -0.023, 0.08, 0.55, 0.0, 814.0, vec3(0.920, 0.450, 0.460), vec3(0.506, 0.248, 0.253));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.920, 0.450, 0.460)*wv*(halo(d, 0.9)*0.21 + halo(d, 2.6)*0.13)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
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
