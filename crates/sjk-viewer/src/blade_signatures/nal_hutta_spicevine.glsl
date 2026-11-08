// Nal Hutta Spicevine (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): a Hutt-world vine wound on itself, two dark stems twining up a sour green glow.
// @part Twining vine
// @part Spice glow
// @raycache fast
// @trail plague
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.25,0.55,0.0],"corona.rim_hot":[0.6,1.0,0.15],"corona.inner":[0.9,1.0,0.7],"core.white":[0.97,1.0,0.9],"core.fringe_cool":[0.4,0.75,0.05],"core.fringe_hot":[0.75,1.0,0.4],"light.color":[0.55,1.0,0.15],"trail":[0.55,1.0,0.15]}
// @speed 1.0

const vec3 M = vec3(0.500, 0.860, 0.120);
const vec3 D = vec3(0.140, 0.280, 0.020);
const vec3 H = vec3(0.900, 1.000, 0.550);
const vec3 A = vec3(0.280, 0.500, 0.060);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.46 + 0.05*sin(y*0.3 + sd);
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
    float tw = sin(ang*1.0 + y*0.3 - T*0.1);
    float tw2 = sin(ang*1.0 - y*0.3 + 1.6 + T*0.08);
    col = mix(M, D, smoothstep(0.55, 0.9, tw)*0.85);
    col = mix(col, A, smoothstep(0.65, 0.95, tw2)*0.8);
    if (part(1.0) > 0.5) col += H*(1.0 - smoothstep(0.2, 0.5, abs(tw)))*0.25*(0.7 + 0.5*sin(y*0.2 - T*0.6));
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
vec3 kVine(vec3 ro, vec3 rd, float px, float r0, float r1, float y0, float y1, float turns, float a0, float wob, float w, vec3 hot, vec3 col, float k){
  if (fxNear(ro, rd, min(y0, y1), max(y0, y1)) > max(r0, r1) + wob + 1.2) return vec3(0.0);
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(r0, a0, y0);
  for (int i = 1; i <= 26; i++){
    float f = float(i)/26.0;
    vec3 q = around(mix(r0, r1, f) + wob*sin(f*9.0 + uClock*0.7 + k), a0 + turns*f, mix(y0, y1, f));
    float d = raySeg(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(i) - 1.0 + s)/26.0 - bf)*wN/bWs; }
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
  if (part(1.0) > 0.5){ fx += kVine(ro, rd, px, 0.85, 1.25, 0.3, L - 1.0, 1.3, 0.02*C, 0.18, 0.07, M, A, 3.0); for (int i = 0; i < 6; i++){ float k = float(i); fx += kLeaf(ro, rd, px, L*(0.08 + (0.9 - 0.08)*fract(k*0.618)), k*0.382 + 0.0, 2.0*(0.7 + 0.5*h11(k*3.1)), 0.2, 0.1, M, A); } }
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
const vec3 SX_HOT = vec3(0.600, 1.000, 0.150);
const vec3 SX_DEEP = vec3(0.250, 0.550, 0.000);
const vec3 SX_PALE = vec3(0.900, 1.000, 0.700);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 1.0*gxKin(ro, rd, 6.0, 7.0, 1.80, 0.040, 0.50, 0.08, 0.96, 1.0, 7793.0, SX_PALE, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(1.0) > 0.5){
  a += 1.5*shFlakes(ro, rd, 3.56, 0.5, L - 0.5, 22.5, 2.6, -0.44, 0.038, 0.07, 0.6, 0.0, 799.0, vec3(0.900, 1.000, 0.700), vec3(0.600, 1.000, 0.150));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.600, 1.000, 0.150)*wv*(halo(d, 0.9)*0.21 + halo(d, 2.6)*0.13)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(1.0) > 0.5){
    a += 1.3*vec3(0.0);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
