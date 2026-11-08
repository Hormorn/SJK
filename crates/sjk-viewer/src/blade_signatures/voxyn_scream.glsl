// Voxyn Scream (rebuilt 3/11/2026): The voxyn's scream made visible: waves of sound chase up a violet body while soft waves of sound ripple outward through the air. Every eleven seconds it opens up and howls: the blade shudders and the sound reaches twice as far.
// @part Sound on the body
// @part Sound in the air
// @part Howl
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail sonic
// @palette {"corona.rim_cool":[0.341,0.094,0.55],"corona.rim_hot":[0.62,0.17,1.0],"corona.inner":[0.97,0.9,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.496,0.136,0.8],"core.fringe_hot":[0.97,0.9,1.0],"light.color":[0.62,0.17,1.0],"trail":[0.62,0.17,1.0]}
// @speed 1.0

const vec3 Vi = vec3(0.620, 0.170, 1.000);
const vec3 D = vec3(0.040, 0.010, 0.095);
const vec3 Gr = vec3(0.280, 1.000, 0.450);
const vec3 H = vec3(0.970, 0.900, 1.000);
float vx_top(){ return uLength*rxLife(0.0, 0.42, 0.28); }
float vx_scream(){ return rxCycle(11.0, 4.3, 6.5, 7.5, 10.5); }
float vx_half(float y){
  float top = vx_top();
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/2.2, 0.0, 1.0);
  float st = vx_scream();
  return (0.54 + (0.05 + 0.1*st)*sin(y*2.2 - uClock*14.0))*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  float L = uLength, top = vx_top();
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - vx_half(yc), 0.0);
  float st = vx_scream();
  vec3 col = Vi*(halo(d, 0.26)*0.85 + halo(d, 1.1)*(0.3 + 0.3*st) + halo(d, 3.2)*(0.07 + 0.1*st));
  // sound leaving the blade: soft waves running outward from four throats
  if (part(1.0) > 0.5 && uFx > 0.0){
    float wob = 1.1*rxFlow(vec2(b.y*0.16, uClock*0.45)) + 0.35*rxFlow(vec2(b.y*0.5 + 4.0, uClock*0.8));
    float q = abs(b.x) + wob*smoothstep(0.3, 1.8, abs(b.x));
    float wave = sin(q*2.7 - uClock*4.2);
    float band = smoothstep(0.1, 1.0, wave);
    float reach = 4.6 + 3.4*st;
    float env = (1.0 - smoothstep(0.5, reach, abs(b.x)))*smoothstep(0.35, 1.0, abs(b.x));
    float along = smoothstep(0.5, 5.0, b.y)*(1.0 - smoothstep(top - 5.0, top - 0.5, b.y))*rxLife(0.25, 0.8, 0.25);
    col += mix(Vi, mix(Vi, Gr, 0.45), smoothstep(1.0, 4.0, abs(b.x)))*band*env*along*(0.55 + 0.7*st);
  }
  col += lvl(g)*Vi*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = vx_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  float w = sin(b.y*(0.8 + 0.3*sin(uClock*0.3)) + 2.4*u*u - uClock*4.0);
  vec3 col = mix(D*2.0, Vi, 0.5 + 0.5*w);
  col = mix(col, H, smoothstep(0.55, 1.0, w)*(1.0 - smoothstep(0.0, 0.75, abs(u)))*0.8);
  col *= 0.55 + 0.7*nz;
  col += Gr*pow(1.0 - nz, 3.0)*0.45;
  col = mix(col, H, 0.45*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.1, 0.5, 0.3);
  if (aft <= 0.01) return fx;
  float st = vx_scream();
  if (part(2.0) > 0.5) fx += (0.45 + 0.9*st)*shStreaks(ro, rd, 1.7, L*0.04, L*0.95, 8.0, 2.2, 6.0, 0.5, 81.0, H, Vi*0.6);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(2.0) > 0.5){
  a += 1.4*shFlakes(ro, rd, 3.47, 0.5, L - 0.5, 17.1, 2.8, 0.30, 0.028, 0.08, 0.55, 0.0, 565.0, vec3(0.620, 0.170, 1.000), vec3(0.341, 0.094, 0.550));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.34, 4.30, 0.015, 568.0, vec3(0.620, 0.170, 1.000)*0.24, vec3(0.341, 0.094, 0.550)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.620, 0.170, 1.000)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(2.0) > 0.5){
    a += 1.4*gxKin(ro, rd, 8.0, 6.0, 2.60, 0.200, 0.40, 0.08, 0.96, 1.0, 8222.0, vec3(0.970, 0.900, 1.000), vec3(0.620, 0.170, 1.000));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  if (part(2.0) > 0.5){
    col += vec3(0.0);
  }
  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  a += 1.1*gxVortex(ro, rd, px, 9.0, 4.2, 99.0, vec3(0.970, 0.900, 1.000), vec3(0.620, 0.170, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

