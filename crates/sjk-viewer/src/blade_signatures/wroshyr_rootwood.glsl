// Wroshyr Rootwood (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): a living wroshyr tree of Kashyyyk: deep brown bark split by bright green sap, thick roots climbing round it and reaching out, broad leaves sprouting from them, spores drifting up through the green light.
// @part Wroshyr bark
// @part Green sap
// @part Climbing roots
// @part Leaves
// @part Spores
// @raycache fast
// @trail growth
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.193,0.44,0.138],"corona.rim_hot":[0.385,0.88,0.275],"corona.inner":[0.675,0.9,0.625],"core.white":[0.9,1.0,0.75],"core.fringe_cool":[0.28,0.64,0.2],"core.fringe_hot":[0.72,1,0.6],"light.color":[0.35,0.8,0.25],"trail":[0.35,0.8,0.25]}
// @speed 1.0

const vec3 M = vec3(0.420, 0.280, 0.130);
const vec3 D = vec3(0.100, 0.060, 0.030);
const vec3 H = vec3(0.900, 1.000, 0.600);
const vec3 A = vec3(0.420, 0.920, 0.280);

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
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float yc = clamp(b.y, 0.0, L*rxLife(0.0, 0.45, 0.32));
  float d = max(length(vec2(b.x, b.y - yc)) - k_half(yc, b.x < 0.0 ? -1.0 : 1.0), 0.0);
  float T = uTime, C = uClock, y = b.y, s = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  vec3 col = A*(halo(d, 0.16)*0.4 + halo(d, 0.8)*0.26 + halo(d, 2.4)*0.05);
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
    float gr = sin(ang*7.0 + 3.0*kf(x*0.8, y*0.06));
    col = mix(D, M, 0.5 + 0.5*gr);
    col *= 0.8 + 0.35*kf(x*5.0, y*0.2);
    if (part(1.0) > 0.5){ float v = kf(x*1.6, y*0.12 - T*0.09); col += A*kl(v - 0.5, 0.05)*(0.9 + 0.5*sin(y*0.22 - T*0.8)); col += H*kl(v - 0.5, 0.014)*0.9; }
  }
  col *= (0.45 + 0.7*nz)*1.4;
  col += A*pow(1.0 - nz, 3.0)*0.8;
  col = mix(col, H, (1.0 - smoothstep(0.0, 0.55, abs(u)))*0.2);
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
vec3 kVine(vec3 ro, vec3 rd, float px, float r0, float r1, float y0, float y1, float turns, float a0, float wob, float w, vec3 hot, vec3 col, float k){
  if (fxNear(ro, rd, min(y0, y1), max(y0, y1)) > max(r0, r1) + wob + 1.2) return vec3(0.0);
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(r0, a0, y0);
  for (int i = 1; i <= 15; i++){
    float f = float(i)/15.0;
    vec3 q = around(mix(r0, r1, f) + wob*sin(f*9.0 + uClock*0.7 + k), a0 + turns*f, mix(y0, y1, f));
    float d = raySeg(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(i) - 1.0 + s)/15.0 - bf)*wN/bWs; }
    prev = q;
  }
  float pulse = pow(0.5 + 0.5*sin(bf*11.0 - uClock*1.3 + k), 4.0);
  float ww = mix(w, w*0.3, bf);
  return (hot*halo(best, max(ww, px))*(0.6 + 0.9*pulse) + col*halo(best, ww*4.5)*(0.35 + 0.5*pulse))*smoothstep(0.0, 0.06, bf)*(1.0 - smoothstep(0.82, 1.0, bf));
}
vec3 kLeaf(vec3 ro, vec3 rd, float px, float y, float a, float len, float curl, float w, vec3 hot, vec3 col){
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(0.5, a, y);
  for (int i = 1; i <= 6; i++){
    float f = float(i)/6.0;
    vec3 q = around(0.5 + len*f*0.75, a + 0.04*sin(uClock*0.5 + a*20.0)*f, y + len*f*(0.85 - curl*f));
    float d = raySeg(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(i) - 1.0 + s)/6.0 - bf)*wN/bWs; }
    prev = q;
  }
  float ww = w*(1.0 - bf*bf)*smoothstep(0.0, 0.12, bf) + 0.01;
  return hot*(1.0 - smoothstep(ww*0.6, ww + px, best))*0.85 + col*halo(best, ww*3.0)*0.42;
}
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, T = uTime, C = uClock, t;
  float aft = rxLife(0.15, 0.7, 0.35);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float mom = kMoment();
  if (part(2.0) > 0.5) for (int i = 0; i < 5; i++){ float k = float(i); fx += kVine(ro, rd, px, 0.85, 1.4 + 0.9*h11(k*3.0), 0.3 + k*2.5, L*(0.45 + 0.12*k), 0.8, k*0.2 + 0.008*C, 0.25, 0.1, M*1.4 + A*0.25, A*0.5, k); }
  if (part(3.0) > 0.5) for (int i = 0; i < 9; i++){ float k = float(i); fx += kLeaf(ro, rd, px, L*(0.08 + 0.85*fract(k*0.618)), k*0.382 + 0.008*C, 2.2 + h11(k*2.0), 0.3, 0.17, A*1.1, A*0.6); }
  if (part(4.0) > 0.5){ fx += 1.2*gxKin(ro, rd, 6.0, 8.0, 2.20, 0.030, 0.80, 0.08, 0.96, 1.0, 8248.0, H, A); fx += (0.2 + 0.4*mom)*fxAura(ro, rd, 0.0, L, 1.6, A)*0.5; }
  if (part(4.0) > 0.5){ for (int i = 0; i < 9; i++){ float k = float(i); vec3 p = around(1.7 + 1.5*sin(C*0.17 + k*1.9), 0.018*C*(1.0 + 0.3*h11(k)) + k/9.0, L*(0.06 + 0.88*fract(k*0.618)) + 1.4*sin(C*0.23 + k*2.3)); float d = rayPt(ro, rd, p, t); float gl = pow(0.5 + 0.5*sin(C*(0.7 + 0.5*h11(k*2.0)) + k*2.1), 3.0); fx += (vec3(1.0, 0.95, 0.5)*halo(d, max(0.07, px))*1.8 + A*halo(d, 0.42)*0.7)*gl; }
    fx += 0.9*rxRays(ro, rd, px, 7.0, L*0.55, L*0.96, 3.6, 5.0, vec3(0.95, 1.0, 0.65)*0.75, A*0.5); }
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
const vec3 SX_HOT = vec3(0.385, 0.880, 0.275);
const vec3 SX_DEEP = vec3(0.193, 0.440, 0.138);
const vec3 SX_PALE = vec3(0.675, 0.900, 0.625);
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += vec3(0.0);
  // the root lash: every so often great thorned roots tear out of the blade and lash the air, sap flying from their tips
  float lash = part(2.0) > 0.5 ? rxCycle(13.0, 5.0, 7.2, 9.3, 12.3) : 0.0;
  if (lash > 0.01){
    float run13 = C/13.0;
    float cy13 = floor(run13), q13 = fract(run13)*13.0 - 5.0;
    for (int i = 0; i < 5; i++){
      float k = float(i), h0 = h11(k*2.3 + cy13);
      float y0 = L*(0.1 + 0.16*k + 0.06*h0), ang = h11(k*5.7 + cy13) + 0.04*sin(C*0.9 + k);
      float out0 = (3.2 + 2.0*h11(k*3.1 + cy13))*(0.35 + 0.65*lash);
      a += 1.6*lash*syLick(ro, rd, px, y0, ang, out0, (2.4 + 2.4*h11(k*4.3 + cy13))*(0.35 + 0.65*lash), 0.62, k*3.0 + cy13, A + H*0.3, M*1.5 + A*0.25, D*2.0);
      a += 1.3*rxSparkShower(ro, rd, px, around(0.35 + out0, ang, y0 + 3.0*(0.35 + 0.65*lash)), max(q13, 0.0), 16.0, 3.2, cy13 + k*5.0, H, A);
    }
  }
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

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
  a += 1.45*gxKin(ro, rd, 1.0, 6.0, 3.00, 0.050, 0.24, 0.08, 0.96, -1.0, 9442.0, vec3(0.675, 0.900, 0.625), vec3(0.385, 0.880, 0.275));
  a += 1.15*gxTumble(ro, rd, px, 10.0, 4.8, 2.0, 1.0, 0.34, 51.0, vec3(0.675, 0.900, 0.625), vec3(0.385, 0.880, 0.275));
  a += 1.0*gxSwarm(ro, rd, px, 8.0, 3.8, 52.0, vec3(0.675, 0.900, 0.625), vec3(0.385, 0.880, 0.275));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

