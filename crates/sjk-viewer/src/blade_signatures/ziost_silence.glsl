// Ziost Silence (Legendary, 3/11/2026): Ziost as the Emperor eats it: pale stone split by black cracks, torn open in three wounds. Long rivers of life bend in to them from far off, an aurora of drained souls hangs round it and pieces of the world tear loose and spiral in. Then the silence: everything stops, and a cold shell is all that is left.
// @part Black cracks
// @part Rivers of life
// @part Falling world
// @part The silence
// @motes Drawn dust
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail ash
// @palette {"corona.rim_cool":[0.396,0.495,0.55],"corona.rim_hot":[0.72,0.9,1.0],"corona.inner":[1,1,1],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.576,0.72,0.8],"core.fringe_hot":[1,1,1],"light.color":[0.72,0.9,1],"trail":[0.72,0.9,1]}
// @speed 1.0

const vec3 C1 = vec3(0.500, 0.800, 1.000);
const vec3 C2 = vec3(0.200, 0.420, 0.950);
const vec3 Kk = vec3(0.006, 0.008, 0.016);
const vec3 H = vec3(1.000, 1.000, 1.000);
float zi_top(){ return uLength*rxLife(0.0, 0.45, 0.32); }
float zi_hush(){ return part(3.0) > 0.5 ? rxCycle(21.0, 13.0, 15.5, 17.0, 20.0) : 0.0; }
// The wound: where it all goes, wandering slowly along the blade.
float zi_wy(){ return uLength*(0.5 + 0.17*sin(uClock*0.27)); }
float zi_wk(float k){ return uLength*(0.2 + 0.3*k) + 1.4*sin(uClock*0.27 + k*2.1); }
float zi_beat(float y){ return pow(0.5 + 0.5*sin(abs(y - zi_wy())*0.55 - uClock*2.6), 3.0); }
float zi_pull(float y){ float dy = (y - zi_wy())/5.0; return exp(-dy*dy); }
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  float h = zi_hush(), top = zi_top();
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)), 0.0);
  float pull = zi_pull(b.y);
  vec3 col = g*(1.0 - 0.8*h)*(0.55 + 0.4*pull + 0.5*zi_beat(b.y) + 0.6*clamp(uClash, 0.0, 1.0));
  col += C2*(halo(d, 1.7)*(0.14*(1.0 - h) + 0.06) + halo(d, 4.6)*(0.04 + 0.16*h));
  // the wound's own light
  for (int i = 0; i < 3; i++){
    float dw = length(vec2(b.x, (b.y - zi_wk(float(i)))*0.55));
    col += (H*halo(dw, 0.5)*0.65 + C1*(halo(dw, 1.5)*0.36 + halo(dw, 3.8)*0.08))*(1.0 - h)*rxLife(0.4, 1.0, 0.3);
  }
  // as it lights: white fire at the point of the crack going up it
  float rising = 1.0 - smoothstep(0.55, 1.0, rxLife(0.0, 0.6, 0.32));
  col += (H*halo(length(vec2(b.x, b.y - top)), 0.7)*1.5 + C1*halo(length(vec2(b.x, b.y - top)), 2.6)*0.8)*rising;
  return col*tipFadeB(b)*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength, top = zi_top();
  if (b.y < -0.3 || b.y > L + 0.3 || b.y > top + 0.2) return o;
  float e = 1.0 - clamp((top - b.y)/2.0, 0.0, 1.0);
  float half = 0.54*sqrt(max(1.0 - e*e, 0.0));
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  float sh = rxFlow(vec2(b.x*1.3 + 0.3*sin(uClock*0.5), b.y*0.08 - uClock*0.14));
  vec3 col = mix(vec3(0.62, 0.68, 0.76), vec3(0.93, 0.97, 1.0), smoothstep(0.25, 0.75, sh));
  col *= 0.5 + 0.62*nz;
  float from_ = abs(b.y - zi_wy());
  if (part(0.0) > 0.5){
    // black cracks with a torn line: wide and narrow by turns, branching, light bleeding from their lips, spreading from the wound
    float reach = 3.0 + 11.0*fract(uClock*0.045);
    float front = max(1.0 - smoothstep(reach - 4.0, reach, from_), 1.0 - smoothstep(0.6, 2.2, uAge));
    vec2 tear = 0.16*vec2(vn(vec2(b.x*3.2, b.y*1.1)) - 0.5, vn(vec2(b.x*2.1 + 7.0, b.y*0.8)) - 0.5);
    float v1 = rxFlow(vec2(b.x*1.6 + 0.3 + tear.x*2.0, b.y*0.13 - 0.05*uClock + tear.y));
    float v2 = rxFlow(vec2(b.x*3.3 + 7.0 + tear.y*3.0, b.y*0.28 + 2.0 + tear.x));
    float v3 = rxFlow(vec2(b.x*6.0 + 2.0 + tear.x*5.0, b.y*0.55 + tear.y*2.0));
    float w1 = 0.018 + 0.05*vn(vec2(b.y*0.34, 5.0)), w2 = 0.012 + 0.03*vn(vec2(b.y*0.6, 9.0));
    float c1 = 1.0 - smoothstep(w1*0.45, w1, abs(v1 - 0.5));
    float c2 = 1.0 - smoothstep(w2*0.45, w2, abs(v2 - 0.5));
    float c3 = (1.0 - smoothstep(0.006, 0.018, abs(v3 - 0.5)))*smoothstep(0.52, 0.68, vn(vec2(b.y*0.45, 3.0)));
    float crack = max(max(c1, 0.9*c2), 0.75*c3)*max(front, 0.2);
    float lip = (1.0 - smoothstep(w1, w1*4.2, abs(v1 - 0.5)))*(1.0 - c1) + 0.6*(1.0 - smoothstep(w2, w2*4.0, abs(v2 - 0.5)))*(1.0 - c2);
    col = mix(col, Kk, crack);
    col += mix(C1, H, 0.35)*lip*front*(0.42 + 0.6*zi_beat(b.y));
  }
  // the wound: the stone torn open and blazing
  float wo = 0.0;
  for (int i = 0; i < 3; i++) wo = max(wo, 1.0 - smoothstep(0.1, 1.2, length(vec2(b.x*1.4, (b.y - zi_wk(float(i)))*0.5))));
  col = mix(col, H, wo*(1.0 - zi_hush()));
  col += C1*smoothstep(0.72, 1.0, abs(u))*(0.3 + 0.8*zi_beat(b.y));
  col = mix(col, C1*1.1, 0.3*zi_beat(b.y)*(1.0 - abs(u)));
  col = mix(col, col*0.34, 0.85*zi_hush());
  col = mix(col, H, 0.5*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
// A river of life: from far off, bending, into the wound.
vec3 zi_river(vec3 ro, vec3 rd, float px, float k, float hush){
  float L = uLength, wy = zi_wk(mod(k, 3.0));
  float run = uClock*0.1 + k*0.37;
  float cyc = floor(run), ph = fract(run);
  float a = k/8.0 + 0.11*h11(k*2.3 + cyc*1.7) + 0.05*uClock;
  float side = mod(k, 2.0) < 0.5 ? 1.0 : -1.0;
  float R = 5.2 + 3.0*h11(k*3.1 + cyc), span = (5.0 + 6.0*h11(k*5.3 + cyc))*side;
  float y0 = clamp(wy + span, 1.5, L - 1.5);
  float tm;
  if (rayPtRaw(ro, rd, around(R*0.5, a + 0.14, mix(y0, wy, 0.5)), tm) > R*0.8 + abs(y0 - wy)*0.65 + 2.0) return vec3(0.0);
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(R, a, y0);
  for (int j = 1; j <= 10; j++){
    float f = float(j)/10.0;
    float e = f*f*(3.0 - 2.0*f);
    vec3 q = around(mix(R, 0.25, e) + 0.3*sin(f*8.0 - uClock*1.6 + k)*(1.0 - f), a + 0.3*f, mix(y0, wy, sqrt(f)));
    float dd = raySeg(ro, rd, prev, q, s);
    { float wN = bW(dd); bWs += wN; best = min(best, dd); bf += ((float(j) - 1.0 + s)/10.0 - bf)*wN/bWs; }
    prev = q;
  }
  float life = sin(3.1416*ph);
  float bead = pow(0.5 + 0.5*sin(bf*24.0 - uClock*6.0 + k*2.0), 4.0);
  float ww = mix(0.13, 0.04, bf);
  vec3 col = H*halo(best, max(ww*0.55, px))*(0.5 + 1.5*bead) + C1*(halo(best, ww*2.4)*(0.7 + 0.8*bead) + halo(best, 0.85)*0.2);
  return col*smoothstep(0.0, 0.3, bf)*life*(1.0 + 1.2*smoothstep(0.65, 1.0, bf))*(1.0 - hush);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.3, 1.0, 0.35);
  if (aft <= 0.01) return fx;
  float h = zi_hush();
  if (part(1.0) > 0.5) for (int i = 0; i < 8; i++) fx += zi_river(ro, rd, px, float(i), h);
  if (part(2.0) > 0.5){
    fx += (1.0 - 0.85*h)*1.5*shFlakes(ro, rd, 3.6, 0.5, L - 0.4, 26.0, 2.8, 0.9, 0.16, 0.16, 0.42, 0.5, 41.0, C1*0.85, Kk*12.0);
    fx += (1.0 - h)*0.8*syMistSpin(ro, rd, 0.3, L*0.97, 1.3, 5.0, 0.04, 42.0, C2*0.32, Kk*10.0);
    fx += (1.0 - h)*0.9*syMistSpin(ro, rd, 0.3, L*0.97, 1.8, 5.8, -0.03, 43.0, C1*0.2, C2*0.16);
  }
  if (part(3.0) > 0.5) fx += h*(1.05*rxAuraShell(ro, rd, -0.3, L + 0.3, 2.5, 3.0, Kk*5.0, C1));
  // going out: the stone falls away as ash
  if (uAge >= 2000.0) fx += (1.0 - smoothstep(0.0, 0.45, uAge - 2000.0))*2.2*shFlakes(ro, rd, 2.2, 0.5, L - 0.4, 24.0, 2.2, -5.0, 0.1, 0.14, 0.7, 0.0, 44.0, H, C1);
  return fx*(aft + (uAge >= 2000.0 ? 1.0 - aft : 0.0)*0.0)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// Pieces of Ziost's stone torn off the world and dragged in spirals toward the wounds, cracking as they go.
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  float L = uLength;
  if (uFx <= 0.0 || part(2.0) < 0.5 || fxNear(ro, rd, 0.0, L) > 16.0) return o;
  float best = 1e9, h = zi_hush();
  for (int i = 0; i < 10; i++){
    float k = float(i), t;
    float run = uClock*(0.085 + 0.02*h11(k*1.7)) + k*0.27, n = floor(run), q = fract(run);
    float wy = zi_wk(mod(k, 3.0));
    float ya = clamp(wy + (5.0 + 3.0*h11(k + n))*(mod(k + n, 2.0) < 0.5 ? 1.0 : -1.0), 1.0, L - 1.0);
    vec3 c = around(mix(5.2 + 1.2*h11(k*3.1 + n), 0.8, q*q), h11(k*5.3 + n) + 0.55*q, mix(ya, wy, q*q));
    float d0 = rayPt(ro, rd, c, t);
    if (t > best) continue;
    float R0 = (0.42 + 0.46*h11(k*9.1 + n))*(1.0 - 0.6*q*q)*smoothstep(0.0, 0.22, q)*(1.0 - smoothstep(0.78, 1.0, q));
    if (R0 < 0.02) continue;
    if (d0 > R0*1.5 + 0.08) continue;
    // a torn lump of rock: its outline noise taken round the view ray, so no seam and no figure
    vec3 w0 = ro + rd*t - c;
    vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
    vec2 dir = vec2(dot(w0, e1), dot(w0, e2))/max(d0, 0.001);
    float spin = uClock*(0.5 + 0.5*h11(k));
    vec2 tdir = rot(spin)*dir;
    float rag = R0*(0.52 + 0.75*vn(tdir*1.5 + vec2(k*3.0, n)));
    float cov = 1.0 - smoothstep(rag - 0.03 - px, rag + 0.03 + px, d0);
    if (cov <= 0.0) continue;
    best = t;
    float lit = 0.5 + 0.5*dot(dir, normalize(vec2(0.3, 1.0)));
    float rim = smoothstep(rag*0.68, rag, d0);
    vec3 col = mix(vec3(0.07, 0.085, 0.12), vec3(0.34, 0.39, 0.47), lit)*(1.0 - 0.6*h) + C1*rim*(0.5 + 1.5*q)*(1.0 - 0.8*h);
    o = vec4(col, cov*(1.0 - behindBlade(ro, rd, t)));
  }
  o.a *= uFx*rxLife(0.3, 1.0, 0.35)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return o;
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(3.0) > 0.5){
    a += 1.4*syBubbles(ro, rd, 10.0, 4.14, 0.055, 1057.0, vec3(0.620, 0.840, 1.000), vec3(0.200, 0.420, 0.950));
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
  a += 1.27*gxKin(ro, rd, 5.0, 4.0, 3.00, 0.030, 0.70, 0.08, 0.96, 1.0, 9493.0, vec3(0.620, 0.840, 1.000), vec3(0.200, 0.420, 0.950));
  a += 1.0*gxVortex(ro, rd, px, 12.0, 6.0, 218.0, vec3(0.620, 0.840, 1.000), vec3(0.200, 0.420, 0.950));
  a += 1.3*gxTumble(ro, rd, px, 12.0, 6.0, 2.0, 0.0, 0.55, 18.0, vec3(0.620, 0.840, 1.000), vec3(0.200, 0.420, 0.950));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(56.0, 20.0, 30.0, 36.0, 46.0), L = uLength;
  a += 1.45*gxField(ro, rd, px, 16.0, m, 1.3, 6.0, 0.25, -1.0, 0.0, 0.5, 1114.0, vec3(0.620, 0.840, 1.000), vec3(0.200, 0.420, 0.950));
  return base*1.0 + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

