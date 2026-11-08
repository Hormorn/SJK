// Rishi Ocean Flow (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): the storm seas of Rishi in a blade: deep blue water climbing it with light netted through it, great waves rearing out of it and curling back, bright spray thrown high and falling, a soft sea mist round it all.
// @part Deep water
// @part Great waves
// @part Bright spray
// @part Sea mist
// @raycache fast
// @trail gale
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.0,0.15,0.5],"corona.rim_hot":[0.1,0.5,0.95],"corona.inner":[0.6,0.85,1.0],"core.white":[0.9,0.97,1.0],"core.fringe_cool":[0.0,0.3,0.75],"core.fringe_hot":[0.4,0.75,1.0],"light.color":[0.1,0.45,0.95],"trail":[0.15,0.5,1.0]}
// @speed 1.0

const vec3 M = vec3(0.050, 0.320, 0.850);
const vec3 D = vec3(0.000, 0.090, 0.360);
const vec3 H = vec3(0.860, 0.960, 1.000);
const vec3 A = vec3(0.200, 0.650, 0.950);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.7;
  float top = L*rxLife(0.0, 0.45, 0.32);
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/max(w*1.0, 0.05), 0.0, 1.0);
  return w*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float yc = clamp(b.y, 0.0, L*rxLife(0.0, 0.45, 0.32));
  float d = max(length(vec2(b.x, b.y - yc)) - k_half(yc, b.x < 0.0 ? -1.0 : 1.0), 0.0);
  float T = uTime, C = uClock, y = b.y, s = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  vec3 col = M*(halo(d, 0.16)*0.4 + halo(d, 0.8)*0.32 + halo(d, 2.4)*0.06);
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
    float v = kf(x*1.2, y*0.1 - T*0.3);
    col = mix(D, M, smoothstep(0.2, 0.8, v));
    float c = kf(x*3.0 + T*0.25, y*0.4 - T*0.2) + kf(x*3.6 - T*0.2, y*0.5 + 3.0);
    col += H*kl(c - 1.0, 0.05)*0.8;
  }
  col *= (0.45 + 0.7*nz)*1.5;
  col += M*pow(1.0 - nz, 3.0)*0.5;
  col = mix(col, H, (1.0 - smoothstep(0.0, 0.55, abs(u)))*0.45);
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
  if (part(1.0) > 0.5) for (int i = 0; i < 7; i++){ float k = float(i); fx += 0.95*rxTongue(ro, rd, L*(0.09 + 0.14*k), k*0.27 + 0.012*C, 4.4, 2.1, 0.55, k + 12.0, H, A, M); }
  if (part(2.0) > 0.5){ fx += 1.4*rxBorn(ro, rd, px, 22.0, 1.0, L*0.95, 0.6, 3.0, -1.4, 0.05, 21.0, H, A); fx += 1.0*shCaustic(ro, rd, 1.6, 0.5, L - 0.5, 1.6, 0.25, 11.0, H, A); }
  if (part(3.0) > 0.5) fx += 0.45*syMist(ro, rd, L*0.1, L*0.95, 3.0, 0.15, 5.0, A*0.6, M*0.4);
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
const vec3 SX_HOT = vec3(0.100, 0.500, 0.950);
const vec3 SX_DEEP = vec3(0.000, 0.150, 0.500);
const vec3 SX_PALE = vec3(0.600, 0.850, 1.000);
vec3 sig_vol_e1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += sxDrops(ro, rd, px, 12.0, 2.2, 0.16, 204.0, SX_PALE, SX_HOT); a += 1.1*xPuffs(ro, rd, 2.0, 3.0, 1.0, C*0.025, L*0.05, L*0.96, 0.6, 205.0, SX_PALE, SX_HOT*0.5);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
// >>> e1: the great effect round this blade (4/11/2026)
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the swell: great soft sheets of sea rearing slowly round it, netted with light, with foam and flying water along their crests
  a += 1.6*xDust(ro, rd, 2.1, 3.7, 0.65, C*0.016, L*0.03, L*0.97, 0.4, 425.0, H, A);
  a += 1.5*xDust(ro, rd, 3.6, 2.2, -0.55, -C*0.014 + 0.45, L*0.03, L*0.97, 0.4, 426.0, H, A);
  a += 0.5*shCaustic(ro, rd, 2.7, L*0.05, L*0.95, 2.6, 0.5, 427.0, H*0.6, A*0.5);

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  a += 1.5*syBubbles(ro, rd, 11.0, 3.6, 0.05, 421.0, H, A);
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  if (part(3.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.23*gxKin(ro, rd, 2.0, 10.0, 3.20, 0.220, 0.25, 0.08, 0.96, -1.0, 9306.0, vec3(0.600, 0.850, 1.000), vec3(0.100, 0.500, 0.950));
  a += 1.15*gxKin(ro, rd, 1.0, 8.0, 3.00, 0.160, 0.18, 0.08, 0.96, 1.0, 7936.0, vec3(0.600, 0.850, 1.000), vec3(0.100, 0.500, 0.950));
  a += 1.0*gxRain(ro, rd, px, 12.0, 5.2, 35.0, vec3(0.600, 0.850, 1.000), vec3(0.100, 0.500, 0.950));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

