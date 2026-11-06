// Onderon Blazing Strike (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): a blade on fire from within: slow rolling flame fills it, its edges licking outward, heat climbing off it.
// @part Rolling fire
// @part Licking edge
// @part Rising heat
// @raycache fast
// @trail flare
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.7,0.2,0.0],"corona.rim_hot":[1.0,0.55,0.1],"corona.inner":[1.0,0.85,0.55],"core.white":[1.0,0.95,0.85],"core.fringe_cool":[0.9,0.35,0.05],"core.fringe_hot":[1.0,0.7,0.3],"light.color":[1.0,0.5,0.1],"trail":[1.0,0.5,0.1]}
// @speed 1.0

const vec3 M = vec3(1.000, 0.450, 0.060);
const vec3 D = vec3(0.420, 0.080, 0.020);
const vec3 H = vec3(1.000, 0.920, 0.550);
const vec3 A = vec3(1.000, 0.720, 0.140);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.42 + 0.3*pow(kf(y*0.3 - T*0.7, sd*3.0), 2.0);
  float top = L*rxLife(0.0, 0.45, 0.32);
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/max(w*1.0, 0.05), 0.0, 1.0);
  return w*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_34(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float yc = clamp(b.y, 0.0, L*rxLife(0.0, 0.45, 0.32));
  float d = max(length(vec2(b.x, b.y - yc)) - k_half(yc, b.x < 0.0 ? -1.0 : 1.0), 0.0);
  float T = uTime, C = uClock, y = b.y, s = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  vec3 col = M*(halo(d, 0.16)*0.4 + halo(d, 0.8)*0.12 + halo(d, 2.4)*0.035);
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
    float v = kf(x*1.5, y*0.15 - T*0.5) + 0.5*kf(x*3.0, y*0.3 - T*0.8);
    col = mix(D, M, smoothstep(0.25, 0.8, v));
    if (part(1.0) > 0.5) col += H*smoothstep(0.85, 1.25, v)*0.9 + A*smoothstep(0.65, 1.0, abs(u))*0.4;
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
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  if (part(2.0) > 0.5) fx += 0.7*rxBorn(ro, rd, px, 10.0, 0.5, L*0.9, 0.4, 1.8, 2.8, 0.05, 37.0, H, A);
  if (part(2.0) > 0.5) for (int i = 0; i < 6; i++){ float k = float(i); fx += 0.9*rxTongue(ro, rd, L*(0.09 + 0.15*k), k*0.27 + 0.01*C, 3.6 + 0.8*h11(k), 1.0, 0.45, k + 3.0, H, M, D); }
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
const vec3 SX_HOT = vec3(1.000, 0.550, 0.100);
const vec3 SX_DEEP = vec3(0.700, 0.200, 0.000);
const vec3 SX_PALE = vec3(1.000, 0.850, 0.550);
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.5*sxFlames(b, 0.65, 1.8, 1.6, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(2.0) > 0.5){
  a += 1.6*gxKin(ro, rd, 1.0, 8.0, 2.20, 0.160, 0.16, 0.08, 0.96, 1.0, 7832.0, vec3(1.000, 0.850, 0.550), vec3(1.000, 0.550, 0.100));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.550, 0.100)*wv*(halo(d, 0.9)*0.21 + halo(d, 2.6)*0.13)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(2.0) > 0.5){
    a += 1.4*vec3(0.0);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
