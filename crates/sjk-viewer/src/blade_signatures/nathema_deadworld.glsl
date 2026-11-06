// Nathema Deadworld (Legendary, 3/11/2026): Nathema, where the Force was torn from every living thing: a charred black blade with fissures of dying fire. Great licks of dull flame leave it all along its length, black smoke rolls up it and a storm of embers and ash winds round it. Every so often a wave of death bursts all the way up.
// @part Charred blade
// @part Dead fire
// @part Ember storm
// @part Black smoke
// @arcs Dull strikes
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail rust
// @palette {"corona.rim_cool":[0.44,0.154,0.077],"corona.rim_hot":[0.8,0.28,0.14],"corona.inner":[0.95,0.72,0.55],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.64,0.224,0.112],"core.fringe_hot":[0.95,0.72,0.55],"light.color":[0.8,0.28,0.14],"trail":[0.8,0.28,0.14]}
// @speed 1.0

const vec3 R1 = vec3(0.520, 0.070, 0.030);
const vec3 R2 = vec3(1.000, 0.320, 0.060);
const vec3 H = vec3(1.000, 0.720, 0.360);
const vec3 K = vec3(0.012, 0.009, 0.010);
float na_top(){ return uLength*rxLife(0.0, 0.5, 0.32); }
float na_half(float y, float sd){
  float top = na_top();
  if (y > top) return 0.0;
  float tear = vn(vec2(y*1.1 - uClock*0.35, sd*5.0)) + 0.4*vn(vec2(y*3.1 + uClock*0.6, sd*9.0));
  float e = 1.0 - clamp((top - y)/2.6, 0.0, 1.0);
  return (0.42 + 0.2*tear)*sqrt(max(1.0 - e*e, 0.0))*smoothstep(-0.2, 0.5, y);
}
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  vec3 col = g*(0.72 + 0.35*sin(b.y*0.42 - uClock*2.4)*0.5 + 0.5*rxCycle(13.0, 6.0, 8.2, 9.5, 12.5) + 0.7*clamp(uClash, 0.0, 1.0));
  float top = na_top(), sd = b.x < 0.0 ? -1.0 : 1.0;
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - na_half(yc, sd), 0.0);
  col += R1*(halo(d, 0.9)*0.55 + halo(d, 3.2)*0.13);
  return col*tipFadeB(b)*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float sd = b.x < 0.0 ? -1.0 : 1.0;
  float half = na_half(b.y, sd);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  vec3 col = K + vec3(0.07, 0.06, 0.055)*smoothstep(0.35, 0.8, rxFlow(vec2(b.x*4.0, b.y*0.5)))*(0.4 + nz);
  if (part(0.0) > 0.5){
    float v1 = rxFlow(vec2(b.x*1.9 + 0.3 + 0.25*sin(uClock*0.4), b.y*0.14 - uClock*0.16));
    float v2 = rxFlow(vec2(b.x*3.7 + 5.0, b.y*0.3 + 2.0 + uClock*0.22));
    float f = max(1.0 - smoothstep(0.0, 0.05, abs(v1 - 0.5)), 0.8*(1.0 - smoothstep(0.0, 0.035, abs(v2 - 0.5))));
    float beat = 0.35 + 0.65*pow(0.5 + 0.5*sin(b.y*0.42 - uClock*2.4), 2.0);
    col += mix(R1, mix(R2, H, beat*beat), beat)*f*1.5;
  }
  col += R2*smoothstep(0.76, 1.0, abs(u))*0.95;
  col = mix(col, R2, 0.5*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
float na_death(){ return rxCycle(13.0, 6.0, 8.2, 9.5, 12.5); }
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.3, 1.1, 0.35);
  if (aft <= 0.01) return fx;
  float dth = na_death();
  if (part(1.0) > 0.5){
    // great licks of dull fire, leaving every part of the blade in turn, from every side
    for (int i = 0; i < 14; i++){
      float k = float(i);
      float run = uClock*(0.42 + 0.08*h11(k)) + k*0.29, cyc = floor(run), ph = fract(run);
      float y = L*((k + h11(k*2.3 + cyc*1.3))/14.0)*0.86 + 0.3;
      fx += (1.0 + 0.8*dth)*sin(3.1416*ph)*syLick(ro, rd, px, y, h11(k*5.7 + cyc*0.7) + 0.08*ph, 0.8 + 0.9*h11(k*3.1 + cyc) + 0.8*dth, 4.2 + 3.6*h11(k*4.3 + cyc), 0.8, k*3.0 + cyc, vec3(1.0, 0.5, 0.16), R2*0.85, R1*0.8);
    }
    // the wave of death: every so often sparks burst off it all the way up
    for (int i = 0; i < 4; i++){
      float k = float(i), q = (mod(uClock, 13.0) - 8.0 - k*0.3)/1.5;
      fx += 1.2*rxSparkShower(ro, rd, px, vec3(0.0, L*(0.15 + 0.23*k), 0.0), q, 24.0, 5.0, floor(uClock/13.0) + k*4.0, H, R2);
    }
  }
  if (part(2.0) > 0.5){
    // embers and ash carried up in a slow storm round it
    fx += 1.45*xDust(ro, rd, 1.3, 3.8, 1.5, uClock*0.09, L*0.03, L*0.97, 0.95, 51.0, H, R2);
    fx += 1.25*xDust(ro, rd, 3.2, 1.2, -1.3, uClock*0.07 + 0.4, L*0.03, L*0.97, 0.8, 53.0, R2, R1);
    fx += 1.5*shFlakes(ro, rd, 3.9, 0.5, L - 0.4, 26.0, 2.8, 1.1, 0.16, 0.14, 0.42, 0.5, 52.0, vec3(0.42, 0.36, 0.33), R1*0.5);
  }
  if (part(3.0) > 0.5) fx += 0.75*syMistSpin(ro, rd, 0.3, L*0.95, 1.1, 4.2, -0.05, 54.0, R1*0.45, K*5.0);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  if (uFx <= 0.0 || part(3.0) < 0.5 || fxNear(ro, rd, 0.0, uLength) > 16.0) return vec4(0.0);
  vec4 s = rxSmoke(ro, rd, 0.6, uLength - 0.4, 3.4, 1.5, 7.0, K*2.5, 0.75);
  s.a *= 0.85*uFx*rxLife(0.3, 1.1, 0.5)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return s;
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(3.0) > 0.5){
    a += 1.3*syPetalsAir(ro, rd, 10.0, 3.45, 0.039, 1164.0, vec3(0.950, 0.300, 0.080), vec3(0.620, 0.100, 0.040));
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
vec3 sig_vol_m26(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.42*gxKin(ro, rd, 5.0, 5.0, 3.00, 0.030, 0.80, 0.08, 0.96, 1.0, 9238.0, vec3(0.950, 0.300, 0.080), vec3(0.620, 0.100, 0.040));
  a += 1.25*gxTumble(ro, rd, px, 12.0, 5.2, 1.0, 0.0, 0.3, 19.0, vec3(0.950, 0.300, 0.080), vec3(0.620, 0.100, 0.040));
  a += 1.05*gxLanterns(ro, rd, px, 4.0, 4.0, 20.0, vec3(0.950, 0.300, 0.080), vec3(0.620, 0.100, 0.040));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(53.0, 19.0, 29.0, 35.0, 45.0), L = uLength;
  a += 1.35*gxField(ro, rd, px, 15.0, m, 1.1, 6.0, -0.35, -1.0, 0.0, 0.32, 1115.0, vec3(0.950, 0.300, 0.080), vec3(0.620, 0.100, 0.040));
  return base*(1.0 - 0.6*m) + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

