// Malevolence Ion Beam (rebuilt 3/11/2026): The ion cannons of the Malevolence: a narrow channel with charges racing up it faster and faster, and plasma tendrils wandering off it to earth in the air. Every thirteen seconds the guns fire: the whole blade goes white, swells, and a storm of ions leaves the point.
// @part Charging channel
// @part Plasma tendrils
// @part The guns fire
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail ion
// @palette {"corona.rim_cool":[0.121,0.341,0.55],"corona.rim_hot":[0.22,0.62,1],"corona.inner":[0.64,0.9,1],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.176,0.496,0.8],"core.fringe_hot":[0.64,0.9,1],"light.color":[0.22,0.62,1],"trail":[0.22,0.62,1]}
// @speed 1.0

const vec3 B = vec3(0.220, 0.620, 1.000);
const vec3 D = vec3(0.010, 0.070, 0.220);
const vec3 Ic = vec3(0.640, 0.900, 1.000);
const vec3 H = vec3(1.000, 1.000, 1.000);
float ma_top(){ return uLength*rxLife(0.0, 0.42, 0.28); }
// The guns: a 13 second cycle, 9 of it charging, 2 firing.
float ma_charge(){ return smoothstep(1.0, 9.0, mod(uClock, 13.0))*(1.0 - smoothstep(9.0, 9.3, mod(uClock, 13.0))); }
float ma_fire(){ float p = mod(uClock, 13.0); return smoothstep(9.0, 9.15, p)*(1.0 - smoothstep(10.2, 11.2, p)); }
float ma_half(float y){
  float top = ma_top();
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/1.0, 0.0, 1.0);
  return (0.44 + 0.08*ma_charge() + 0.26*ma_fire())*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float top = ma_top();
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - ma_half(yc), 0.0);
  float ch = ma_charge(), fi = ma_fire();
  vec3 col = B*(halo(d, 0.28)*(0.85 + 0.5*ch + 0.7*fi) + halo(d, 1.4)*(0.38 + 0.25*ch + 0.5*fi) + halo(d, 4.2)*(0.11 + 0.08*ch + 0.28*fi));
  col += lvl(g)*B*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = ma_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.04 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.1 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  float ch = ma_charge(), fi = ma_fire();
  vec3 col = mix(D, B, 0.35 + 0.65*abs(u));
  if (part(0.0) > 0.5){
    float run = fract(b.y*0.1 - uClock*0.55);
    col += H*exp(-pow((run - 0.5)/0.1, 2.0))*(1.0 - abs(u)*0.7)*(0.55 + 0.7*ch);
  }
  col = mix(col, H, fi*0.9);
  col = mix(col, H, 0.5*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
// Plasma-globe tendrils: one wandering filament from the blade out to where it earths in the air.
vec3 ma_tendril(vec3 ro, vec3 rd, float px, float k, float ch, float fi){
  float L = uLength;
  float y = L*(0.08 + 0.84*(k + 0.5)/8.0) + 0.9*sin(uClock*0.5 + k*2.4);
  float a = k*0.382 + 0.1*sin(uClock*0.33 + k*1.3);
  float len = (4.6 + 1.6*ch + 1.8*fi)*(0.8 + 0.3*sin(uClock*0.8 + k*3.1));
  vec3 p0 = around(0.32, a, y), p1 = around(0.32 + len, a + 0.03*sin(uClock*0.6 + k), y + (2.6*sign(sin(k*7.7)) + 0.9*sin(uClock*0.42 + k*1.9)));
  vec3 m = around(0.32 + len*0.72, a, y + 0.25*sin(uClock*0.7 + k*0.9));
  float tm;
  if (rayPtRaw(ro, rd, m, tm) > len + 4.0) return vec3(0.0);
  // re-shaped at Storm's rate, so it crackles as Storm's bolts do
  float d = liveBolt(ro, rd, p0, m, p1, 0.3 + 0.25*ch, k*3.7, px);
  float t, de = rayPt(ro, rd, p1, t);
  float flick = 0.7 + 0.3*h11(floor(uClock*22.0) + k);
  vec3 col = (H*halo(d, max(0.035, px))*1.6 + Ic*halo(d, 0.13)*1.2 + B*halo(d, 0.7)*0.48)*flick;
  col += (H*halo(de, 0.07)*1.2 + Ic*halo(de, 0.26)*0.5 + B*halo(de, 0.8)*0.1)*flick;
  return col*(0.72 + 0.4*ch + 0.5*fi);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.1, 0.5, 0.3);
  if (aft <= 0.01) return fx;
  float ch = ma_charge(), fi = ma_fire();
  if (part(1.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 8.0) for (int i = 0; i < 8; i++) fx += ma_tendril(ro, rd, px, float(i), ch, fi);
  if (part(2.0) > 0.5){
    float p = mod(uClock, 13.0), q = (p - 9.0)/2.2;
    fx += fi*1.1*shSheath(ro, rd, 1.9, L*0.03, L*0.96, 2.4, 0.5, 2.4, 0.5, 122.0, B*0.8, H*0.7);
    fx += 1.4*rxSparkShower(ro, rd, px, vec3(0.0, L - 1.5, 0.0), q, 22.0, 5.2, floor(uClock/13.0), H, Ic);
  }
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_vol_beam(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(2.0) > 0.5){
    a += 1.3*vec3(0.0);
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

// Charge collecting on the ion cannon: beads of light climbing it in two strands, and now and then a great soft star.
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_beam(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  a += 1.15*gxHelix(ro, rd, px, 16.0, 2.6, 1.8, 0.05, 11.0, vec3(0.85, 0.95, 1.0), vec3(0.3, 0.55, 1.0));
  a += 1.0*gxFlares(ro, rd, px, 5.0, 5.2, 12.0, vec3(0.9, 0.96, 1.0), vec3(0.3, 0.5, 1.0));
  return base + a*rxLife(0.8, 2.6, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
