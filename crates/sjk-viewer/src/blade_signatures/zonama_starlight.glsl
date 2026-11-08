// Zonama Starlight (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): Zonama Sekot, the living world that fled through the stars: a dark blade holding its night sky, huge living filaments of warm gold growing up through it, and golden stars wandering slowly round it on long curved paths.
// @part Night of Zonama
// @part Boras
// @part Seed-partner glow
// @part Wandering stars
// @part Sekot's song
// @part Rising sparks
// @raycache fast
// @room 10
// @solid 1
// @trail starlight
// @palette {"corona.rim_cool":[0.5,0.55,0.8],"corona.rim_hot":[1.0,0.92,0.7],"corona.inner":[1.0,0.98,0.92],"core.white":[1.0,1.0,0.97],"core.fringe_cool":[0.7,0.75,0.95],"core.fringe_hot":[1.0,0.95,0.8],"light.color":[1.0,0.93,0.75],"trail":[1.0,0.93,0.75]}
// @speed 1.0

const vec3 M = vec3(0.080, 0.140, 0.200);
const vec3 D = vec3(0.015, 0.025, 0.060);
const vec3 H = vec3(1.000, 0.980, 0.880);
const vec3 A = vec3(1.000, 0.860, 0.500);

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
  vec3 col = vec3(1.0, 0.88, 0.62)*(halo(d, 0.16)*0.7 + halo(d, 0.8)*0.3 + halo(d, 2.4)*0.06);
  col += lvl(g)*vec3(1.0, 0.88, 0.62)*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.28;
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
    vec2 c = vec2(ang*2.8, y*0.6 - T*0.05);
    vec2 cell = floor(c), f = fract(c) - 0.5;
    float star = halo(length(f - (h22(cell) - 0.5)*0.7), 0.1)*step(0.5, h21(cell));
    col = mix(D, M, kf(x*1.2, y*0.14)) + H*star*(0.6 + 0.4*sin(C*(0.6 + h21(cell)) + 9.0*h21(cell + 2.0)))*1.2;
    if (part(1.0) > 0.5){ float v = kf(x*1.5, y*0.14 - T*0.07); col += A*kl(v - 0.5, 0.05)*(0.8 + 0.5*sin(y*0.16 - T*0.6)); col += H*kl(v - 0.5, 0.014)*0.8; }
    if (part(2.0) > 0.5) col += A*halo(abs(u), 0.4)*(0.12 + 0.14*sin(C*0.6));
  }
  col *= (0.75 + 0.4*nz)*1.5;
  col += vec3(1.0, 0.88, 0.62)*pow(1.0 - nz, 3.0)*0.8;
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
  if (part(5.0) > 0.5) fx += 1.0*rxBorn(ro, rd, px, 14.0, 0.5, L*0.9, 0.3, 1.6, 1.6, 0.04, 17.0, H, A);
  if (part(4.0) > 0.5) fx += 1.3*rxStarCloud(ro, rd, px, 34.0, 3.6, 0.8, L*0.96, 0.05, 9.0, H, A);
  if (part(3.0) > 0.5) for (int i = 0; i < 5; i++){ float k = float(i); vec3 p = around(2.6, 0.02*C + 0.5*mod(k, 2.0), L*(0.12 + 0.19*k)); fx += fxFlare(ro, rd, p, px, 0.17, 1.8, 4.0, 0.08, A); }
  if (part(1.0) > 0.5) for (int i = 0; i < 3; i++){ float k = float(i); fx += 1.1*rxComet(ro, rd, px, 1.5 + 0.8*k, L*(0.06 + 0.26*k), L*0.3, 7.0 + 2.5*k, k + 12.0, H, A); }
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
const vec3 SX_HOT = vec3(1.000, 0.920, 0.700);
const vec3 SX_DEEP = vec3(0.500, 0.550, 0.800);
const vec3 SX_PALE = vec3(1.000, 0.980, 0.920);
vec3 sig_vol_e1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 4.0, 6.0, 3.60, 0.030, 0.24, 0.08, 0.96, 1.0, 8287.0, SX_PALE, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
// >>> e1: the great effect round this blade (4/11/2026)
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the living world's breath: a wide soft cloud of gold and night spreading round it, stars adrift inside
  a += 1.35*syMistSpin(ro, rd, 0.3, L*0.97, 2.2, 6.0, 0.01, 441.0, A*0.3, D*7.0);
  a += 1.1*syMistSpin(ro, rd, 0.3, L*0.97, 1.5, 4.4, -0.008, 442.0, SX_DEEP*0.32, D*5.0);
  a += 1.5*xDust(ro, rd, 2.0, 4.4, 0.9, C*0.02, L*0.03, L*0.97, 1.6, 443.0, H, A);

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_e1(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  float cl = rxFlow(vec2(b.y*0.1 - C*0.04, b.x*0.14 + 3.0));
  col += mix(A, SX_DEEP, 0.3)*cl*cl*halo(d, 4.8)*0.42*on;

  return col;
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  if (part(5.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.35*gxKin(ro, rd, 3.0, 7.0, 3.40, 0.030, 0.20, 0.08, 0.96, 1.0, 9510.0, vec3(1.000, 0.980, 0.920), vec3(1.000, 0.920, 0.700));
  a += 1.2*gxPlanets(ro, rd, px, 2.0, 4.6, 42.0, vec3(1.000, 0.980, 0.920), vec3(1.000, 0.920, 0.700));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

