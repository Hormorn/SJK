// Nihilus Hunger (Mythical, rebuilt on Sol's glow and core 3/11/2026): Darth Nihilus, a wound in the Force that eats worlds: a hollow black blade with a thin red edge and red veins draining toward the hand. Long soft streams of stolen life bend in from all round it and sink into the blade, dark fire rises off it and a slow red mist is pulled in. When it feeds, everything is drawn in faster.
// @part Hollow blade
// @part Draining veins
// @part Stolen life
// @part Dark flame
// @part Drawn mist
// @part The hunger
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail hunger
// @palette {"corona.rim_cool":[0.3,0.0,0.01],"corona.rim_hot":[0.9,0.03,0.03],"corona.inner":[0.8,0.05,0.05],"core.white":[0.4,0.02,0.02],"core.fringe_cool":[0.45,0.0,0.02],"core.fringe_hot":[0.95,0.08,0.05],"light.color":[0.9,0.03,0.03],"trail":[0.9,0.04,0.04]}
// @speed 1.0

const vec3 Rd = vec3(0.950, 0.040, 0.030);
const vec3 Dp = vec3(0.300, 0.004, 0.012);
const vec3 Bk = vec3(0.008, 0.004, 0.006);
const vec3 Pl = vec3(1.000, 0.700, 0.620);
float ni_top(){ return uLength*rxLife(0.0, 0.5, 0.32); }
float ni_feed(){ return part(5.0) > 0.5 ? rxCycle(16.0, 8.3, 10.5, 12.5, 15.5) : 0.0; }
float ni_half(float y){
  float top = ni_top();
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/2.2, 0.0, 1.0);
  return (0.5 + 0.03*sin(y*0.5 + uClock*1.6))*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  float top = ni_top(), yc = clamp(b.y, 0.0, top), fd = ni_feed();
  float d = max(length(vec2(b.x, b.y - yc)) - ni_half(yc), 0.0);
  float w = 0.5 + 0.5*sin(b.y*0.36 + uClock*2.2);
  vec3 col = g*(0.28 + 0.22*w + 0.5*fd + 0.7*clamp(uClash, 0.0, 1.0));
  col += Rd*(halo(d, 0.12)*(0.7 + 0.3*w)) + Dp*(halo(d, 0.7)*(0.42 + 0.5*fd) + halo(d, 2.8)*(0.1 + 0.2*fd));
  return col*tipFadeB(b)*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid_e2(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  // its body breathes and is eaten: the edge wanders as if something were drawing on it from within
  float bite = rxFlow(vec2(b.y*0.21 - uClock*0.32, b.x > 0.0 ? 2.0 : 7.0)) - 0.5;
  float half = ni_half(b.y)*(1.0 + 0.2*bite + 0.06*sin(b.y*0.5 + uClock*0.9));
  float cov = (1.0 - smoothstep(half - 0.14 - px, half + 0.07 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  float fd = ni_feed();
  // a round black mass: light only reaches its sides, and deep inside something red moves, slower than its surface
  float deep = rxFlow(vec2(asin(u)*0.9 + 3.0, b.y*0.09 + uClock*0.17));
  vec3 col = Bk + Dp*(0.35 + 0.9*deep*deep)*nz;
  if (part(1.0) > 0.5){
    // what it feeds on, running inward and up: veins that thicken and thin, pulled toward the hand, wrapped on the roundness
    vec2 w0 = vec2(asin(u)*1.5, b.y*0.14 + uClock*0.42);
    w0 += 0.35*vec2(rxFlow(w0*0.8 + 4.0) - 0.5, rxFlow(w0*0.9 + 8.0) - 0.5);
    float v = rxFlow(w0), v2 = rxFlow(w0*1.9 + 5.0);
    float thick = 0.03 + 0.06*rxFlow(vec2(b.y*0.1 - uClock*0.25, 2.0));
    float vein = max(1.0 - smoothstep(0.0, thick, abs(v - 0.5)), 0.6*(1.0 - smoothstep(0.0, thick*0.7, abs(v2 - 0.5))));
    float wave = 0.3 + 0.7*pow(0.5 + 0.5*sin(b.y*0.36 + uClock*2.2), 2.0);
    col += mix(Dp*2.0, Rd, wave)*vein*(0.5 + 0.6*nz)*(1.1 + fd);
  }
  if (part(0.0) > 0.5) col += Rd*pow(1.0 - nz, 3.0)*0.75*(0.7 + 0.5*deep);
  col = mix(col, Rd, 0.5*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
// A black splinter of a dead world: long and sharp, one face catching a dull red light.
vec4 ni_splinter(vec3 ro, vec3 rd, float px, vec3 c, float R, float k){
  float tm, d = rayPtRaw(ro, rd, c, tm);
  if (d > R*1.7) return vec4(0.0);
  vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1), w0 = ro + rd*tm - c;
  vec2 u = rot(uClock*0.07*(0.5 + h11(k)) + k*1.9)*vec2(dot(w0, e1), dot(w0, e2))/R;
  float kite = abs(u.x)*2.6 + abs(u.y)*0.72;
  float cov = (1.0 - smoothstep(0.94 - px/R, 1.0 + px/R, kite))*(1.0 - behindBlade(ro, rd, tm));
  vec3 col = mix(Bk, Dp*0.8, u.x > 0.0 ? 0.65 : 0.1) + Rd*smoothstep(0.78, 1.0, kite)*0.4;
  return vec4(col, cov);
}
// Pieces of dead worlds: black shards hanging in the air, each slowly falling inward and being carried round it.
// The world it is eating: a dead planet hanging beside it, cracked, its life streaming off it toward the blade in a line of dying sparks.
// The world's life: whole, then it bursts, and slowly gathers again from nothing.
float ni_year(){ return fract(uClock/34.0); }
float ni_whole(){ float y = ni_year(); return smoothstep(0.9, 0.995, y) + (1.0 - smoothstep(0.6, 0.64, y))*step(y, 0.84); }
vec3 ni_world(){ return around(4.0 + 0.9*sin(uClock*0.13), 0.62 + uClock*0.022, uLength*(0.56 + 0.26*sin(uClock*0.09 + 1.0))); }
vec3 ni_shard(float k, out float R){
  float L = uLength, h1 = h11(k*3.1), h2 = h11(k*7.7), h3 = h11(k*5.3);
  R = 0.42 + 0.7*h3;
  return around(2.2 + 1.9*h1 + 0.5*sin(uClock*0.13 + k), h2 + uClock*(0.012 + 0.012*h3), L*(0.08 + 0.84*fract(k*0.618)) + 0.7*sin(uClock*0.15 + k*2.3));
}
vec3 sig_vol_e3(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.3, 1.1, 0.35);
  if (aft <= 0.01) return fx;
  float fd = ni_feed();
  if (part(2.0) > 0.5){
    // what it has taken, sinking toward the hand as sparks of red
    fx += (1.4 + fd)*xDust(ro, rd, 3.2, 1.4, 0.6, -C*0.04, L*0.04, L, 0.9, 305.0, Rd + Pl*0.3, Dp*1.6);
    fx += 1.5*vec3(0.0);
  }
  // the glow of the shards turning round it
  if (part(3.0) > 0.5){
    float t;
    for (int i = 0; i < 9; i++){
      float k = float(i), R;
      vec3 c = ni_shard(k, R);
      fx += Rd*0.5*halo(rayPt(ro, rd, c, t), R*2.3)*0.5*(0.6 + 0.4*sin(C*0.5 + k*2.0));
    }
  }
  if (part(4.0) > 0.5) fx += (0.9 + 0.5*fd)*syMistSpin(ro, rd, 0.3, L, 1.2, 5.2, -0.022, 303.0, Dp*1.1, Bk*5.0);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec4 sig_vol_solid_e3(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx <= 0.0 || part(3.0) < 0.5 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  for (int i = 0; i < 9; i++){
    float k = float(i), R;
    vec3 c = ni_shard(k, R);
    vec4 q = ni_splinter(ro, rd, px, c, R, k);
    if (q.a > o.a) o = q;
  }
  o.a *= uFx*rxLife(0.3, 1.3, 0.35)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return o;
}
vec4 sig_solid(vec4 o, vec2 b, float px){
  // the hunger itself: a black cloud behind it being drawn in toward the blade, red veins of what it feeds on running inward through it
  if (part(4.0) > 0.5 && uFx > 0.0 && b.y > 0.0 && b.y < uLength){
    float d = abs(b.x), side = b.x > 0.0 ? 2.0 : 6.0;
    float reach = (2.8 + 2.2*rxFlow(vec2(b.y*0.1 + uClock*0.04, side)))*smoothstep(0.0, uLength*0.14, b.y)*(1.0 - smoothstep(uLength*0.86, uLength*1.02, b.y));
    float a = (1.0 - smoothstep(reach*0.25, reach, d))*0.9*rxLife(0.4, 1.4, 0.3);
    if (a > 0.003){
      vec2 q = vec2(d*0.62 + uClock*0.11 + side, b.y*0.17 + 0.03*sin(uClock*0.2));
      vec2 w0 = vec2(rxFlow(q*1.2 + 3.0), rxFlow(q*1.4 + 7.0)) - 0.5;
      float n = rxFlow(q + w0*1.5);
      vec3 dark = mix(vec3(0.006, 0.002, 0.004), Dp*0.5, smoothstep(0.42, 0.8, n));
      dark += Rd*(1.0 - smoothstep(0.0, 0.1, abs(n - 0.5)))*0.2*(0.6 + 0.6*ni_feed());
      o = vec4(mix(o.rgb, dark, a), max(o.a, a));
    }
  }
  return sig_solid_e2(o, b, px);
}
vec3 sig_vol_r24(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e3(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(5.0) > 0.5){
    vec3 w0 = ni_world();
    float t, dm = rayPtRaw(ro, rd, w0, t);
    a += (Rd*0.42*halo(dm, 2.3) + Dp*halo(dm, 4.0)*0.5)*(0.7 + 0.3*sin(C*0.5))*(1.0 - behindBlade(ro, rd, t));
    // its life torn away: sparks leaving it one after another and falling into the blade
    for (int i = 0; i < 14; i++){
      float k = float(i), q = fract(C*0.07 + k/14.0);
      vec3 to = vec3(0.0, w0.y*(0.55 + 0.04*sin(k*2.0)), 0.0);
      float side = 0.7*sin(k*1.7 + C*0.2)*(1.0 - q)*q*4.0;
      vec3 pos = mix(w0, to, q*q) + vec3(side*0.5, side, -side*0.4);
      float dp = rayPt(ro, rd, pos, t);
      a += (Pl*halo(dp, 0.05)*1.5 + Rd*halo(dp, 0.2)*0.85)*smoothstep(0.0, 0.08, q)*(1.0 - smoothstep(0.86, 1.0, q))*ni_whole();
    }
    // it bursts: a slow bloom of red light, its pieces fly out and race round the blade until they fade
    float yr = ni_year(), burst = smoothstep(0.58, 0.64, yr)*(1.0 - smoothstep(0.66, 0.8, yr));
    a += (Pl*halo(dm, 1.0)*0.8 + Rd*halo(dm, 2.2)*0.5)*burst;
    // out from the ball, round the blade, back into it
    float qb = (yr - 0.6)/0.4;
    if (qb > 0.0 && qb < 1.0){
      float leave = 1.0 - pow(1.0 - clamp(qb*4.5, 0.0, 1.0), 3.0), race = smoothstep(0.08, 0.3, qb)*(1.0 - smoothstep(0.62, 0.86, qb)), life = smoothstep(0.0, 0.03, qb)*(1.0 - smoothstep(0.97, 1.0, qb));
      for (int i = 0; i < 14; i++){
        float k = float(i), h0 = h11(k*3.1 + 7.0), h1 = h11(k*5.3 + 2.0), h2 = h11(k*8.7 + 4.0), sg = mod(k, 2.0) < 0.5 ? 1.0 : -1.0;
        vec3 dir = normalize(vec3(h0 - 0.5, h1 - 0.5, h2 - 0.5) + 0.001);
        vec3 near_ = w0 + dir*(1.0 + 2.6*h0)*leave*(1.0 - smoothstep(0.8, 1.0, qb));
        // each its own path: quick round the blade, climbing and sinking at its own beat
        vec3 path = around(2.4 + 2.4*h1 + 0.8*sin(C*0.6 + k*2.0), h2 + sg*C*(0.42 + 0.28*h0), L*(0.5 + 0.38*sin(C*(0.35 + 0.3*h1) + k*2.4)));
        a += 1.8*gxPiece(ro, rd, px, mix(near_, path, race), (0.3 + 0.24*h0)*(1.0 - 0.6*smoothstep(0.86, 1.0, qb)), k + 40.0, 0.0, Pl, Rd)*life;
      }
    }
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = sig_vol_solid_e3(ro, rd, px);
  if (uFx > 0.0 && part(5.0) > 0.5 && fxNear(ro, rd, 0.0, uLength) < 6.6){
    float tm, R = 1.7*rxLife(0.6, 2.2, 0.35)*ni_whole();
    vec3 c = ni_world();
    float d = rayPtRaw(ro, rd, c, tm);
    if (R > 0.05 && d < R + 0.1){
      vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1), w0 = ro + rd*tm - c;
      vec2 u = vec2(dot(w0, e1), dot(w0, e2))/R;
      float r0 = length(u), nz = sqrt(max(1.0 - r0*r0, 0.0));
      vec2 p = u/(0.55 + 0.45*nz) + vec2(uClock*0.004, 0.0);
      float v = rxFlow(p*2.1 + 4.0), v2 = rxFlow(p*4.4 + 9.0);
      float crack = max(1.0 - smoothstep(0.0, 0.03, abs(v - 0.5)), 0.7*(1.0 - smoothstep(0.0, 0.022, abs(v2 - 0.5))));
      float lit = 0.22 + 0.65*clamp(dot(vec3(u, nz), normalize(vec3(-0.6, 0.5, 0.6))), 0.0, 1.0);
      vec3 col = mix(vec3(0.02, 0.012, 0.014), vec3(0.15, 0.09, 0.09), smoothstep(0.3, 0.7, vn(p*3.0)))*(0.5 + 1.6*lit) + Rd*crack*(0.75 + 0.5*ni_feed()) + Rd*smoothstep(0.55, 0.95, r0)*0.25;
      float cov = (1.0 - smoothstep(0.86 - px/R, 1.0 + px/R, r0))*(1.0 - behindBlade(ro, rd, tm));
      cov *= uFx*tipFade(ro, rd)*hiltFade(ro, rd);
      if (cov > o.a) o = vec4(col, cov);
    }
  }
  return o;
}

// >>> how it answers: as it lights, in a swing, in a clash
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_r24(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float ig = uAge < 2000.0 ? 1.0 - smoothstep(0.3, 3.0, uAge) : 0.0, sw = clamp(uSwing, 0.0, 1.0);
  a += ig*1.35*gxVortex(ro, rd, px, 14.0, 6.0, 305.0, Pl, Rd);
  a += 1.45*gxBurst(ro, rd, px, 11.0, 4.8, 0.0, 0.36, 306.0, Pl, Rd);
  a += sw*1.1*gxVortex(ro, rd, px, 10.0, 4.6, 307.0, Pl, Rd);
  return base + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

