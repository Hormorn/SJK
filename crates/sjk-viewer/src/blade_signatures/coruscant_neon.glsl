// Coruscant Skylanes (reworked 2/11/2026, update 24; Creyon: improve what has to be improved): Coruscant at night in a blade: a dark tower lit from within by rushing lanes of magenta and cyan traffic, airspeeders streaking round it, the glow of the sign-lit level on its edges.
// @part Night city
// @part Traffic lanes
// @part Air traffic
// @part Neon sign light
// @raycache fast
// @trail neon
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.55,0.11,0.413],"corona.rim_hot":[1,0.22,0.825],"corona.inner":[1,0.6,0.875],"core.white":[1.0,0.95,1.0],"core.fringe_cool":[0.8,0.16,0.6],"core.fringe_hot":[1,0.54,1],"light.color":[1.0,0.2,0.75],"trail":[1.0,0.2,0.75]}
// @speed 1.0

const vec3 M = vec3(0.080, 0.060, 0.140);
const vec3 D = vec3(0.020, 0.020, 0.050);
const vec3 H = vec3(1.000, 0.920, 1.000);
const vec3 A = vec3(0.200, 0.900, 1.000);

float kf(float a, float c){ return rxFlow(vec2(a, c)); }
float kl(float v, float w){ return 1.0 - smoothstep(0.0, w, abs(v)); }
float k_half(float y, float sd){
  float L = uLength;
  float T = uTime, C = uClock;
  float s = clamp(y/max(L, 1.0), 0.0, 1.0);
  float w = 0.6;
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
  vec3 col = vec3(1.0, 0.2, 0.75)*(halo(d, 0.16)*1.0 + halo(d, 0.8)*0.3 + halo(d, 2.4)*0.035);
  col += lvl(g)*vec3(1.0, 0.2, 0.75)*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.28;
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
    col = mix(D, M, 0.6 + 0.4*kf(x*4.0, y*0.3));
    if (part(1.0) > 0.5){ float lane = floor(ang*4.0 + 8.0);
    float go = mod(lane, 2.0) < 0.5 ? 1.0 : -1.0;
    float run = kf(lane*4.1, y*0.18 - go*T*(0.9 + 0.5*h11(lane)));
    float inlane = 1.0 - smoothstep(0.26, 0.5, abs(fract(ang*4.0 + 8.0) - 0.5));
    vec3 hue = go > 0.0 ? vec3(1.0, 0.16, 0.72) : A;
    col += hue*smoothstep(0.52, 0.78, run)*inlane*1.25; }
  }
  col *= (0.7 + 0.4*nz)*1.8;
  col += vec3(1.0, 0.2, 0.75)*pow(1.0 - nz, 3.0)*1.2;
  col = mix(col, H, (1.0 - smoothstep(0.0, 0.55, abs(u)))*0.0);
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
  if (part(2.0) > 0.5) for (int i = 0; i < 4; i++){ float k = float(i); vec3 c = mod(k, 2.0) < 0.5 ? vec3(1.0, 0.2, 0.75) : A; fx += 0.85*rxComet(ro, rd, px, 1.7 + 0.5*k, L*(0.08 + 0.2*k), L*0.2, 4.5 + 1.3*k, k + 70.0, H, c); }
  if (part(3.0) > 0.5) fx += vec3(1.0, 0.2, 0.75)*0.04*fxAura(ro, rd, 0.0, L, 1.8, vec3(1.0));
  if (part(1.0) > 0.5) fx += 1.0*shStreaks(ro, rd, 1.35, 0.5, L, 8.0, 2.8, 7.0, 0.0, 41.0, vec3(1.0, 0.2, 0.75), A*0.6);
  if (part(3.0) > 0.5){ fx += 2.2*vec3(0.0); fx += 2.2*vec3(0.0); fx += 0.28*syMist(ro, rd, L*0.1, L*0.95, 3.4, 0.08, 7.0, vec3(1.0, 0.2, 0.75)*0.5, A*0.25); }
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
const vec3 SX_HOT = vec3(1.000, 0.220, 0.825);
const vec3 SX_DEEP = vec3(0.550, 0.110, 0.413);
const vec3 SX_PALE = vec3(1.000, 0.600, 0.875);
vec3 sig_vol_e1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += gxKin(ro, rd, 0.0, 12.0, 3.40, 0.140, 0.55, 0.08, 0.96, 1.0, 7221.0, SX_PALE, SX_HOT);a += vec3(0.0);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
// >>> e1: the great effect round this blade (4/11/2026)
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the skylanes: levels of traffic winding up round the tower, thin lanes of light with speeders running along them
  if (part(2.0) > 0.5){
    for (int i = 0; i < 3; i++){
      float k = float(i), rr = 2.0 + 0.8*k, tn = (mod(k, 2.0) < 0.5 ? 1.0 : -1.0)*(1.3 - 0.2*k), ph = k*0.31 + 0.004*C;
      vec3 lane = k < 0.5 ? A : (k < 1.5 ? SX_HOT : vec3(1.0, 0.7, 0.3));
      a += 0.8*syRibbon(ro, rd, px, rr, rr + 0.1, tn, ph, L*0.03, L*0.97, 0.1, 391.0 + k, lane*0.8, lane*0.5);
      a += 1.7*xBeads(ro, rd, rr, rr + 0.1, tn, ph, L*0.03, L*0.97, 0.26, 394.0 + k, H, lane);
      a += 1.4*xBeads(ro, rd, rr, rr + 0.1, tn, ph, L*0.03, L*0.97, 0.2, 397.0 + k, vec3(1.0, 0.3, 0.3), SX_HOT);
    }
  }
  // smog lit by the signs, and the windows of far towers
  if (part(3.0) > 0.5){
    a += 1.1*syMistSpin(ro, rd, 0.3, L*0.96, 1.5, 5.2, 0.012, 400.0, SX_HOT*0.2 + A*0.08, A*0.06);
    a += 1.1*gxKin(ro, rd, 0.0, 10.0, 2.60, 0.110, 0.45, 0.08, 0.96, -1.0, 7234.0, H, A);
  }

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
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
  if (part(3.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.44*gxKin(ro, rd, 8.0, 6.0, 2.40, 0.160, 0.50, 0.08, 0.96, 1.0, 9085.0, vec3(1.000, 0.600, 0.875), vec3(1.000, 0.220, 0.825));
  a += 1.15*gxComets(ro, rd, px, 4.0, 4.6, 0.05, 2.4, 55.0, vec3(1.000, 0.600, 0.875), vec3(1.000, 0.220, 0.825));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

