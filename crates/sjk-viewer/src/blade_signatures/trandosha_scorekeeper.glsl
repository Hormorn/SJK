// Trandosha Scorekeeper (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): Trandoshan scales running up the blade, overlapping and glinting, with a scored ritual count burning between them; a red mist hangs round it for the Scorekeeper.
// @part Scaled hide
// @part Ritual score
// @part Blood mist
// @part Clawed spray
// @raycache fast
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.55,0.055,0.066],"corona.rim_hot":[1,0.11,0.132],"corona.inner":[1,0.55,0.56],"core.white":[1.0,0.92,0.9],"core.fringe_cool":[0.8,0.08,0.096],"core.fringe_hot":[1,0.42,0.444],"light.color":[1.0,0.1,0.12],"trail":[1.0,0.1,0.12]}
// @speed 1.0

const vec3 M = vec3(0.720, 0.070, 0.080);
const vec3 D = vec3(0.130, 0.020, 0.030);
const vec3 H = vec3(1.000, 0.720, 0.550);
const vec3 A = vec3(0.320, 0.500, 0.140);

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
    float sc = 0.5 + 0.5*sin(ang*5.0 + y*0.42)*sin(ang*5.0 - y*0.42);
    col = mix(D*2.0, M, 0.25 + 0.75*sc);
    col = mix(col, A, 0.25*kf(x*1.4, y*0.14));
    if (part(1.0) > 0.5) col += H*smoothstep(0.6, 0.95, sc)*pow(0.5 + 0.5*sin(y*0.28 - T*1.2), 4.0)*0.8;
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
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  if (part(2.0) > 0.5) fx += 0.3*syMist(ro, rd, L*0.1, L*0.95, 2.4, 0.2, 9.0, M*0.7, D);
  if (part(3.0) > 0.5) fx += 0.9*fxSparks(ro, rd, px, L*0.2, L*0.95, 7.0, 0.8, 0.9, 62.0, M);
  if (part(3.0) > 0.5) for (int i = 0; i < 5; i++){ float k = float(i); fx += 0.8*rxTongue(ro, rd, L*(0.09 + 0.18*k), k*0.41 + 0.01*C, 3.4, 1.4, 0.34, k + 8.0, H, M, D); }
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
const vec3 SX_HOT = vec3(1.000, 0.110, 0.132);
const vec3 SX_DEEP = vec3(0.550, 0.055, 0.066);
const vec3 SX_PALE = vec3(1.000, 0.550, 0.560);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 0.9*xPuffs(ro, rd, 2.5, 1.3, 1.0, C*0.012, L*0.06, L*0.96, 0.46, 317.0, SX_PALE, SX_HOT*0.5);a += vec3(0.0);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.6*gxKin(ro, rd, 7.0, 5.0, 2.20, 0.040, 0.28, 0.08, 0.96, 1.0, 8118.0, vec3(1.000, 0.550, 0.560), vec3(1.000, 0.110, 0.132));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.110, 0.132)*wv*(halo(d, 0.9)*0.21 + halo(d, 2.6)*0.13)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(3.0) > 0.5){
    a += 1.3*vec3(0.0);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
