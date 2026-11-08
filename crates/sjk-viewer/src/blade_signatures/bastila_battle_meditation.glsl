// Bastila Battle Meditation (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): Bastila Shan's battle meditation: a deep blue stillness that breathes, one slow swell passing through it, while points of light stand all round it in perfect ranks and move as one: out on the breath, in on the breath. Nothing here is out of step.
// @part Still deep
// @part Shared breath
// @part Formation
// @part Unbroken will
// @part Calm spreading
// @raycache fast
// @trail equilibrium
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.2,0.4,0.85],"corona.rim_hot":[0.55,0.75,1.0],"corona.inner":[0.92,0.96,1.0],"core.white":[1.0,0.98,0.92],"core.fringe_cool":[0.35,0.55,0.95],"core.fringe_hot":[0.75,0.88,1.0],"light.color":[0.5,0.7,1.0],"trail":[0.5,0.7,1.0]}
// @speed 1.0

const vec3 M = vec3(0.200, 0.360, 0.800);
const vec3 D = vec3(0.030, 0.060, 0.220);
const vec3 H = vec3(1.000, 1.000, 1.000);
const vec3 A = vec3(0.550, 0.750, 1.000);

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
vec3 sig_glow_e1(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float yc = clamp(b.y, 0.0, L*rxLife(0.0, 0.45, 0.32));
  float d = max(length(vec2(b.x, b.y - yc)) - k_half(yc, b.x < 0.0 ? -1.0 : 1.0), 0.0);
  float T = uTime, C = uClock, y = b.y, s = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  vec3 col = M*(halo(d, 0.16)*0.6 + halo(d, 0.8)*0.3 + halo(d, 2.4)*0.06);
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
    float br = 0.5 + 0.5*sin(C*0.7);
    float flowv = kf(x*1.5, y*0.1 - T*0.1);
    col = mix(D, M, smoothstep(0.15, 0.8, flowv));
    if (part(1.0) > 0.5){ col += A*br*0.42*nz*smoothstep(0.3, 0.8, flowv); col += H*kl(flowv - 0.55, 0.03)*(0.3 + 0.7*br); }
  }
  col *= (0.45 + 0.7*nz)*1.5;
  col += M*pow(1.0 - nz, 3.0)*0.5;
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
  float br = 0.5 + 0.5*sin(C*0.7);
  if (part(2.0) > 0.5) fx += 1.2*xPuffs(ro, rd, 2.1 + 0.5*br, 2.1 + 0.5*br, 1.0, C*0.012, L*0.05, L*0.95, 0.8, 7.0, A*0.55, M*0.5);
  if (part(3.0) > 0.5) fx += vec3(0.0);
  if (part(4.0) > 0.5){ fx += (0.5 + 0.35*br + 0.5*mom)*rxAuraShell(ro, rd, -0.3, L + 0.3, 2.2 + 1.4*br + 1.2*mom, 3.0, M, A); }
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
const vec3 SX_HOT = vec3(0.550, 0.750, 1.000);
const vec3 SX_DEEP = vec3(0.200, 0.400, 0.850);
const vec3 SX_PALE = vec3(0.920, 0.960, 1.000);
vec3 sig_vol_e1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
// >>> e1: the great effect round this blade (4/11/2026)
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the meditation reaching out: long calm veils, and lights moving in slow formation between them
  for (int i = 0; i < 3; i++){
    float k = float(i);
    a += 1.6*xBeads(ro, rd, 3.0, 3.0, 0.0, k/3.0 + C*0.006, L*(0.1 + 0.32*k), L*(0.32 + 0.32*k), 0.22, 434.0 + k, H, A);
  }
  a += 1.2*gxKin(ro, rd, 8.0, 6.0, 2.60, 0.060, 0.50, 0.08, 0.96, 1.0, 7117.0, H, A);

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_e1(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  // the one slow breath of everyone she holds together, going through the air round it
  float br = pow(0.5 + 0.5*sin(b.y*0.2 - C*0.7), 2.0);
  col += A*br*(halo(d, 1.5)*0.22 + halo(d, 4.2)*0.2)*on;

  return col;
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  a += 1.3*vec3(0.0);
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  if (part(4.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.35*gxKin(ro, rd, 3.0, 7.0, 3.00, 0.050, 0.16, 0.08, 0.96, 1.0, 9051.0, vec3(0.920, 0.960, 1.000), vec3(0.550, 0.750, 1.000));
  a += 1.2*gxBeads(ro, rd, px, 3.0, 3.2, 43.0, vec3(0.920, 0.960, 1.000), vec3(0.550, 0.750, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

