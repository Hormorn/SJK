// Chirrut Whills (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): Chirrut Imwe saw nothing and knew everything. Each tap of his staff sends a wave of sound up the dark blade, and the motes in the air round it exist only while the wave passes: touched by it they light, then go dark again. Rain falls, seen only the same way.
// @part Guardian's staff
// @part Sound through it
// @part Seen by sound
// @part Falling rain
// @part The Force
// @part Guardian's cloth
// @raycache fast
// @trail gale
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.45,0.55,0.75],"corona.rim_hot":[0.8,0.88,1.0],"corona.inner":[0.95,0.97,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.6,0.7,0.9],"core.fringe_hot":[0.88,0.93,1.0],"light.color":[0.75,0.85,1.0],"trail":[0.78,0.86,1.0]}
// @speed 1.0

const vec3 M = vec3(0.540, 0.600, 0.760);
const vec3 D = vec3(0.100, 0.120, 0.200);
const vec3 H = vec3(1.000, 1.000, 1.000);
const vec3 A = vec3(0.700, 0.820, 1.000);

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
  vec3 col = M*(halo(d, 0.16)*0.5 + halo(d, 0.8)*0.34 + halo(d, 2.4)*0.07);
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
    float pl = fract(C*0.26)*(L + 8.0) - 2.0;
    col = mix(D, M, 0.25 + 0.6*kf(x*2.2, y*0.14));
    if (part(1.0) > 0.5){ float dy = y - pl; col += A*halo(abs(dy), 1.2)*1.2 + H*halo(abs(dy), 0.3)*0.9; col += A*(1.0 - smoothstep(0.0, 8.0, pl - y))*step(y, pl)*0.2; }
  }
  col *= (0.45 + 0.7*nz)*1.5;
  col += M*pow(1.0 - nz, 3.0)*0.7;
  col = mix(col, H, (1.0 - smoothstep(0.0, 0.55, abs(u)))*0.75);
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
  float pl = fract(C*0.26)*(L + 8.0) - 2.0;
  if (part(2.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 7.0) for (int i = 0; i < 70; i++){ float k = float(i); vec3 p = around(1.2 + 3.2*h11(k*3.1), h11(k*5.3) + 0.004*C, L*h11(k*7.7)); float seen = 0.16 + 0.84*halo(abs(p.y - pl), 2.0 + length(p.xz)*0.25); float d = rayPt(ro, rd, p, t); fx += (H*halo(d, max(0.055, px))*2.0 + A*halo(d, 0.28)*0.55)*seen; }
  if (part(3.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 6.0) for (int i = 0; i < 16; i++){ float k = float(i); float yy = L*fract(h11(k*2.7) - C*0.5*(0.7 + 0.3*h11(k*1.3))); vec3 p0 = around(1.0 + 2.4*h11(k*8.1), h11(k*4.9), yy); float s2; float d = raySeg(ro, rd, p0, p0 + vec3(0.0, 0.8, 0.0), s2); fx += A*halo(d, max(0.025, px))*0.8*halo(abs(yy - pl), 2.2); }
  if (part(4.0) > 0.5) fx += 0.06*fxAura(ro, rd, 0.0, L, 1.1, A);
  if (part(4.0) > 0.5) fx += 0.45*syMist(ro, rd, L*0.1, L, 2.8, 0.06, 3.0, A*0.5, M*0.4);
  if (part(5.0) > 0.5) for (int i = 0; i < 2; i++){ float k = float(i); fx += 1.15*rxRibbon(ro, rd, 1.1, 2.6, 0.75, k*0.5 + 0.009*T, L*(0.03 + 0.18*k), L*(0.72 + 0.2*k), 0.42, k + 21.0, vec3(1.0, 0.42, 0.22), vec3(0.75, 0.08, 0.06)); }
  if (part(4.0) > 0.5){ fx += (0.35 + 0.5*halo(abs(fract(C*0.26)*(L + 8.0) - 2.0 - L*0.5), L*0.5))*rxAuraShell(ro, rd, -0.3, L + 0.3, 2.6, 3.0, M*0.8, A); fx += 1.2*rxBorn(ro, rd, px, 14.0, 0.5, L - 0.5, 0.35, 2.4, 1.5, 0.05, 15.0, H, A); }
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
const vec3 SX_HOT = vec3(0.800, 0.880, 1.000);
const vec3 SX_DEEP = vec3(0.450, 0.550, 0.750);
const vec3 SX_PALE = vec3(0.950, 0.970, 1.000);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 3.0, 6.0, 2.40, 0.050, 0.18, 0.08, 0.96, 1.0, 7182.0, vec3(1.0), SX_HOT);a += sxDrops(ro, rd, px, 8.0, 2.6, 0.1, 377.0, SX_PALE, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(5.0) > 0.5){
  a += 1.5*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.44, 4.36, -0.017, 741.0, vec3(0.800, 0.880, 1.000)*0.24, vec3(0.450, 0.550, 0.750)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.800, 0.880, 1.000)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(5.0) > 0.5){
    a += 1.3*syPetalsAir(ro, rd, 8.0, 3.03, 0.045, 1541.0, vec3(0.950, 0.970, 1.000), vec3(0.800, 0.880, 1.000));
    a += 1.4*syBubbles(ro, rd, 8.0, 3.85, 0.043, 1538.0, vec3(0.950, 0.970, 1.000), vec3(0.800, 0.880, 1.000));
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
  a += 1.1*gxHelix(ro, rd, px, 12.0, 2.2, 1.3, 0.03, 71.0, vec3(0.950, 0.970, 1.000), vec3(0.800, 0.880, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

