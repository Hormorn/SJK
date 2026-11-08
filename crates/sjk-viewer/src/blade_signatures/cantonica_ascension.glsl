// Cantonica Champagne (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): the casinos of Canto Bight: a blade of struck gold, pale champagne bubbles streaming up inside it and bursting out into the air, a sweep of white light passing over the metal as if under chandeliers, gold glitter falling.
// @part Struck gold
// @part Rising bubbles
// @part Sweep of light
// @part Bubbles and glitter
// @part Casino glow
// @raycache fast
// @trail gilded
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.65,0.42,0.0],"corona.rim_hot":[1.0,0.85,0.25],"corona.inner":[1.0,0.97,0.75],"core.white":[1.0,1.0,0.92],"core.fringe_cool":[0.9,0.65,0.05],"core.fringe_hot":[1.0,0.92,0.5],"light.color":[1.0,0.85,0.3],"trail":[1.0,0.85,0.3]}
// @speed 1.0

const vec3 M = vec3(0.880, 0.660, 0.140);
const vec3 D = vec3(0.300, 0.160, 0.030);
const vec3 H = vec3(1.000, 0.980, 0.820);
const vec3 A = vec3(1.000, 0.860, 0.320);

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
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float yc = clamp(b.y, 0.0, L*rxLife(0.0, 0.45, 0.32));
  float d = max(length(vec2(b.x, b.y - yc)) - k_half(yc, b.x < 0.0 ? -1.0 : 1.0), 0.0);
  float T = uTime, C = uClock, y = b.y, s = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  vec3 col = M*(halo(d, 0.16)*0.55 + halo(d, 0.8)*0.3 + halo(d, 2.4)*0.06);
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
    col = mix(D, M, 0.6 + 0.4*sin(ang*11.0 + 3.0*kf(x, y*0.05)));
    col *= 0.85 + 0.3*kf(x*6.0, y*0.2);
    if (part(1.0) > 0.5){ vec2 c = vec2(ang*3.2, y*0.42 - T*0.55); vec2 id = floor(c); vec2 f = fract(c) - 0.5; float bub = kl(length(f - (h22(id) - 0.5)*0.4) - 0.14, 0.05)*step(0.4, h21(id + 4.0)); col += H*bub*0.9; }
    if (part(2.0) > 0.5) col += H*smoothstep(0.86, 1.0, sin(y*0.1 - C*0.5 + ang*1.6))*0.85;
  }
  col *= (0.45 + 0.7*nz)*1.5;
  col += M*pow(1.0 - nz, 3.0)*0.6;
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
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  if (part(3.0) > 0.5){ for (int i = 0; i < 16; i++){ float k = float(i); float q = fract(C*(0.022 + 0.012*h11(k*2.3)) + h11(k*6.7)); vec3 p = around(2.4 + 2.6*h11(k*3.7), h11(k*5.1) + 0.012*C*(k < 8.0 ? 1.0 : -1.0), L*(0.03 + 0.95*q)); float d = rayPt(ro, rd, p, t); float r = 0.34 + 0.5*h11(k*9.1); vec3 c = mix(mix(A, H, h11(k*1.3)), vec3(1.0, 0.72, 0.62), step(0.72, h11(k*8.3))); fx += c*halo(d, r)*(0.5 + 0.22*sin(C*0.9 + k*1.9))*sin(3.1416*q); }
   fx += 0.8*rxRays(ro, rd, px, 6.0, L*0.1, L*0.95, 3.4, 41.0, H*0.5, A*0.28); }
  if (part(4.0) > 0.5){ for (int i = 0; i < 2; i++){ float k = float(i); fx += 1.1*xPuffs(ro, rd, 1.3, 2.0, 0.9, k*0.5 + 0.02*C, L*0.05, L*0.97, 0.36, k + 43.0, H, A*0.7); fx += 1.2*xDust(ro, rd, 1.4, 2.1, 0.9, k*0.5 + 0.02*C, L*0.05, L*0.97, 0.22, k + 45.0, H, A); } fx += (0.12 + 0.2*mom)*fxAura(ro, rd, 0.0, L, 1.8, A); }
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
const vec3 SX_HOT = vec3(1.000, 0.850, 0.250);
const vec3 SX_DEEP = vec3(0.650, 0.420, 0.000);
const vec3 SX_PALE = vec3(1.000, 0.970, 0.750);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 4.0, 9.0, 3.60, 0.060, 0.24, 0.08, 0.96, 1.0, 7156.0, vec3(1.0, 0.96, 0.8), SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(4.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.59, 5.19, 0.012, 744.0, vec3(1.000, 0.850, 0.250)*0.24, vec3(0.650, 0.420, 0.000)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.850, 0.250)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(4.0) > 0.5){
    a += 1.3*vec3(0.0);
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
  a += 1.1*gxLanterns(ro, rd, px, 5.0, 3.8, 80.0, vec3(1.000, 0.970, 0.750), vec3(1.000, 0.850, 0.250));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

