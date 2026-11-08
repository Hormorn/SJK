// Malachor Rose (id saber_malachor_rose): the Force as it was wielded on Malachor V, drawn to a single thin edge: a body of pale rose light clouded with white, climbing the blade, a fine sheen along its outline and a narrow tear in space around it. Long crescents of white light and crimson wheel round it in dark red air, with small stars.
// @part Crimson aura
// @part Torn space
// @part Light sickles
// @part Star sparks
// @part Crimson storm
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail scythe
// @palette {"corona.rim_cool":[0.42,0.03,0.2],"corona.rim_hot":[0.76,0.3,1.0],"corona.inner":[0.95,0.72,1.0],"core.white":[1.0,0.92,0.98],"core.fringe_cool":[0.66,0.34,0.98],"core.fringe_hot":[0.95,0.74,1.0],"light.color":[0.98,0.48,0.86],"trail":[0.98,0.5,0.88]}
// @speed 1.0

const vec3 RK_WHITE = vec3(1.0, 0.95, 1.0);
const vec3 RK_PALE = vec3(0.98, 0.72, 0.96);
const vec3 RK_LAV = vec3(0.82, 0.5, 0.95);
const vec3 RK_PURPLE = vec3(0.48, 0.14, 0.72);
const vec3 RK_PINK = vec3(1.0, 0.3, 0.72);
const vec3 RK_DARK = vec3(0.045, 0.004, 0.03);

