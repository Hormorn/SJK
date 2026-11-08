// Dxun Beastfang (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): the hide of a Dxun beast pulled over a blade: orange fur barred with heavy black stripes, its flanks rising and falling, embers and warm jungle haze round it.
// @part Beast's hide
// @part Hide breathing
// @part Rising embers
// @part Hunting haze
// @raycache fast
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.55,0.275,0.028],"corona.rim_hot":[1,0.55,0.055],"corona.inner":[1,0.75,0.525],"core.white":[1.0,0.9,0.7],"core.fringe_cool":[0.8,0.4,0.04],"core.fringe_hot":[1,0.9,0.36],"light.color":[1.0,0.5,0.05],"trail":[1.0,0.5,0.05]}
// @speed 1.0

const vec3 M = vec3(1.000, 0.480, 0.050);
const vec3 D = vec3(0.040, 0.030, 0.020);
const vec3 H = vec3(1.000, 0.840, 0.500);
const vec3 A = vec3(0.900, 0.260, 0.030);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.5 + 0.12*sin(y*0.22 + sd) + 0.24*smoothstep(0.92, 0.35, s)*smoothstep(0.0, 0.2, s)*0.2;
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
    float w = y*0.22 + ang*0.75 + 1.2*kf(ang*0.7, y*0.1);
    float bar = smoothstep(0.35, 0.65, sin(w*2.0))*smoothstep(-0.8, 0.2, sin(y*0.31 + ang*1.3));
    col = mix(M*(0.72 + 0.4*kf(x*7.0, y*0.8)), D, bar);
    if (part(1.0) > 0.5) col *= 0.82 + 0.3*sin(T*0.9 + y*0.1);
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
vec3 kLine(vec3 ro, vec3 rd, float px, vec3 a, vec3 c, float sag, float w, vec3 hot, vec3 col, float flow){
  float tm;
  if (rayPtRaw(ro, rd, mix(a, c, 0.5), tm) > 0.5*length(c - a) + abs(sag) + 1.2) return vec3(0.0);
  vec3 prev = a;
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s;
  for (int i = 1; i <= 7; i++){
    float f = float(i)/7.0;
    vec3 q = mix(a, c, f) + vec3(0.0, sag*4.0*f*(1.0 - f), 0.0);
    float d = raySeg(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(i) - 1.0 + s)/7.0 - bf)*wN/bWs; }
    prev = q;
  }
  float bead = pow(0.5 + 0.5*sin(bf*9.0 - uClock*flow), 6.0);
  return (hot*halo(best, max(w, px))*(0.5 + 1.2*bead) + col*halo(best, w*5.0)*(0.3 + 0.6*bead))*smoothstep(0.0, 0.08, bf)*(1.0 - smoothstep(0.85, 1.0, bf));
}
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  if (part(2.0) > 0.5) fx += 1.0*fxMotes(ro, rd, px, L*0.05, L*0.95, 1.3, 2.2, 8.0, 0.22, 74.0, A);
  if (part(3.0) > 0.5) fx += 0.22*syMist(ro, rd, L*0.1, L*0.95, 3.2, 0.2, 7.0, A*0.45, D);
  if (part(2.0) > 0.5) for (int i = 0; i < 5; i++){ float k = float(i); fx += 0.9*rxTongue(ro, rd, L*(0.09 + 0.18*k), k*0.41 + 0.01*C, 3.2, 1.3, 0.4, k + 6.0, H, M, A); }
  if (part(2.0) > 0.5) for (int i = 0; i < 2; i++){ float k = float(i); float cyc = C*(0.19 + 0.05*k) + k*0.47; float n = floor(cyc), q = fract(cyc); float rk = smoothstep(0.0, 0.14, q)*pow(1.0 - q, 2.0); if (rk > 0.01){ float a0 = h11(n*3.1 + k), y0 = L*(0.2 + 0.6*h11(n*1.7 + k)); for (int j = 0; j < 3; j++){ float m = float(j); fx += 1.6*rk*kLine(ro, rd, px, around(1.0, a0 - 0.05, y0 + 2.6 + m*0.8), around(2.2, a0 + 0.15, y0 - 2.6 + m*0.8), 0.25, 0.05, H, M, 1.0); } } }
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
const vec3 SX_HOT = vec3(1.000, 0.550, 0.055);
const vec3 SX_DEEP = vec3(0.550, 0.275, 0.028);
const vec3 SX_PALE = vec3(1.000, 0.750, 0.525);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 1.0*gxKin(ro, rd, 6.0, 8.0, 2.00, 0.050, 0.70, 0.08, 0.96, 1.0, 7286.0, SX_PALE, SX_HOT);a += 0.9*xPuffs(ro, rd, 2.3, 1.3, 1.0, C*0.016, L*0.06, L*0.96, 0.46, 371.0, SX_PALE, SX_HOT*0.5);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.4*sxFlames(b, 0.65, 1.6, 1.5, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.20, 4.41, -0.009, 645.0, vec3(1.000, 0.550, 0.055)*0.24, vec3(0.550, 0.275, 0.028)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.550, 0.055)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(3.0) > 0.5){
    a += 1.4*vec3(0.0);
    a += 1.3*vec3(0.0);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  a += 1.1*gxSwarm(ro, rd, px, 8.0, 4.0, 94.0, vec3(1.000, 0.750, 0.525), vec3(1.000, 0.550, 0.055));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

