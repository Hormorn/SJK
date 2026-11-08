// Tatooine Krayt Spine (new 2/11/2026, update 21, in place of Ziost Dark Anvil): the blade is the spine
// of a krayt dragon, bleached by Tatooine's twin suns and still alive. A chain of ivory vertebrae
// runs from the hilt to a long fang at the point, each with a curved thorn on either side, and the
// whole spine moves as a serpent does, a slow wave running up it. Between the bones a molten
// amber light beats. Sand pours off it in the wind, and krayt pearls drift round it, gleaming. Every
// 18 seconds the roar: the spine rears, its thorns spread wide and the fire between the bones blazes.
// Update 24 (Creyon: more effects, and not Vulptex's trail): two serpents of sand now wind up round the
// spine as the dragon's lost flesh, embers climb out of its joints, the air over it shimmers with heat,
// and at the roar a storm of sand and sparks bursts off every joint. Its trail is its own, sand and bone.
// @part Bone spine
// @part Thorns
// @part Pearl fire
// @part Blowing sand
// @part Twin suns
// @part The roar
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail bone
// @palette {"corona.rim_cool":[0.5,0.25,0.05],"corona.rim_hot":[1.0,0.6,0.2],"corona.inner":[1.0,0.85,0.6],"core.white":[1.0,0.95,0.85],"core.fringe_cool":[0.8,0.45,0.15],"core.fringe_hot":[1.0,0.75,0.4],"light.color":[1.0,0.62,0.25],"trail":[1.0,0.62,0.25]}
// @speed 1.0

const vec3 KR_BONE = vec3(0.86, 0.8, 0.64);
const vec3 KR_DARK = vec3(0.26, 0.19, 0.11);
const vec3 KR_AMBER = vec3(1.0, 0.58, 0.14);
const vec3 KR_HOT = vec3(1.0, 0.86, 0.55);
const vec3 KR_SAND = vec3(0.86, 0.68, 0.4);
const float KR_P = 3.1;

