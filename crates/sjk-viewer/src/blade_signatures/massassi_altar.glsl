// Massassi Altar (rebuilt 3/11/2026): An altar of the Massassi on Yavin 4: rough stones stacked into a blade, each slowly shifting on the next, with fire burning in the gaps between them. Flames lick up from every gap, fire runs through old carved channels and dark smoke turns round it.
// @part Carved stone
// @part Altar fires
// @part Altar smoke
// @part Sparks
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail cinder
// @palette {"corona.rim_cool":[0.55,0.187,0.028],"corona.rim_hot":[1,0.34,0.05],"corona.inner":[1,0.93,0.66],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.8,0.272,0.04],"core.fringe_hot":[1,0.93,0.66],"light.color":[1,0.34,0.05],"trail":[1,0.34,0.05]}
// @speed 1.0

const vec3 St = vec3(0.290, 0.210, 0.160);
const vec3 Dk = vec3(0.040, 0.025, 0.020);
const vec3 O = vec3(1.000, 0.340, 0.050);
const vec3 Y = vec3(1.000, 0.700, 0.170);
const vec3 H = vec3(1.000, 0.930, 0.660);
float al_top(){ return uLength*rxLife(0.0, 0.6, 0.32); }
const float AL_N = 6.0;
// Which block y sits in and how far through it: stacked stones with gaps of fire between.
float al_blocks(float y, out float id){
  float L = max(uLength, 1.0), p = y/L*AL_N;
  id = floor(p);
  return fract(p);
}
float al_shift(float id){ return 0.13*sin(uClock*0.35 + id*2.3); }
float al_half(float y, out float gap){
  float top = al_top(), id, f = al_blocks(y, id);
  gap = 0.0;
  if (y > top) return 0.0;
  float L = max(uLength, 1.0), step_ = L/AL_N;
  float edge = min(f, 1.0 - f)*step_;
  gap = 1.0 - smoothstep(0.0, 0.42, edge);
  float taper = mix(0.86, 0.5, clamp(y/L, 0.0, 1.0));
  float e = 1.0 - clamp((top - y)/1.0, 0.0, 1.0);
  return taper*(1.0 - 0.72*gap)*sqrt(max(1.0 - e*e, 0.0))*smoothstep(-0.25, 0.4, y);
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  float top = al_top(), gap, id;
  float yc = clamp(b.y, 0.0, top);
  float f = al_blocks(yc, id);
  float x = b.x - al_shift(id);
  float d = max(length(vec2(x, b.y - yc)) - al_half(yc, gap), 0.0);
  vec3 col = O*(halo(d, 0.26)*(0.65 + 1.1*gap) + halo(d, 1.4)*(0.38 + 0.42*gap) + halo(d, 3.8)*0.17);
  col += lvl(g)*O*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.7*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float gap, id;
  float half = al_half(b.y, gap);
  float f = al_blocks(b.y, id);
  float x = b.x - al_shift(id);
  float cov = (1.0 - smoothstep(half - 0.07 - px, half + 0.04 + px, abs(x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  float rough = rxFlow(vec2(x*3.2 + id*7.0, b.y*0.8));
  vec3 col = mix(Dk*3.0, St*1.5, smoothstep(0.25, 0.75, rough))*(0.5 + 0.9*nz);
  if (part(0.0) > 0.5){
    // old carved channels, with fire running in them from the gaps
    float cut = rxFlow(vec2(x*1.8 + id*3.0, b.y*0.22 + id));
    float line = 1.0 - smoothstep(0.0, 0.06, abs(cut - 0.5));
    col = mix(col, Dk, line*0.8);
    float flow = pow(0.5 + 0.5*sin(b.y*0.9 + uClock*1.3 + id), 3.0);
    col += mix(O, Y, flow)*line*(0.5 + 1.8*flow);
  }
  col = mix(col, mix(Y, H, gap), smoothstep(0.1, 0.9, gap));
  col = mix(col, H, 0.35*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.25, 1.0, 0.35);
  if (aft <= 0.01) return fx;
  if (part(1.0) > 0.5) for (int i = 1; i < 6; i++){
    float k = float(i);
    float y = L*k/AL_N;
    for (int j = 0; j < 3; j++){
      float m = float(j);
      float a = m/3.0 + k*0.17 + 0.02*sin(uClock*0.6 + m);
      float lick = 0.72 + 0.28*sin(uClock*(2.2 + 0.6*m) + k*3.0 + m*2.0);
      fx += 1.5*rxTongue(ro, rd, y - 0.05, a, (3.2 + 1.6*h11(k*3.0 + m))*lick, 0.55, 0.8, k*2.0 + m*5.0, H, Y, O*0.7);
    }
  }
  if (part(2.0) > 0.5) fx += 0.75*syMistSpin(ro, rd, L*0.04, L*0.96, 1.2, 3.6, 0.012, 131.0, vec3(0.2, 0.09, 0.05), vec3(0.04));
  if (part(3.0) > 0.5) fx += 1.3*gxKin(ro, rd, 5.0, 4.0, 2.40, 0.040, 0.60, 0.02, 0.50, 1.0, 7702.0, H, O);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.6*vec3(0.0);
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.56, 5.04, -0.015, 811.0, vec3(1.000, 0.340, 0.050)*0.24, vec3(0.550, 0.187, 0.028)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.340, 0.050)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
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
  a += 1.1*gxMeteors(ro, rd, px, 3.0, 3.6, 102.0, vec3(1.000, 0.930, 0.660), vec3(1.000, 0.340, 0.050));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

