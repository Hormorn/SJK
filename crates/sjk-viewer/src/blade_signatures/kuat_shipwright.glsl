// Kuat Shipwright (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): the great orbital drydocks of Kuat: a long smooth warship hull in gold-lit steel with lights running along its length, slim scaffold beams turning slowly round it, welders throwing sparks and small arcs from it. Every 24 seconds the drive test: the whole blade flares.
// @part Hull steel
// @part Running lights
// @part Scaffold beams
// @part Welding
// @part Drive glow
// @raycache fast
// @trail forge
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.55,0.44,0.138],"corona.rim_hot":[1,0.88,0.275],"corona.inner":[1,0.9,0.625],"core.white":[1.0,0.97,0.85],"core.fringe_cool":[0.8,0.64,0.2],"core.fringe_hot":[1,1,0.6],"light.color":[1.0,0.8,0.25],"trail":[1.0,0.8,0.25]}
// @speed 1.0

const vec3 M = vec3(0.820, 0.680, 0.320);
const vec3 D = vec3(0.140, 0.110, 0.080);
const vec3 H = vec3(1.000, 0.970, 0.820);
const vec3 A = vec3(1.000, 0.820, 0.300);

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
  vec3 col = M*(halo(d, 0.16)*0.4 + halo(d, 0.8)*0.3 + halo(d, 2.4)*0.06);
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
    col = mix(D, M, 0.55 + 0.45*kf(x*2.4, y*0.1));
    col *= 0.85 + 0.25*sin(ang*6.0);
    if (part(1.0) > 0.5){ col += H*(kl(abs(u) - 0.55, 0.09))*(0.25 + 1.0*shPulse(y, 3.0, 7.0, 1.6)); col += A*kl(u, 0.1)*shPulse(y, 2.0, 11.0, 2.5); }
  }
  col *= (0.45 + 0.7*nz)*1.45;
  col += M*pow(1.0 - nz, 3.0)*0.7;
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
float kMoment(){ return rxCycle(24.0, 13.0, 15.2, 17.2, 20.2); }
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
  if (part(2.0) > 0.5) for (int i = 0; i < 3; i++){ float k = float(i); float a = k/3.0 + 0.012*C; float yy = L*(0.2 + 0.27*k); fx += 0.9*kLine(ro, rd, px, around(2.6, a, yy - 4.0), around(2.6, a + 0.02, yy + 4.0), 0.0, 0.04, H, A, 2.2); fx += 0.8*kLine(ro, rd, px, around(0.9, a, yy), around(2.6, a, yy), 0.0, 0.025, H, A, 1.1); }
  if (part(3.0) > 0.5){ fx += 1.4*fxSparks(ro, rd, px, 1.0, L - 1.0, 12.0, 1.3, 1.3, 15.0, A); fx += vec3(0.0); }
  if (part(4.0) > 0.5) fx += (0.1 + 0.55*mom)*fxAura(ro, rd, 0.0, L, 1.8, A);
  if (part(2.0) > 0.5){ for (int i = 0; i < 4; i++){ float k = float(i); float a = k/4.0 + 0.014*C; float yy = L*(0.14 + 0.24*k); float sb; float db = raySeg(ro, rd, around(5.6, a + 0.04, yy + 2.5), around(0.9, a, yy), sb); fx += vec3(0.5, 0.75, 1.0)*(halo(db, 0.28)*0.16 + halo(db, 0.75)*0.07)*(0.6 + 0.4*sin(C*0.5 + k*2.0))*smoothstep(0.0, 0.2, sb); }
   for (int i = 0; i < 6; i++){ float k = float(i); vec3 p = around(2.6, floor(k/2.0)/3.0 + 0.012*C + mod(k, 2.0)*0.0, L*(0.2 + 0.27*floor(k/2.0)) + (mod(k, 2.0) < 0.5 ? -4.0 : 4.0)); float d = rayPt(ro, rd, p, t); float bl = smoothstep(0.75, 0.95, sin(C*1.6 + k*1.3)); vec3 cc = mod(k, 2.0) < 0.5 ? vec3(1.0, 0.18, 0.12) : vec3(0.2, 1.0, 0.4); fx += (cc*halo(d, 0.32)*0.7 + H*halo(d, max(0.05, px))*1.2)*(0.12 + 0.88*bl); } }
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
const vec3 SX_HOT = vec3(1.000, 0.880, 0.275);
const vec3 SX_DEEP = vec3(0.550, 0.440, 0.138);
const vec3 SX_PALE = vec3(1.000, 0.900, 0.625);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += vec3(0.0);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  float L = uLength, C = uClock;
  vec4 n = (sxShards(ro, rd, px, 5.0, 3.4, 0.85, 0.012, 223.0, vec3(0.14, 0.13, 0.12), SX_PALE))*rxLife(0.45, 1.3, 0.4)*uFx*hiltFade(ro, rd);
  return vec4(mix(o.rgb, n.rgb, n.a), max(o.a, n.a));
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(4.0) > 0.5){
  a += 1.6*gxKin(ro, rd, 7.0, 6.0, 3.20, 0.040, 0.40, 0.08, 0.96, 1.0, 7585.0, vec3(1.000, 0.900, 0.625), vec3(1.000, 0.880, 0.275));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.23, 4.71, 0.015, 766.0, vec3(1.000, 0.880, 0.275)*0.24, vec3(0.550, 0.440, 0.138)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.880, 0.275)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
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
  a += 1.1*gxTumble(ro, rd, px, 9.0, 4.0, 0.0, 0.0, 0.28, 89.0, vec3(1.000, 0.900, 0.625), vec3(1.000, 0.880, 0.275));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