float kr_top(){ return (uLength + 0.3)*rxLife(0.0, 0.5, 0.5); }
float kr_after(){ return rxLife(0.3, 1.2, 0.5); }
float kr_roar(){ return part(5.0) > 0.5 ? rxCycle(18.0, 10.0, 12.2, 13.7, 16.7)*kr_after() : 0.0; }
// the serpent's wave: where the spine's middle is across the blade at y (only its picture)
float kr_mid(float y){
  float env = smoothstep(0.5, 7.0, y);
  return env*(0.34 + 0.25*kr_roar())*sin(y*0.24 - uClock*1.15) + env*0.08*sin(y*0.6 + uClock*0.8);
}
// x: how far the point lies inside the spine's bone (0 or more inside), y: across, z: along a bone
vec3 kr_bone(vec2 b, float top){
  float y = b.y;
  float dx = b.x - kr_mid(y);
  float x = abs(dx);
  float v = y/KR_P + 0.25;
  float n = floor(v);
  float f = v - n - 0.5;
  float af = abs(f);
  // the centrum: waisted, swelling toward each joint
  float e0 = 1.0 - clamp((top - y)/1.6, 0.0, 1.0);
  float taper = sqrt(max(1.0 - e0*e0, 0.0));
  float w = (0.42 + 0.26*smoothstep(0.1, 0.5, af))*(0.3 + 0.7*taper);
  float inside = w - x;
  // the thorn: from the middle of each bone, curving back toward the hand, sharp at its end
  if (part(1.0) > 0.5){
    float reach = (1.55 + 0.45*h11(n*3.7 + (dx < 0.0 ? 1.0 : 0.0)) + 0.8*kr_roar())*(0.25 + 0.75*taper)*smoothstep(0.6, 3.0, y);
    float s = clamp(x/max(reach, 0.05), 0.0, 1.0);
    float centre = 0.05 + 0.34*s*s;
    float thick = 0.24*(1.0 - s)*(1.0 - s*0.25);
    if (x < reach) inside = max(inside, thick - abs(f - centre)*KR_P);
  }
  // the long fang at the point
  return vec3(inside, dx/max(w, 0.01), f);
}
vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = kr_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    vec3 k = kr_bone(b, top);
    float wg = 0.42 + 0.26*smoothstep(0.1, 0.5, abs(k.z));
    float out_ = max(abs(b.x - kr_mid(b.y)) - wg, 0.0);
    float rr = kr_roar();
    // a little heat round the bone, and light spilling from the joints
    float joint = smoothstep(0.32, 0.5, abs(k.z));
    col = KR_AMBER*(halo(out_, 0.14)*0.28 + halo(out_, 0.7)*(0.05 + 0.2*rr))*(0.5 + joint);
  }
  col += lvl(g)*KR_AMBER*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.3;
  col *= tipFadeB(b)*step(-0.8, b.y);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = kr_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.2) return o;
  vec3 k = kr_bone(b, top);
  float cov = smoothstep(-0.06 - px, 0.07 + px, k.x)*smoothstep(-0.2, 0.05, b.y);
  if (cov <= 0.0) return o;
  float across = clamp(abs(k.y), 0.0, 1.0);
  float v = abs(k.z);
  // round and pitted: lit from one side, darker toward its edges and into its joints
  float nz = sqrt(max(1.0 - across*across, 0.0));
  float lit = 0.3 + 0.75*max(dot(normalize(vec3(k.y, 0.0, nz + 0.2)), normalize(vec3(-0.5, 0.0, 0.8))), 0.0);
  float pits = rxFlow(vec2(b.x*5.0 + 2.0, b.y*2.2));
  vec3 bone = mix(KR_DARK, KR_BONE, smoothstep(0.1, 0.8, lit)*(0.82 + 0.3*pits));
  bone *= 1.0 - 0.45*smoothstep(0.16, 0.5, v);
  // the fire between the bones, beating up the spine
  float beat = 0.55 + 0.45*sin(b.y*0.42 - uClock*2.4);
  float fire = part(2.0) > 0.5 ? smoothstep(0.2, 0.5, v)*smoothstep(0.2, 0.5, v)*(1.0 - smoothstep(0.55, 1.0, across)) : 0.0;
  vec3 col = mix(bone, mix(KR_AMBER, KR_HOT, beat), fire*(0.75 + 0.25*beat));
  // seen through hairline cracks too
  float crack = (1.0 - smoothstep(0.0, 0.09, abs(rxFlow(vec2(b.x*3.0 + 3.0, b.y*0.6)) - 0.5)))*0.5;
  col += KR_AMBER*crack*(0.2 + 0.6*beat + 1.0*kr_roar())*step(0.0, part(2.0) - 0.5);
  col += KR_HOT*fire*kr_roar()*0.8;
  col = mix(col, KR_HOT, 0.45*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec4 sig_vol_solid_34(vec3 ro, vec3 rd, float px){ return vec4(0.0); }
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t;
  float aft = kr_after();
  float rr = kr_roar();
  // sand pouring off it in long veils that blow back along it, with grains in them
  if (part(3.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 7.0){
    fx += (0.3 + 0.5*rr)*shStreaks(ro, rd, 2.3, 0.6, L - 0.5, 9.0, 3.2, 3.4, 0.0, 77.0, KR_SAND*0.8, KR_SAND*0.3)*aft;
    fx += shFlakes(ro, rd, 2.6, 0.5, L - 0.5, 34.0, 2.4, 1.6, 0.03, 0.05, 0.5, 0.45, 79.0, KR_SAND, KR_HOT*0.8)*aft;
  }
  // two serpents of sand winding up round the spine: thick grains in long moving coils
  if (part(3.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 5.0){
    for (int i = 0; i < 44; i++){
      float k = float(i);
      float side = mod(k, 2.0);
      float f = fract(k/44.0*2.0 + uClock*0.05);
      float wave = sin(f*18.0 - uClock*0.8 + side*3.14);
      vec3 p = around(1.5 + 0.35*wave + 1.1*rr, side*0.5 + f*2.6 + uClock*0.02, L*(0.03 + 0.92*f));
      float d = rayPt(ro, rd, p, t);
      float big = 0.05 + 0.05*h11(k*3.1);
      fx += (KR_SAND*halo(d, max(big, px))*1.25 + KR_AMBER*halo(d, big*4.0)*0.14)*sin(3.1416*f)*(0.8 + 0.6*rr)*aft;
    }
  }
  // embers climbing out of the joints, one bone at a time
  if (part(2.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 5.5){
    for (int i = 0; i < 16; i++){
      float k = float(i);
      float cyc = uClock*(0.26 + 0.1*h11(k*2.3)) + h11(k*6.7);
      float n = floor(cyc), q = fract(cyc);
      float joint = floor(h11(n*3.1 + k)*11.0);
      float y0 = joint*KR_P + KR_P*0.25;
      if (y0 > L - 2.0) continue;
      vec3 p = around(0.5 + 1.5*q + 2.5*rr*q, h11(n*5.1 + k) + 0.2*q, y0 + 3.0*q + kr_mid(y0)*0.0);
      float d = rayPt(ro, rd, p, t);
      fx += (KR_HOT*halo(d, max(0.04, px))*1.5 + KR_AMBER*halo(d, 0.2)*0.4)*sin(3.1416*q)*(1.0 + 1.2*rr)*aft;
    }
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += (KR_HOT*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 0.9)*1.2 + KR_AMBER*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 2.6)*0.3)*cl;
  return fx*uFx*hiltFade(ro, rd);
}
// (update 26) the body starts softly in the emitter, no hard end above the hilt
// Now and then a layer of light climbs slowly over the bones from the hand to the point, a soft wide front, and the spine is a true blade for a while; then it sinks back the same way.
float kr_true(){ return rxCycle(46.0, 16.0, 26.0, 34.0, 44.0); }
vec4 sig_solid(vec4 o, vec2 b, float px){
  vec4 r = sig_solid_raw(o, b, px);
  float j = smoothstep(-0.25, 0.3, b.y);
  r = vec4(mix(o.rgb, r.rgb, j), mix(o.a, r.a, j));
  float t0 = kr_true();
  if (t0 > 0.0 && b.y > -0.2){
    // the layer lies on the spine itself and moves with it; the thorns melt away into the light as it passes
    float x0 = b.x - kr_mid(b.y);
    float m = smoothstep(0.0, 1.0, (t0*(uLength + 20.0) - 10.0 - b.y)/10.0), e0 = clamp((uLength - b.y)/1.6, 0.0, 1.0);
    float half_ = 0.72*sqrt(max(1.0 - (1.0 - e0)*(1.0 - e0), 0.0)), u = abs(x0)/max(half_, 0.01);
    float cov = (1.0 - smoothstep(0.7, 1.0 + px, u))*smoothstep(-0.2, 0.4, b.y)*m;
    r.a *= 1.0 - m*smoothstep(0.9, 1.3, u);
    vec3 c0 = mix(vec3(1.0, 0.97, 0.88), vec3(1.0, 0.72, 0.28), smoothstep(0.05, 1.0, u));
    r = vec4(mix(r.rgb, c0, cov), max(r.a, cov));
  }
  return r;
}
// (update 30, Creyon: nothing changed, something added) the sand dragon: a long stream of golden sand as thick
// as a body winding up round the spine, turning slowly, dust falling from it; round the whole blade the
// haze of the twin suns; and two small suns, one white and one amber, wheeling slowly round it
vec3 kr_serpent(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength, t;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.4, 1.4, 0.5);
  if (part(3.0) > 0.5){
    fx += 1.0*xPuffs(ro, rd, 1.5, 2.3, 2.1, 0.022*uClock, L*0.05, L*0.97, 0.75, 71.0, KR_HOT, KR_AMBER*0.6);
    fx += 1.5*xDust(ro, rd, 1.55, 2.5, 2.1, 0.022*uClock, L*0.05, L*0.97, 0.5, 72.0, KR_HOT, KR_AMBER);
    fx += 1.1*xDust(ro, rd, 2.3, 3.3, 2.1, 0.022*uClock + 0.05, L*0.05, L*0.92, 0.9, 73.0, KR_AMBER, KR_AMBER*0.4);
    fx += 0.75*syMistSpin(ro, rd, L*0.05, L*0.98, 1.1, 3.7, 0.025, 74.0, KR_AMBER*0.5, vec3(0.3, 0.14, 0.05)*0.5);
  }
  if (part(4.0) > 0.5){
    for (int i = 0; i < 2; i++){
      float k = float(i);
      vec3 p = around(3.0 - 0.5*k, 0.5*k + uClock*0.035*(1.0 - 0.3*k), L*(0.42 + 0.3*k + 0.1*sin(uClock*0.17 + k*2.0)));
      vec3 c = k < 0.5 ? vec3(1.0, 0.95, 0.82) : vec3(1.0, 0.6, 0.22);
      float d = rayPt(ro, rd, p, t);
      fx += c*(halo(d, 0.2)*1.8 + halo(d, 0.7)*0.4 + halo(d, 2.2)*0.09);
    }
  }
  return fx*aft*uFx*hiltFade(ro, rd);
}
// (update 29, Creyon: nothing past the tip) every effect ends at the blade's end
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_tc(ro, rd, px) + kr_serpent(ro, rd, px);
  float s;
  raySeg(ro, rd, vec3(0.0), vec3(0.0, uLength + 4.0, 0.0), s);
  return fx*tipFade(ro, rd);
}
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(1.000, 0.600, 0.200);
const vec3 SX_DEEP = vec3(0.500, 0.250, 0.050);
const vec3 SX_PALE = vec3(1.000, 0.850, 0.600);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += 1.1*xDust(ro, rd, 3.0, 1.0, 1.2, C*0.03, L*0.08, L*0.96, 0.42, 81.0, vec3(1.0, 0.86, 0.6), vec3(0.85, 0.55, 0.22)); a += 0.7*syMistSpin(ro, rd, L*0.03, L*0.6, 1.2, 3.8, 0.02, 82.0, vec3(0.42, 0.28, 0.12), vec3(0.12, 0.07, 0.03));
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = sig_vol_solid_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  float L = uLength, C = uClock;
  vec4 n = (vec4(0.0))*rxLife(0.45, 1.3, 0.4)*uFx*hiltFade(ro, rd);
  return vec4(mix(o.rgb, n.rgb, n.a), max(o.a, n.a));
}
// <<< update 34

