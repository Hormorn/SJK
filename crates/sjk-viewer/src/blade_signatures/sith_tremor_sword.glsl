// Sith Tremor Sword (rebuilt 3/11/2026): The Sith tremor sword, whose blade shakes to cut: a true sword of dark steel with a fuller and burning edges, its outline blurred by a hard fast tremble that gets worse as you swing. The air cracks round it, sparks shake loose and old rage rises round it.
// @part Trembling steel
// @part Cracked air
// @part Shed sparks
// @part Old rage
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail tremor
// @palette {"corona.rim_cool":[0.55,0.039,0.039],"corona.rim_hot":[1,0.07,0.07],"corona.inner":[1,0.86,0.78],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.8,0.056,0.056],"core.fringe_hot":[1,0.86,0.78],"light.color":[1,0.07,0.07],"trail":[1,0.07,0.07]}
// @speed 1.0

const vec3 Stl = vec3(0.160, 0.140, 0.150);
const vec3 Dk = vec3(0.022, 0.016, 0.020);
const vec3 Rd = vec3(1.000, 0.070, 0.070);
const vec3 Or = vec3(1.000, 0.360, 0.180);
const vec3 H = vec3(1.000, 0.860, 0.780);
float tr_top(){ return uLength*rxLife(0.0, 0.42, 0.28); }
float tr_rage(){ return rxCycle(12.0, 5.0, 7.2, 8.5, 11.5); }
// How far the blade is thrown sideways at y this instant.
float tr_shake(float y, float at){
  float amp = 0.06 + 0.12*tr_rage() + 0.14*clamp(uSwing, 0.0, 1.0);
  return amp*(sin(y*0.3 + at*62.0) + 0.45*sin(y*0.7 - at*97.0))*smoothstep(0.5, 8.0, y);
}
float tr_half(float y){
  float top = tr_top(), L = uLength;
  if (y > top) return 0.0;
  // a sword: broad from the guard, narrowing to a long true point
  float w = mix(0.86, 0.42, smoothstep(0.0, L, y));
  float p = 1.0 - clamp((top - y)/5.0, 0.0, 1.0);
  return w*sqrt(max(1.0 - p*p, 0.0));
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  float top = tr_top();
  float yc = clamp(b.y, 0.0, top);
  float ra = tr_rage();
  vec3 col = vec3(0.0);
  // the tremble: the same outline at four moments, its glow blurred across them
  for (int i = 0; i < 4; i++){
    float x = b.x - tr_shake(yc, uClock - float(i)*0.012);
    float d = max(length(vec2(x, b.y - yc)) - tr_half(yc), 0.0);
    col += Rd*(halo(d, 0.22)*0.3 + halo(d, 1.2)*(0.14 + 0.06*ra));
  }
  float d0 = max(length(vec2(b.x, b.y - yc)) - tr_half(yc), 0.0);
  col += Rd*halo(d0, 3.4)*(0.1 + 0.16*ra);
  col += lvl(g)*Rd*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float x = b.x - tr_shake(b.y, uClock);
  float half = tr_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.04 + px, abs(x)))*smoothstep(0.02, 0.1 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), -1.0, 1.0);
  // two flats meeting a dark fuller down the middle, and edges burning red
  float flat_ = 0.4 + 0.75*abs(u);
  float wear = rxFlow(vec2(x*5.0, b.y*0.5));
  vec3 col = mix(Dk*2.0, Stl*2.3, flat_)*(0.8 + 0.5*wear);
  if (part(0.0) > 0.5){
    float scr = rxFlow(vec2(x*3.0 + b.y*0.2, b.y*0.14));
    col += Rd*(1.0 - smoothstep(0.0, 0.05, abs(scr - 0.5)))*0.65*(0.4 + 0.6*abs(u));
  }
  float edge = smoothstep(0.66, 0.98, abs(u));
  float wave = 0.5 + 0.5*sin(b.y*0.42 - uClock*7.0);
  col = mix(col, mix(Rd, mix(Or, H, wave), 0.3 + 0.7*wave*tr_rage() + 0.2*wave), edge);
  col *= 1.0 - 0.7*(1.0 - smoothstep(0.0, 0.1, abs(u)));
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
  float ra = tr_rage();
  // the air cracks where the blade shakes it: short jagged tears at Storm's beat
  if (part(1.0) > 0.5) for (int i = 0; i < 4; i++){
    float k = float(i);
    float cy = uClock*6.5 + k*0.618, m = floor(cy), q = fract(cy);
    if (h11(m*1.7 + k*3.3) < mix(0.38, 0.1, ra)) continue;
    float a = h11(m*4.1 + k), y = L*(0.08 + 0.84*h11(m*2.3 + k*5.0));
    float R = 1.9 + 1.9*h11(m*6.1 + k) + 1.2*ra;
    vec3 p0 = around(0.5, a, y), p1 = around(0.5 + R, a + 0.03, y + 0.6*(h11(m + k) - 0.5));
    float d = liveBolt(ro, rd, p0, mix(p0, p1, 0.5), p1, 0.55, m + k*2.1, px);
    fx += (mix(Or, H, 0.6)*halo(d, max(0.035, px))*1.5 + Rd*(halo(d, 0.16)*0.9 + halo(d, 0.6)*0.22))*pow(1.0 - q, 1.5);
  }
  if (part(2.0) > 0.5) fx += (0.9 + 1.1*ra)*fxSparks(ro, rd, px, L*0.05, L*0.92, 9.0, 1.2, 1.3, 141.0, Or);
  if (part(3.0) > 0.5) fx += ra*0.7*syMistSpin(ro, rd, L*0.02, L*0.96, 1.0, 3.2, 0.03, 142.0, Rd*0.4, Dk*3.0);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.4*shFlakes(ro, rd, 3.32, 0.5, L - 0.5, 17.5, 2.8, 0.40, 0.026, 0.08, 0.55, 0.0, 597.0, vec3(1.000, 0.070, 0.070), vec3(0.550, 0.039, 0.039));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.36, 4.94, 0.016, 600.0, vec3(1.000, 0.070, 0.070)*0.24, vec3(0.550, 0.039, 0.039)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.070, 0.070)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
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
  a += 1.1*gxTumble(ro, rd, px, 7.0, 3.6, 3.0, 0.0, 0.3, 103.0, vec3(1.000, 0.860, 0.780), vec3(1.000, 0.070, 0.070));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

