// Umbara Gloaming (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): Umbara, the shadow world where no sun reaches: dark indigo fading to nothing, rows of bioluminescent lume spots waking along it, silent lightning far off inside it, a cold mist hanging round it from hilt to point.
// @part Shadow world
// @part Lume spots
// @part Far lightning
// @part Dark mist
// @part Lost lights
// @raycache fast
// @trail nebula
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.2,0.15,0.35],"corona.rim_hot":[0.6,0.45,0.95],"corona.inner":[0.85,0.8,1.0],"core.white":[0.95,0.93,1.0],"core.fringe_cool":[0.35,0.3,0.6],"core.fringe_hot":[0.7,0.6,1.0],"light.color":[0.55,0.45,0.9],"trail":[0.5,0.42,0.85]}
// @speed 2.6

const vec3 M = vec3(0.160, 0.100, 0.320);
const vec3 D = vec3(0.020, 0.010, 0.050);
const vec3 H = vec3(0.920, 0.860, 1.000);
const vec3 A = vec3(0.500, 0.320, 0.980);

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
  vec3 col = A*(halo(d, 0.16)*0.75 + halo(d, 0.8)*0.3 + halo(d, 2.4)*0.06);
  col += lvl(g)*A*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.28;
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
    col = mix(D, M, 0.25 + 0.75*kf(x*1.1, y*0.1 - T*0.09));
    if (part(1.0) > 0.5){ vec2 c = vec2(ang*2.6, y*0.42); vec2 id = floor(c); vec2 f = fract(c) - 0.5; float sp = halo(length(f - (h22(id) - 0.5)*0.5), 0.15)*step(0.42, h21(id)); col += A*sp*(0.3 + 1.2*pow(0.5 + 0.5*sin(C*0.7 + 6.28*h21(id + 3.0)), 3.0)); }
    if (part(2.0) > 0.5){ float cyc = C*0.31; float q = fract(cyc); float yy = L*h11(floor(cyc)*3.1); col += H*halo(abs(y - yy), 5.0)*pow(max(0.0, 1.0 - q*4.0), 2.0)*0.7; }
  }
  col *= (0.7 + 0.45*nz)*1.5;
  col += A*pow(1.0 - nz, 3.0)*1.0;
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
  if (part(3.0) > 0.5){ fx += 0.5*syMist(ro, rd, L*0.1, L*0.95, 3.2, 0.08, 9.0, A*0.5, M*0.6); fx += 0.95*rxAurora(ro, rd, 2.3, L*0.05, L*0.98, 5.0, 4.0, A, M*1.6); }
  if (part(4.0) > 0.5) fx += 1.3*fxMotes(ro, rd, px, L*0.04, L*0.95, 1.4, 3.4, 12.0, 0.08, 57.0, A);
  if (part(2.0) > 0.5) fx += vec3(0.0);
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
const vec3 SX_HOT = vec3(0.600, 0.450, 0.950);
const vec3 SX_DEEP = vec3(0.200, 0.150, 0.350);
const vec3 SX_PALE = vec3(0.850, 0.800, 1.000);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 5.0, 6.0, 2.80, 0.030, 0.70, 0.08, 0.96, 1.0, 8170.0, SX_PALE, SX_HOT); a += vec3(0.0);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(4.0) > 0.5){
  a += 1.4*shFlakes(ro, rd, 3.02, 0.5, L - 0.5, 16.0, 2.8, 0.25, -0.039, 0.08, 0.55, 0.0, 758.0, vec3(0.600, 0.450, 0.950), vec3(0.200, 0.150, 0.350));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.54, 4.21, -0.016, 761.0, vec3(0.600, 0.450, 0.950)*0.24, vec3(0.200, 0.150, 0.350)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.600, 0.450, 0.950)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(4.0) > 0.5){
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
  float L = uLength;
  a += 1.42*gxKin(ro, rd, 3.0, 5.0, 3.00, 0.030, 0.20, 0.08, 0.96, -1.0, 9391.0, vec3(0.850, 0.800, 1.000), vec3(0.600, 0.450, 0.950));
  a += 1.2*gxVortex(ro, rd, px, 13.0, 5.4, 39.0, vec3(0.850, 0.800, 1.000), vec3(0.600, 0.450, 0.950));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

