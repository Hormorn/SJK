// Rakghoul Plague (rebuilt 3/11/2026): The rakghoul plague of Taris: sick yellow flesh full of black veins and raw sores. Boils swell slowly out of its outline until they burst in a spray of pus, spores sink off it and a low breath of plague hangs about the lower blade.
// @part Infected flesh
// @part Bursting boils
// @part Spores
// @part Plague breath
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail plague
// @palette {"corona.rim_cool":[0.341,0.396,0.066],"corona.rim_hot":[0.62,0.72,0.12],"corona.inner":[1,0.98,0.76],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.496,0.576,0.096],"core.fringe_hot":[1,0.98,0.76],"light.color":[0.62,0.72,0.12],"trail":[0.62,0.72,0.12]}
// @speed 1.0

const vec3 M = vec3(0.620, 0.720, 0.120);
const vec3 Nc = vec3(0.060, 0.050, 0.020);
const vec3 Sr = vec3(0.920, 0.180, 0.120);
const vec3 Y = vec3(0.960, 1.000, 0.420);
const vec3 H = vec3(1.000, 0.980, 0.760);
float ra_top(){ return uLength*rxLife(0.0, 0.5, 0.32); }
// A boil: where it sits, how swollen (it swells for six seconds, then pops).
float ra_boil(float k, out float y, out float side, out float pop){
  float run = uClock/(6.0 + 1.3*k) + k*0.37;
  float n = floor(run), q = fract(run);
  y = uLength*(0.1 + 0.8*h11(k*3.1 + n*1.7));
  side = h11(k*7.3 + n) < 0.5 ? -1.0 : 1.0;
  pop = q;
  return smoothstep(0.0, 0.92, q)*(1.0 - smoothstep(0.92, 0.96, q));
}
float ra_half(float y, float sd){
  float top = ra_top();
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/1.4, 0.0, 1.0);
  float w = 0.5 + 0.1*vn(vec2(y*0.8, sd*2.0));
  for (int i = 0; i < 5; i++){
    float by, side, pop, sw = ra_boil(float(i), by, side, pop);
    if (side == sd) w += 1.05*sw*exp(-(y - by)*(y - by)/(0.6 + 0.8*sw));
  }
  return w*sqrt(max(1.0 - e*e, 0.0))*smoothstep(-0.25, 0.8, y);
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  float top = ra_top(), sd = b.x < 0.0 ? -1.0 : 1.0;
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - ra_half(yc, sd), 0.0);
  vec3 col = M*(halo(d, 0.32)*1.05 + halo(d, 1.4)*0.5 + halo(d, 3.8)*0.17);
  col += lvl(g)*M*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.7*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float sd = b.x < 0.0 ? -1.0 : 1.0;
  float half = ra_half(b.y, sd);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  float v = rxFlow(vec2(b.x*1.5, b.y*0.11 - uClock*0.05));
  vec3 col = mix(M*0.55, Y*0.9, smoothstep(0.3, 0.75, v));
  if (part(0.0) > 0.5){
    // dead black veins branching up the flesh
    float v1 = rxFlow(vec2(b.x*2.2 + 0.4, b.y*0.17));
    float v2 = rxFlow(vec2(b.x*4.2 + 6.0, b.y*0.33 + 3.0));
    float vein = max(1.0 - smoothstep(0.0, 0.055, abs(v1 - 0.5)), 0.75*(1.0 - smoothstep(0.0, 0.04, abs(v2 - 0.5))));
    col = mix(col, Nc, vein*0.92);
    float sore = vn(vec2(b.x*1.8, b.y*0.45 + uClock*0.03));
    col = mix(col, Sr, smoothstep(0.7, 0.82, sore));
    col += H*smoothstep(0.86, 0.94, sore)*0.8;
  }
  for (int i = 0; i < 5; i++){
    float by, side, pop, sw = ra_boil(float(i), by, side, pop);
    if (side != sd || sw <= 0.0) continue;
    float q = length(vec2(b.x - sd*half*0.55, (b.y - by)*0.7));
    float dome = 1.0 - smoothstep(0.0, 0.7 + 0.45*sw, q);
    col = mix(col, mix(Y, H, sw), dome*sw);
  }
  col *= 0.55 + 0.7*nz;
  col = mix(col, H, 0.45*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.25, 1.0, 0.35);
  if (aft <= 0.01) return fx;
  if (part(1.0) > 0.5) for (int i = 0; i < 5; i++){
    float by, side, pop, sw = ra_boil(float(i), by, side, pop);
    // the pop: a spray of pus and a sick puff
    float q = (pop - 0.94)*8.0;
    if (q < 0.0 || q > 1.0) continue;
    vec3 c = vec3(side*0.8, by, 0.0);
    fx += 1.7*rxSparkShower(ro, rd, px, c, q, 22.0, 3.2, float(i)*3.0, H, Y);
    float t, d = rayPt(ro, rd, c, t);
    fx += Y*halo(d, 0.5 + 1.8*q)*0.75*(1.0 - q)*(1.0 - q);
  }
  if (part(2.0) > 0.5) fx += 1.15*gxKin(ro, rd, 5.0, 5.0, 2.60, 0.050, 0.50, 0.08, 0.96, 1.0, 7923.0, Y, M);
  if (part(3.0) > 0.5) fx += 0.85*syMistSpin(ro, rd, 0.3, L*0.7, 1.2, 3.8, 0.01, 92.0, M*0.38, Nc*2.0);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.4*shFlakes(ro, rd, 2.82, 0.5, L - 0.5, 15.5, 2.8, 0.34, -0.040, 0.08, 0.55, 0.0, 704.0, vec3(0.620, 0.720, 0.120), vec3(0.341, 0.396, 0.066));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.25, 4.63, -0.008, 707.0, vec3(0.620, 0.720, 0.120)*0.24, vec3(0.341, 0.396, 0.066)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.620, 0.720, 0.120)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
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
  a += 1.1*gxLanterns(ro, rd, px, 4.0, 3.4, 100.0, vec3(1.000, 0.980, 0.760), vec3(0.620, 0.720, 0.120));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

