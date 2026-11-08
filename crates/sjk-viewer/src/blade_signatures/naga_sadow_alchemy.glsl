// Naga Sadow Alchemy (rebuilt 3/11/2026): Naga Sadow's forbidden alchemy at the boil: a blade of swallowing blood with bubbles climbing through it. Heavy ropes of blood are thrown out of its sides, arc through the air and fall back in, fumes roll off it and thick drops fall from it.
// @part Boiling blood
// @part Thrown ropes
// @part Alchemic fume
// @part Falling drops
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail blood
// @palette {"corona.rim_cool":[0.55,0.187,0.11],"corona.rim_hot":[1.0,0.34,0.2],"corona.inner":[1,0.86,0.72],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.8,0.272,0.16],"core.fringe_hot":[1,0.86,0.72],"light.color":[1,0.34,0.2],"trail":[1,0.34,0.2]}
// @speed 1.0

const vec3 R = vec3(0.900, 0.050, 0.070);
const vec3 D = vec3(0.070, 0.004, 0.012);
const vec3 O = vec3(1.000, 0.360, 0.110);
const vec3 H = vec3(1.000, 0.880, 0.720);
float ng_top(){ return uLength*rxLife(0.0, 0.5, 0.32); }
float ng_boil(){ return rxCycle(18.0, 9.0, 11.2, 13.2, 16.2); }
float ng_half(float y){
  float top = ng_top();
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/1.4, 0.0, 1.0);
  float gulp = 0.13*sin(y*0.55 - uClock*1.5) + 0.07*sin(y*1.35 + uClock*0.9);
  return (0.56 + gulp*smoothstep(0.5, 3.0, y))*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float top = ng_top();
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - ng_half(yc), 0.0);
  float st = ng_boil();
  vec3 col = R*(halo(d, 0.3)*0.95 + halo(d, 1.4)*(0.42 + 0.3*st) + halo(d, 4.0)*(0.13 + 0.12*st));
  col += lvl(g)*R*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = ng_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  vec3 col = R;
  if (part(0.0) > 0.5){
    float v = rxFlow(vec2(b.x*1.7, b.y*0.13 - uClock*0.3));
    col = mix(D, R, smoothstep(0.18, 0.72, v));
    col += O*smoothstep(0.6, 0.84, v)*0.7;
    // bubbles climbing through the liquid
    float q = vn(vec2(b.x*3.4, b.y*0.9 - uClock*1.6));
    col += mix(O, H, 0.6)*smoothstep(0.72, 0.84, q)*0.85;
  }
  col *= 0.55 + 0.75*nz;
  col += O*pow(1.0 - nz, 3.0)*0.9;
  col = mix(col, H, 0.45*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
// A rope of blood thrown out of the blade, arcing and falling back into it.
vec3 ng_rope(vec3 ro, vec3 rd, float px, float k, float boil){
  float L = uLength;
  float run = uClock*(0.2 + 0.05*h11(k*1.9)) + k*0.29;
  float cyc = floor(run), ph = fract(run);
  float a = h11(k*2.3 + cyc*1.7), y = L*(0.08 + 0.62*h11(k*5.1 + cyc*0.9));
  float tall = 3.6 + 1.8*h11(k*3.1 + cyc) + 1.4*boil, span = 3.5 + 2.8*h11(k*6.3 + cyc);
  float tm;
  if (rayPtRaw(ro, rd, around(1.0 + tall*0.5, a, y + span*0.5), tm) > tall + span*0.6 + 2.0) return vec3(0.0);
  float head = ph*1.7, tail = head - 0.85;
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s;
  // only the living stretch of the arc, its two ends sliding smoothly: the rope never jumps a segment
  float f0 = clamp(tail, 0.0, 1.0), f1 = clamp(head, 0.0, 1.0);
  if (f1 - f0 < 0.002) return vec3(0.0);
  vec3 prev = around(0.5 + tall*sin(3.1416*f0), a + 0.03*sin(6.28*f0), y + span*f0);
  for (int j = 1; j <= 9; j++){
    float f = mix(f0, f1, float(j)/9.0);
    vec3 q = around(0.5 + tall*sin(3.1416*f), a + 0.03*sin(6.28*f), y + span*f);
    float dd = raySeg(ro, rd, prev, q, s);
    { float wN = bW(dd); bWs += wN; best = min(best, dd); bf += (mix(f0, f1, (float(j) - 1.0 + s)/9.0) - bf)*wN/bWs; }
    prev = q;
  }
  if (best > 3.0) return vec3(0.0);
  float at = clamp((bf - tail)/max(head - tail, 0.01), 0.0, 1.0);
  float ww = mix(0.05, 0.27, at*at);
  float fade = smoothstep(0.0, 0.25, at);
  vec3 col = (H*halo(best, max(ww*0.28, px))*(0.5 + 1.3*at) + mix(R, O, at*0.5)*halo(best, ww)*1.4 + R*(halo(best, ww*3.0)*0.75 + halo(best, 1.3)*0.2));
  return col*fade*(1.0 - smoothstep(1.4, 1.7, head));
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.2, 0.9, 0.35);
  if (aft <= 0.01) return fx;
  float st = ng_boil(), C_N = uClock;
  if (part(1.0) > 0.5) for (int i = 0; i < 5; i++) fx += ng_rope(ro, rd, px, float(i), st);
  if (part(2.0) > 0.5){
    fx += 1.05*syMistSpin(ro, rd, L*0.03, L*0.96, 1.1, 4.0, 0.015, 21.0, R*0.42, D*6.0);
    // the fumes of the work: tall dark-orange vapours curling off the blade, and embers going up with them
    for (int i = 0; i < 4; i++){
      float k = float(i);
      float run = C_N*(0.16 + 0.04*h11(k)) + k*0.31, cyc = floor(run), ph = fract(run);
      float en = sin(3.1416*ph);
      fx += en*en*syLick(ro, rd, px, L*(0.06 + 0.7*h11(k*2.3 + cyc)), h11(k*5.7 + cyc) + 0.06*ph, 1.3 + 1.2*h11(k*3.1 + cyc), 4.2 + 3.0*h11(k*4.3 + cyc), 0.5, k*3.0 + cyc, O*0.9, R*0.7 + O*0.15, D*3.0);
    }
    fx += (1.5 + st)*gxKin(ro, rd, 2.0, 6.0, 2.20, 0.140, 0.20, 0.08, 0.96, -1.0, 7780.0, H, O);
  }
  if (part(3.0) > 0.5) fx += sxDrops(ro, rd, px, 8.0, 1.25, 0.09, 22.0, H, O);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
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
  a += 1.30*gxKin(ro, rd, 5.0, 4.0, 2.60, 0.050, 0.60, 0.08, 0.96, 1.0, 9221.0, vec3(1.000, 0.860, 0.720), vec3(1.000, 0.340, 0.200));
  a += 1.2*vec3(0.0);
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

