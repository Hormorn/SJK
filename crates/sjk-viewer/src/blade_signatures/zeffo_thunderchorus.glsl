// Zeffo Thunderchorus (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): a thunderstorm held in a blade: blazing blue-white plasma with charged cloud racing up inside it, a great thunderhead hanging round it that flashes from within, huge forked bolts leaping off into the air, and wherever one strikes a star of light hangs and fades. Every 20 seconds the great storm: the sky lights, and the bolts come together.
// @part Charged plasma
// @part Thunderhead
// @part Sheet lightning
// @part Great bolts
// @part Storm glints
// @part The great storm
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail storm
// @palette {"corona.rim_cool":[0.05,0.3,0.95],"corona.rim_hot":[0.35,0.8,1.0],"corona.inner":[0.8,0.95,1.0],"core.white":[0.95,1.0,1.0],"core.fringe_cool":[0.3,0.5,1.0],"core.fringe_hot":[0.65,0.9,1.0],"light.color":[0.4,0.7,1.0],"trail":[0.4,0.8,1.0]}
// @speed 1.0

const vec3 M = vec3(0.140, 0.420, 1.000);
const vec3 D = vec3(0.020, 0.060, 0.240);
const vec3 H = vec3(0.960, 1.000, 1.000);
const vec3 A = vec3(0.500, 0.820, 1.000);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.78;
  float top = L*rxLife(0.0, 0.45, 0.32);
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/max(w*1.0, 0.05), 0.0, 1.0);
  return w*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_d(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float yc = clamp(b.y, 0.0, L*rxLife(0.0, 0.45, 0.32));
  float d = max(length(vec2(b.x, b.y - yc)) - k_half(yc, b.x < 0.0 ? -1.0 : 1.0), 0.0);
  float T = uTime, C = uClock, y = b.y, s = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  vec3 col = M*(halo(d, 0.16)*0.75 + halo(d, 0.8)*0.42 + halo(d, 2.4)*0.09);
  col += lvl(g)*M*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.28;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.6*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core_d(vec3 c, vec2 b, float px){ return c*0.0; }
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
    float v = kf(x*1.1, y*0.1 - T*0.42) + 0.5*kf(x*2.4, y*0.2 - T*0.65);
    col = mix(D, M, smoothstep(0.2, 0.85, v));
    col += A*smoothstep(0.85, 1.2, v)*0.9;
    if (part(2.0) > 0.5){ float cyc = C*0.42; float q = fract(cyc); float yy = L*h11(floor(cyc)*3.1); col += H*halo(abs(y - yy), 5.0)*pow(max(0.0, 1.0 - q*3.5), 2.0)*1.0; }
    if (part(0.0) > 0.5){
    vec2 zq = vec2(ang*1.25 + 0.25*sin(y*0.17 + C*0.5), y*0.13 - C*0.95);
    float zw = rxFlow(zq*0.8 + 4.0);
    float zf = rxFlow(zq + vec2(zw, 0.6*zw));
    col = mix(D*2.2, col, 0.45 + 0.55*smoothstep(0.25, 0.7, zf));
    float zr = abs(zf - 0.52);
    col += H*(1.0 - smoothstep(0.0, 0.05, zr))*0.75 + A*(1.0 - smoothstep(0.0, 0.16, zr))*0.5;
    float roll = sin(y*0.42 - C*3.3 + 2.0*zw);
    col += mix(A, H, 0.5)*smoothstep(0.55, 1.0, roll)*0.55;
    }
  }
  col *= (0.45 + 0.7*nz)*1.7;
  col += M*pow(1.0 - nz, 3.0)*0.5;
  col = mix(col, H, (1.0 - smoothstep(0.0, 0.5, abs(u)))*0.72);
  col += H*halo(max(b.y, 0.0), 0.5)*0.5;
  col = mix(col, H, 0.45*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec4 sig_solid(vec4 o, vec2 b, float px){
  vec4 r = k_solid(o, b, px);
  float j = smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, r.rgb, j), mix(o.a, r.a, j));
}
float kMoment(){ return rxCycle(20.0, 8.0, 10.2, 12.2, 15.2); }
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  float cyc = C*0.42; float fq = fract(cyc); float flash = pow(max(0.0, 1.0 - fq*3.5), 2.0)*step(0.5, part(2.0));
  if (part(1.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 9.0){
    fx += (0.6 + 0.9*flash + 0.7*mom)*syMistSpin(ro, rd, L*0.1, L*0.98, 1.8, 5.2, 0.012, 3.0, A*0.5, D*1.5);
    fx += 0.9*rxStarCloud(ro, rd, px, 26.0, 3.2, 0.8, L*0.95, 0.04, 9.0, H, A); }
  if (part(3.0) > 0.5){ fx += (1.5 + 1.4*mom)*liveArcs(ro, rd, px, L*0.06, L*0.97, 5.0 + 3.0*mom, 2.8, 0.45, 23.0, A);
    fx += 1.0*fxCoils(ro, rd, px, 0.8, L - 0.8, 1.45, 0.35, 0.1, 2.0, A); }
  if (part(4.0) > 0.5) for (int i = 0; i < 5; i++){ float k = float(i); float c2 = C*(0.16 + 0.04*k) + k*0.37; float n = floor(c2), q = fract(c2); vec3 p = around(2.0 + 2.2*h11(n*5.1 + k), h11(n*3.3 + k), L*(0.1 + 0.8*h11(n*1.7 + k))); fx += fxFlare(ro, rd, p, px, 0.2, 2.0, 4.0, 0.0, A)*pow(1.0 - q, 2.0)*smoothstep(0.0, 0.04, q); }
  if (part(5.0) > 0.5) fx += (0.1 + 0.5*flash)*mom*fxAura(ro, rd, 0.0, L, 3.0, A);
  if (part(0.0) > 0.5){ float zb = 0.6 + 0.4*sin(C*1.7); for (int i = 0; i < 2; i++){ float k = float(i); fx += 1.3*liveArcs(ro, rd, px, L*0.04, L*0.97, 3.0, 1.5, 0.75, 57.0 + k*9.0, mix(A, H, 0.5))*zb; }
    fx += 0.8*syMistSpin(ro, rd, L*0.06, L*0.98, 1.0, 2.8, 0.05, 58.0, A*0.5, M*0.3); }
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
const vec3 SX_HOT = vec3(0.350, 0.800, 1.000);
const vec3 SX_DEEP = vec3(0.050, 0.300, 0.950);
const vec3 SX_PALE = vec3(0.800, 0.950, 1.000);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += vec3(0.0); a += 0.55*syMistSpin(ro, rd, L*0.55, L*0.98, 2.0, 6.0, 0.012, 22.0, SX_DEEP*0.8, SX_DEEP*0.3);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34

// >>> dim: less glow, so its effects and body can be seen
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){ return sig_glow_d(g, b, r, px)*0.78; }
vec3 sig_core(vec3 c, vec2 b, float px){ return sig_core_d(c, b, px)*1.0; }
// <<< dim

