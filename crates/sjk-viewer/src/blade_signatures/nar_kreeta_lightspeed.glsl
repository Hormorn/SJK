// Nar Kreeta Lightspeed (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): the jump to lightspeed held in a blade: gold star-drive plasma rushing up it, the stars around it drawn out into long lines that run past, blue ahead and gold behind. Every 18 seconds the jump: all the lines stretch and flash and the blade blazes.
// @part Star-drive plasma
// @part Star lines
// @part The jump
// @part Blue shift
// @raycache fast
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.55,0.484,0.11],"corona.rim_hot":[1,0.968,0.22],"corona.inner":[1,0.94,0.6],"core.white":[1.0,1.0,0.9],"core.fringe_cool":[0.8,0.704,0.16],"core.fringe_hot":[1,1,0.54],"light.color":[1.0,0.88,0.2],"trail":[1.0,0.88,0.2]}
// @speed 1.0

const vec3 M = vec3(1.000, 0.720, 0.140);
const vec3 D = vec3(0.350, 0.150, 0.020);
const vec3 H = vec3(1.000, 0.980, 0.820);
const vec3 A = vec3(0.500, 0.750, 1.000);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.68;
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
    float run = kf(x*1.5, y*0.1 - T*0.55) + 0.5*kf(x*2.8, y*0.2 - T*0.8);
    col = mix(D, M, smoothstep(0.2, 0.85, run));
    col += H*smoothstep(0.95, 1.3, run)*0.9;
  }
  col *= (0.45 + 0.7*nz)*1.5;
  col += M*pow(1.0 - nz, 3.0)*0.5;
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
float kMoment(){ return rxCycle(18.0, 10.3, 12.5, 14.5, 17.5); }
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  if (part(1.0) > 0.5) for (int i = 0; i < 16; i++){ float k = float(i); float rr = 1.2 + 3.0*h11(k*3.1); float a = h11(k*5.3); float sp = (3.0 + 2.4*h11(k*2.2))*(1.0 + 1.5*mom); float yy = mod(C*sp + k*7.3, L + 8.0) - 4.0; float ln = (2.0 + 2.5*h11(k*7.1))*(1.0 + 1.5*mom); vec3 p0 = around(rr, a, yy), p1 = around(rr, a, yy + ln); float s2; float d = raySeg(ro, rd, p0, p1, s2); vec3 hue = mix(M, A, step(0.5, h11(k*9.9))); fx += (H*halo(d, max(0.03, px))*1.2 + hue*halo(d, 0.16)*0.65)*s2*smoothstep(0.0, 2.0, yy)*(1.0 - smoothstep(L - 2.0, L + 2.0, yy + ln)); }
  if (part(2.0) > 0.5){ fx += mom*0.25*fxAura(ro, rd, 0.0, L, 2.4, M); fx += 1.0*fxCoils(ro, rd, px, 1.0, L - 1.0, 1.15, 0.35, 0.22, 2.0, M); }
  if (part(3.0) > 0.5) for (int i = 0; i < 3; i++){ float k = float(i); fx += 1.1*rxComet(ro, rd, px, 1.6 + 0.8*k, L*0.05, L*0.9, 5.0 + 1.3*k, k + 3.0, H, A); }
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
const vec3 SX_HOT = vec3(1.000, 0.968, 0.220);
const vec3 SX_DEEP = vec3(0.550, 0.484, 0.110);
const vec3 SX_PALE = vec3(1.000, 0.940, 0.600);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 8.0, 10.0, 2.60, 0.300, 0.90, 0.08, 0.96, 1.0, 7806.0, vec3(1.0), SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.29, 4.31, 0.010, 666.0, vec3(1.000, 0.968, 0.220)*0.24, vec3(0.550, 0.484, 0.110)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.968, 0.220)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
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
  a += 1.1*gxMeteors(ro, rd, px, 4.0, 4.0, 91.0, vec3(1.000, 0.940, 0.600), vec3(1.000, 0.968, 0.220));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