// how far it has come out: a little slower than a saber, smooth at both ends
float rk_life(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, 0.45, uAge - 2000.0);
  return smoothstep(0.0, 0.55, uAge);
}
// (update 30, Creyon: no black line, a cool effect instead) the rose aura: a flame of rose light round the blade
float rk_aura_on(){
  if (part(0.0) < 0.5 || uFx <= 0.0) return 0.0;
  return uAge >= 2000.0 ? 1.0 - smoothstep(0.0, 0.25, uAge - 2000.0) : smoothstep(0.45, 1.5, uAge);
}
// its two edges across the blade at y, its whole length L: x one edge, y the other
vec2 rk_edges(float y, float L){
  // (update 30, Creyon: make the hilt normal) the blade leaves the emitter at nearly its whole width
  // (update 30, Creyon's picture) the crescent: the whole blade one long bow, thin at both ends, its belly out away
  // from the inside. Over the other shapes when it is on; both off, it is the straight blade
  float w = 1.12*mix(0.62, 1.0, smoothstep(0.0, 1.4, y));
  // the sword: a long parallel run, straight lines to the point
  w = min(w, 1.12*max(L - y, 0.0)/5.2);
  vec2 ed = vec2(-w, w);
  return ed;
}
// how far b lies outside the blade (negative inside); a clean edge
float rk_d(vec2 b, float L, out vec2 e){
  float yc = clamp(b.y, 0.0, L);
  e = rk_edges(yc, L);
  float inside = max(e.x - b.x, b.x - e.y);
  float past = max(b.y - L, -b.y);
  return past > 0.0 ? length(vec2(max(inside, 0.0), past)) : inside;
}
// torn space (update 30, after Creyon's picture): behind the blade the air opens on a crimson and violet
// night with small stars in it, and long tapered strokes of red energy sweep along the blade
// Its great moment: the light gathers slowly, the sheen, the storm and the crescents swell for a while, and it breathes out again.
float gk_surge(){ return rxCycle(44.0, 16.0, 23.0, 29.0, 38.0); }
float rk_rift_on(){
  if (part(1.0) < 0.5 || uFx <= 0.0) return 0.0;
  float g = uAge >= 2000.0 ? 1.0 - smoothstep(0.0, 0.25, uAge - 2000.0) : smoothstep(0.7, 2.4, uAge);
  return g;
}
vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float L = (uLength + 0.2)*rk_life();
  if (L < 0.15) return vec3(0.0);
  vec2 e;
  float d = max(rk_d(b, L, e), 0.0);
  // only what a burning edge gives off: thin, close, never a cloud
  // a fine sheen hugging its outline
  vec3 col = vec3(1.0, 0.62, 0.9)*(halo(d, 0.1)*1.15 + halo(d, 0.32)*0.72) + vec3(0.95, 0.2, 0.62)*halo(d, 0.85)*0.42;
  if (part(0.0) > 0.5) col += vec3(0.62, 0.03, 0.2)*(halo(d, 1.1)*0.3 + halo(d, 3.0)*0.09);
    if (part(0.0) > 0.5 && uFx > 0.0){
    float on = uAge >= 2000.0 ? 1.0 - smoothstep(0.0, 0.25, uAge - 2000.0) : smoothstep(0.5, 2.2, uAge);
    float w0 = 0.75 + 0.75*rxFlow(vec2(b.y*0.14 - uClock*0.3, b.x > e.x ? 2.0 : 6.0));
    float wall = 1.0 - smoothstep(w0*0.55, w0*1.15, d);
    float inner = 0.55 + 0.45*rxFlow(vec2(b.x*0.9 + uClock*0.1, b.y*0.2 - uClock*0.35));
    float ends = smoothstep(0.0, 2.5, b.y)*(1.0 - smoothstep(L - 0.5, L + 1.2, b.y));
    col += vec3(0.5, 0.02, 0.14)*wall*inner*0.22*on*ends;
  }
  // the crimson aura: tall soft flames streaming up both edges, dark red like the picture
  float aura = rk_aura_on();
  if (aura > 0.01 && d < 4.0){
    float side = b.x > 0.5*(e.x + e.y) ? 2.0 : 8.0;
    float f1 = rxFlow(vec2(b.y*0.3 - uClock*1.1, side));
    float f2 = rxFlow(vec2(b.y*0.75 - uClock*1.7, side + 3.0));
    float along = smoothstep(0.5, 3.5, b.y)*(1.0 - smoothstep(L - 3.0, L - 0.5, b.y));
    float reach = 0.6 + 2.2*f1*f1 + 0.7*f2*f2;
    float q = clamp(d/max(reach, 0.05), 0.0, 1.0);
    float fl = (1.0 - q)*(1.0 - q)*along;
    col += mix(vec3(0.3, 0.006, 0.08), vec3(0.8, 0.04, 0.2), fl)*fl*aura*0.5;
  }
  col *= 1.0 + 0.7*gk_surge();
  // the sickles: long strokes of white light and red energy sweeping along it, following its own curve, tapered at both ends
  if (false){
    float on = uAge >= 2000.0 ? 1.0 - smoothstep(0.0, 0.25, uAge - 2000.0) : smoothstep(0.9, 2.6, uAge);
    for (int i = 0; i < 6; i++){
      float k = float(i), white = 1.0 - mod(k, 2.0), sd = k < 2.5 ? 1.0 : -1.0;
      float run = uClock*(0.085 + 0.02*h11(k*5.3)) + h11(k*7.7), cyc = floor(run), q = fract(run);
      float at = 0.25 + 0.7*h11(k*3.1 + cyc);
      float cy = L*(q*1.4 - 0.2), len = L*(0.3 + 0.16*h11(k*2.9 + cyc));
      float u0 = clamp((b.y - cy)/len, -1.0, 1.0), taper = max(1.0 - u0*u0, 0.0);
      float off = sd*(b.x - 0.5*(e.x + e.y)) > 0.0 ? d - at : 1e3;
      float wide = 0.025 + 0.09*taper*taper;
      vec3 hot = mix(vec3(1.0, 0.1, 0.28), vec3(1.0, 0.92, 1.0), white), soft = mix(vec3(0.6, 0.01, 0.13), vec3(0.75, 0.36, 1.0), white);
      col += (hot*halo(abs(off), wide)*0.7 + soft*halo(abs(off), wide*3.0)*0.22)*taper*taper*sin(3.1416*q)*on;
    }
  }
  return col*roomFade(abs(b.x))*(1.0 + 0.6*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core_d(vec3 c, vec2 b, float px){ return c*0.0; }
// How far the edge's white band frays here, small flames running up it.
float detail_jag(vec2 b, float px){
  return 0.13*(1.0 - smoothstep(0.08, 0.3, px))*pow(rxFlow(vec2(b.y*2.2 - uClock*1.5, b.x > 0.0 ? 2.0 : 7.0)), 2.0);
}
vec4 sig_solid_flat(vec4 o, vec2 b, float px){
  float L = (uLength + 0.2)*rk_life();
  if (L < 0.15 || b.y < -0.5 || b.y > L + 2.5) return o;
  vec2 e;
  float d = rk_d(b, L, e);
  vec4 res = o;
  float ends = smoothstep(0.0, 1.5, b.y);
  // the torn space behind it: a wound in the air flowing outward from the blade: black voids, crimson cloud with glowing
  // filaments in it, two depths of small twinkling stars, a hot edge against the blade. Everything soft where it ends.
  float rift = rk_rift_on();
  if (rift > 0.01 && d > -0.05 && d < 4.2){
    float side = b.x > 0.5*(e.x + e.y) ? 3.0 : 8.0;
    // its outline is torn, not a smooth shape: two sizes of fray, all of it feathered, and it opens and closes slowly
    vec2 tq = vec2(d*0.35 + side*3.0, b.y*0.13 - uClock*0.035);
    float frayed = d + (rxFlow(tq) - 0.5)*0.5 + (rxFlow(tq*2.7 + 4.0) - 0.5)*0.26;
    float reach = (0.65 + 1.15*rxFlow(vec2(b.y*0.09 - uClock*0.06, side)))*smoothstep(0.0, L*0.12, b.y)*(1.0 - smoothstep(L*0.9, L*1.04, b.y));
    float a = (1.0 - smoothstep(reach*0.25, reach*1.2, frayed))*rift*0.62*roomFade(abs(b.x));
    if (a > 0.003){
      // the far side of the tear: a black night, crimson gas drawn slowly in towards the blade, a magenta heart in the gas
      vec2 q = vec2(d*0.5 + uClock*0.12 + side, b.y*0.15 + 0.05*sin(uClock*0.13 + side));
      vec2 warp = vec2(rxFlow(q*1.1 + 3.0), rxFlow(q*1.3 + 7.0)) - 0.5;
      float n1 = rxFlow(q + warp*1.5), n2 = rxFlow(q*2.1 + warp*2.2 + 5.0);
      vec3 night = mix(vec3(0.008, 0.002, 0.02), vec3(0.2, 0.008, 0.075), smoothstep(0.42, 0.85, n1));
      night = mix(night, vec3(0.55, 0.05, 0.22), 0.6*smoothstep(0.74, 0.98, 0.7*n1 + 0.3*n2));
      night *= 1.0 - 0.85*smoothstep(0.42, 0.72, n2);
      // fine soft veins in the gas, never a drawn line
      night += vec3(1.0, 0.16, 0.42)*pow(max(1.0 - 2.0*abs(n1 - 0.5), 0.0), 6.0)*0.22*(1.0 - 0.7*smoothstep(0.52, 0.78, n2));
      // the lip of the tear, lit like an ember where the world ends, and the heat against the blade
            night += vec3(1.0, 0.35, 0.75)*halo(d, 0.5)*0.4;
      res = vec4(mix(res.rgb, night, a), max(res.a, a));
    }
  }
  float fr = detail_jag(b, px)*0.4;
  float cov = (1.0 - smoothstep(-0.34 - px + fr, 0.1 + px + fr, d))*smoothstep(-0.25, 0.2, b.y);
  if (cov <= 0.0) return res;
  // the body itself: its picture (`assets/blade_body/malachor_rose.png`, made by `scripts/blade_body.py`), lengthwise along the blade, climbing it, a second slower layer so it never repeats
  float across = clamp((b.x - e.x)/max(e.y - e.x, 0.05), 0.02, 0.98);
  vec3 t1 = skinBody(vec2(b.y/13.0 - uClock*0.13, across));
  vec3 t2 = skinBody(vec2(b.y/21.0 - uClock*0.055 + 0.37, 0.1 + 0.8*across));
  const vec3 RK_MEAN = vec3(0.93, 0.71, 0.94);
  vec3 col = clamp(RK_MEAN + 1.3*(mix(t1, t2, 0.35) - RK_MEAN), vec3(0.0), vec3(1.0));
  col = mix(col, RK_MEAN, smoothstep(0.14, 0.6, px));
  // a thin torn white edge
  float tear = detail_jag(b, px);
  col = mix(col, RK_WHITE, (1.0 - smoothstep(0.02 + tear*0.3, 0.4 + tear, -d))*0.7);
  col = mix(col, RK_WHITE, 0.5*clamp(uClash, 0.0, 1.0));
  float keep = max(res.a, cov);
  return vec4(mix(res.rgb, col, cov/max(keep, 0.001)), keep);
}
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  // the mist: pink energy turning slowly round it from the hand to the point (with the aura's switch)
  // the torn space's breath: red and violet air turning round it
  float t;
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += RK_WHITE*halo(rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), t), 1.0)*cl*0.9;
  return fx*rxLife(0.4, 1.0, 0.3)*hiltFade(ro, rd);
}
// (update 29, Creyon: nothing past the tip) every effect ends at the blade's end
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_tc(ro, rd, px);
  float s;
  raySeg(ro, rd, vec3(0.0), vec3(0.0, uLength + 4.0, 0.0), s);
  return fx*tipFade(ro, rd);
}
vec3 sig_glow_d(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(1.000, 0.250, 0.620);
const vec3 SX_DEEP = vec3(0.500, 0.030, 0.250);
const vec3 SX_PALE = vec3(1.000, 0.700, 0.920);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34

vec4 sig_solid_l(vec4 o, vec2 b, float px){ return sig_solid_flat(o, b, px); }

// >>> update 42: this blade's great effects in the air
vec3 sig_vol_r24(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the dark red air round it
  if (part(0.0) > 0.5) a += 0.55*syMistSpin(ro, rd, 0.3, L, 1.6, 4.4, 0.014, 332.0, vec3(0.42, 0.015, 0.12), vec3(0.06, 0.004, 0.02));
  // storm behind them: dark red clouds lit from within in rose, and now and then a great star
  if (part(4.0) > 0.5){
    a += 1.25*gxThunderheads(ro, rd, px, 5.0, 5.0, 23.0, vec3(1.0, 0.45, 0.75), vec3(0.55, 0.02, 0.16));
    a += 1.1*gxFlares(ro, rd, px, 5.0, 5.2, 24.0, vec3(1.0, 0.86, 0.98), vec3(0.85, 0.2, 0.55));
  }
  // as in the pictures: long crescents of white light and crimson wheeling round it in the air, tapered at both ends
  if (part(2.0) > 0.5) for (int i = 0; i < 8; i++){
    float k = float(i), white = 1.0 - mod(k, 2.0);
    float run = C*(0.13 + 0.014*k) + h11(k*7.7), cyc = floor(run), q = fract(run);
    float a0 = h11(k*3.1 + cyc) + C*0.04, y0 = L*(-0.1 + 0.86*q), Rk = 2.4 + 1.5*h11(k*2.9 + cyc), span = L*(0.26 + 0.14*h11(k*6.1));
    float bWs = 0.0;
    float best = 1e3, bs = 0.0, s0;
    vec3 prev = around(Rk*0.72, a0, y0);
    for (int j = 1; j <= 8; j++){
      float f = float(j)/8.0;
      vec3 p = around(Rk*(0.72 + 0.5*sin(3.1416*f)), a0 + 0.24*f, y0 + span*f);
      float dd = raySegRaw(ro, rd, prev, p, s0);
      { float wN = bW(dd); bWs += wN; best = min(best, dd); bs += ((float(j) - 1.0 + s0)/8.0 - bs)*wN/bWs; }
      prev = p;
    }
    float taper = pow(sin(3.1416*clamp(bs, 0.0, 1.0)), 1.4), wd = max(0.03 + 0.16*taper, px);
    vec3 hot = mix(vec3(1.0, 0.12, 0.3), vec3(1.0, 0.94, 1.0), white), soft = mix(vec3(0.62, 0.01, 0.14), vec3(0.8, 0.4, 1.0), white);
    a += (hot*halo(best, wd)*1.05 + soft*halo(best, wd*3.2)*0.42)*taper*gxEnv(q);
  }
  if (part(3.0) > 0.5) a += 1.5*gxKin(ro, rd, 1.0, 7.0, 2.40, 0.120, 0.14, 0.08, 0.96, 1.0, 8131.0, vec3(1.0, 0.86, 1.0), vec3(0.8, 0.4, 1.0));
  float sg = gk_surge();
  if (sg > 0.0) a += sg*1.3*gxHelix(ro, rd, px, 18.0, 2.2, 2.4, 0.08, 25.0, vec3(1.0, 0.86, 0.98), vec3(0.85, 0.2, 0.55));
  a *= 1.0 + 0.8*sg;
  return base + a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<< update 42
// >>> dim: less glow, so its effects and body can be seen
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){ return sig_glow_d(g, b, r, px)*(0.82 + 0.4*sin(b.y*0.32 - uClock*2.1)); }
vec3 sig_core(vec3 c, vec2 b, float px){ return sig_core_d(c, b, px)*1.0; }
// <<< dim
// >>> life: waves of light running along its body
vec4 sig_solid(vec4 o, vec2 b, float px){
  vec4 r = sig_solid_l(o, b, px);
  if (r.a <= 0.0) return r;
  return vec4(r.rgb*(0.92 + 0.16*pow(0.5 + 0.5*sin(b.y*0.32 - uClock*2.1), 2.0)), r.a);
}
// <<< life
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }

// >>> how it answers: as it lights, in a swing, in a clash
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_r24(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float ig = uAge < 2000.0 ? 1.0 - smoothstep(0.3, 3.0, uAge) : 0.0, sw = clamp(uSwing, 0.0, 1.0);
  a += ig*1.4*gxHelix(ro, rd, px, 18.0, 2.2, 2.6, 0.16, 311.0, vec3(1.00, 0.88, 0.98), vec3(0.90, 0.20, 0.55));
  a += 1.45*gxBurst(ro, rd, px, 12.0, 5.0, 0.0, 0.34, 312.0, vec3(1.00, 0.80, 0.95), vec3(0.80, 0.08, 0.35));
  a += 1.2*gxBurst(ro, rd, px, 9.0, 3.6, 2.0, 0.0, 313.0, vec3(1.00, 0.92, 1.00), vec3(0.85, 0.25, 0.60));
  a += sw*1.1*gxComets(ro, rd, px, 3.0, 3.6, 0.08, 2.0, 314.0, vec3(1.00, 0.86, 0.98), vec3(0.85, 0.15, 0.50));
  return base + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