// >>> update 52: this blade's great effects in the air
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  a += vec3(0.0);
  a += 0.95*fxFall(ro, rd, px, 1.0, L - 0.8, 3.8, 20.0, 5.2, 1.5, 0.14, 452.0, vec3(0.55, 0.72, 0.95));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.97, 1.4, 5.4, 0.08, 453.0, vec3(0.1, 0.25, 0.55), vec3(0.015, 0.04, 0.12));
  a += 1.1*xDust(ro, rd, 1.6, 4.2, 1.8, C*0.12, L*0.05, L*0.97, 0.7, 454.0, vec3(0.85, 0.95, 1.0), vec3(0.3, 0.55, 1.0));
  uTurnFx += a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return base;
}
// <<< update 52
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(5.0) > 0.5){
    a += 1.3*syPetalsAir(ro, rd, 10.0, 3.31, 0.051, 1793.0, vec3(0.800, 0.950, 1.000), vec3(0.350, 0.800, 1.000));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  if (part(5.0) > 0.5){
    col += vec3(0.0);
  }
  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol_m26(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.26*gxKin(ro, rd, 8.0, 5.0, 3.00, 0.120, 0.60, 0.08, 0.96, 1.0, 9476.0, vec3(0.800, 0.950, 1.000), vec3(0.350, 0.800, 1.000));
  a += 1.1*gxFlares(ro, rd, px, 5.0, 5.2, 203.0, vec3(0.800, 0.950, 1.000), vec3(0.350, 0.800, 1.000));
  a += 1.25*gxThunderheads(ro, rd, px, 5.0, 4.8, 3.0, vec3(0.800, 0.950, 1.000), vec3(0.350, 0.800, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(47.0, 16.0, 18.0, 30.0, 33.0), L = uLength;
  a += 1.5*gxChorus(ro, rd, px, 6.0, 4.6, clamp((mod(uClock + uSeed*7.0, 47.0) - 16.0)/15.0, 0.0, 1.0), 1103.0, vec3(0.800, 0.950, 1.000), vec3(0.350, 0.800, 1.000));
  return base*1.0 + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