// >>> update 52: this blade's great effects in the air
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // warm desert air lying quietly round the spine
  a += 0.8*syMistSpin(ro, rd, 0.3, L, 1.4, 4.2, 0.012, 550.0, vec3(0.5, 0.33, 0.14), vec3(0.1, 0.06, 0.02));
  a += 1.7*xDust(ro, rd, 1.5, 3.6, 2.0, C*0.2, L*0.02, L*0.97, 1.3, 552.0, vec3(1.0, 0.86, 0.55), vec3(0.72, 0.42, 0.14));
  a += 1.35*xDust(ro, rd, 3.6, 1.8, -1.7, C*0.16 + 0.5, L*0.02, L*0.97, 1.0, 553.0, vec3(0.95, 0.74, 0.42), vec3(0.5, 0.28, 0.08));
  a += 1.05*xPuffs(ro, rd, 2.4, 4.4, 1.6, C*0.12, L*0.02, L*0.8, 1.0, 554.0, vec3(0.5, 0.33, 0.14), vec3(0.14, 0.08, 0.03));
  return base + a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<< update 52
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(5.0) > 0.5){
    a += 1.4*gxKin(ro, rd, 0.0, 8.0, 3.00, 0.100, 0.25, 0.08, 0.96, 1.0, 8079.0, vec3(1.000, 0.850, 0.600), vec3(1.000, 0.600, 0.200));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  if (part(5.0) > 0.5){
    col += vec3(0.0);
  }
  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol_r24(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> how it answers: as it lights, in a swing, in a clash
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_r24(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float ig = uAge < 2000.0 ? 1.0 - smoothstep(0.3, 3.0, uAge) : 0.0, sw = clamp(uSwing, 0.0, 1.0);
  a += ig*1.3*gxGale(ro, rd, px, 14.0, 5.2, 308.0, vec3(1.00, 0.86, 0.55), vec3(0.85, 0.50, 0.16));
  a += 1.4*gxBurst(ro, rd, px, 11.0, 4.6, 0.0, 0.34, 309.0, vec3(1.00, 0.93, 0.76), vec3(0.70, 0.45, 0.20));
  a += sw*0.8*gxGale(ro, rd, px, 8.0, 4.0, 310.0, vec3(1.00, 0.86, 0.55), vec3(0.85, 0.50, 0.16));
  return base + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

