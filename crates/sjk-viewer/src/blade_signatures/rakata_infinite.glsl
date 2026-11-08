// Rakata Infinite: the calm blue power of a god, as the ancient Rakata dreamed it. Rebuilt 1/11/2026
// to Creyon's bar (the space round it filled, as Kitsune Fire's tails and Abyssal Eye's eye fill
// theirs). Its own body is a blade of deep sapphire ki with a white-hot heart and a slow grain of
// light rising through it. Round it stands a towering god aura in real 3D: a crown of great flames
// of god ki rising from all along the blade, each a broad tongue rooted in the blade, leaning out
// and up and curling back in at its tip, white at its heart, sky blue through its body and deep
// blue at its edges, swaying slowly, the whole crown turning slowly round the blade. Inside it a
// calm sheath of pale light hugs the blade, round sparkles of god ki rise through the flames and
// fade, and now and then a thread of blue lightning crawls up through the aura. Every 30 seconds
// it is pushed past its limit: from the emitter up the flames turn crimson round the calm blue
// blade, grow taller, the blade's heart blazes, the sparkles turn red-gold, and slowly it all
// cools back to blue. Lit, the flames burst up out of the blade from the emitter to the point one
// after another; let go, they fold back into the blade and it fades in a scatter of sparkles. A
// clash is a blue-white flash. Its trail is god ki (`@trail infinite`). No figures; nothing over the
// point.
// @part God ki body
// @part God aura
// @part Star streams
// @part Lightning threads
// @part Limit surge
// @part Star Forge suns
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail infinite
// @palette {"corona.rim_cool":[0.0,0.25,0.85],"corona.rim_hot":[0.2,0.7,1.0],"corona.inner":[0.75,0.95,1.0],"core.white":[0.95,1.0,1.0],"core.fringe_cool":[0.1,0.45,1.0],"core.fringe_hot":[0.55,0.85,1.0],"light.color":[0.2,0.6,1.0],"trail":[0.2,0.65,1.0]}
// @speed 1.5

const vec3 GK_SAPPHIRE = vec3(0.08, 0.38, 1.0);
const vec3 GK_SKY = vec3(0.45, 0.85, 1.0);
const vec3 GK_DEEP = vec3(0.02, 0.12, 0.6);
const vec3 GK_RED = vec3(1.0, 0.1, 0.06);
const vec3 GK_CRIMSON = vec3(0.6, 0.0, 0.05);
const vec3 GK_GOLD = vec3(1.0, 0.75, 0.35);
const vec3 GK_WHITE = vec3(0.93, 0.98, 1.0);
// how many flames stand in the crown
const float GK_N = 12.0;

float gk_top(){ return (uLength + 0.4)*rxLife(0.0, 0.3, 0.6); }
float gk_after(){ return rxLife(0.2, 1.4, 0.8); }
// the limit surge on its 30-second cycle, 0 to 1, slow both ways
// (update 26: on the clock alone, no seed, so its trail can turn with it, `saber_trail.rs`)
float gk_surge(){
  if (part(4.0) < 0.5) return 0.0;
  float t = mod(uClock, 30.0);
  return smoothstep(18.0, 21.0, t)*(1.0 - smoothstep(25.0, 29.0, t))*gk_after();
}
// how far up the blade the red has climbed (units)
float gk_surge_y(){ return (uLength + 8.0)*smoothstep(18.0, 21.0, mod(uClock, 30.0)) - 3.0; }
// a slow, smooth sway, 0 to 1
float gk_flow(vec2 p){ return rxFlow(p); }

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = gk_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half = rxHalf(clamp(b.y, 0.0, top), top, 0.56, 1.0);
    float d = rxOut(b, top, half);
    float after = gk_after();
    float sg = gk_surge();
    float red = sg*(1.0 - smoothstep(gk_surge_y() - 3.0, gk_surge_y(), b.y));
    col = mix(GK_SAPPHIRE, GK_RED, red)*halo(d, 0.6)*0.9 + mix(GK_DEEP, GK_CRIMSON, red)*halo(d, 1.8)*0.6 + GK_WHITE*halo(d, 0.15)*0.5;
    // effect 2 (with the aura): the calm inner sheath of pale light hugging the blade, and the
    // aura's wide glow round the whole crown, crimson where the surge has climbed
    if (part(1.0) > 0.5){
      col += mix(GK_SKY, GK_RED*0.8, red)*halo(d, 0.6)*0.4*after;
      col += mix(GK_SAPPHIRE, GK_RED, red)*(halo(d, 3.0)*0.16 + halo(d, 1.1)*0.3)*after*smoothstep(-1.0, 2.0, b.y);
    }
  }
  col += lvl(g)*mix(GK_SKY, GK_RED, gk_surge()*smoothstep(-3.0, 0.0, gk_surge_y()))*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.6;
  col *= tipFadeB(b);
  vec3 lit0 = col*roomFade(abs(b.x))*(1.0 + 0.9*clamp(uClash, 0.0, 1.0));
  // (update 28) the surge's red stays red under a chroma, so its transition shows
  float red0 = gk_surge()*(1.0 - smoothstep(gk_surge_y() - 3.0, gk_surge_y(), b.y));
  uTurnFx += lit0*red0;
  return lit0*(1.0 - red0);
}
vec3 sig_core_d(vec3 c, vec2 b, float px){
  return c*vec3(0.6, 0.85, 1.0)*0.3*(1.0 - smoothstep(0.3, 2.0, b.y));
}

// effect 1: the god-ki body, deep sapphire with a white-hot heart, a slow grain of light rising
// through it; its heart blazing in the surge
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  // (update 28) red through the surge whatever colour it is worn in
  
  float top = gk_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.3) return o;
  float half = rxHalf(clamp(b.y, 0.0, top), top, 0.56, 1.0);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float sg = gk_surge();
  vec3 col = mix(GK_WHITE*1.1, GK_SAPPHIRE*1.1, smoothstep(0.25, 0.85, u));
  col = mix(col, GK_DEEP*1.6, smoothstep(0.88, 1.0, u)*0.6);
  if (part(0.0) > 0.5){
    float grain = gk_flow(vec2(b.x*7.0, b.y*1.2 - tCalm()*1.6));
    float streak = pow(gk_flow(vec2(b.x*11.0 + 3.0, b.y*0.5 - tCalm()*1.0)), 6.0);
    col *= 0.88 + 0.25*grain;
    col += GK_WHITE*streak*1.4*(1.0 - u*0.5);
    // (update 18, Creyon: "I see almost nothing of the god ki body") broad currents of sky-blue ki
    // winding up through it, crossing as they climb, bright enough to read from a distance
    float wind = 0.5 + 0.5*sin(b.x*9.0 + b.y*0.75 - tCalm()*2.6);
    float wind2 = 0.5 + 0.5*sin(-b.x*11.0 + b.y*0.55 - tCalm()*1.9 + 2.0);
    col += GK_SKY*(pow(wind, 5.0) + 0.7*pow(wind2, 6.0))*0.95;
    col += GK_WHITE*pow(wind*wind2, 6.0)*1.2;
  }
  float redB = sg*(1.0 - smoothstep(gk_surge_y() - 3.0, gk_surge_y(), b.y));
  col = mix(col, mix(vec3(1.0, 0.75, 0.75), GK_RED*1.2, smoothstep(0.3, 0.9, u)), redB*0.7);
  col += GK_WHITE*sg*(1.0 - u)*0.6;
  col += GK_WHITE*halo(max(b.y, 0.0), 0.9)*0.8;
  col = mix(col, GK_WHITE, 0.55*clamp(uClash, 0.0, 1.0));
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// effect 2: the god aura (rebuilt 1/11/2026, Creyon: the crown of flames "looks like a tree"):
// a great egg of blue god-ki flame wrapped round the whole blade in three dimensions, as a
// god-ki fighter's aura is drawn: a glowing outline of streaming flame narrow at the hand,
// widest two thirds up and licking into a crown at the top, a softer glow filling it, and long
// streaks of ki rushing up through it. In the limit surge it all burns crimson from the hand up.
// As the blade lights it swells out of the blade; let go, it sinks back in.
float gk_red_at(float y, float sg){ return sg*(1.0 - smoothstep(gk_surge_y() - 3.0, gk_surge_y(), y)); }
vec3 gk_crown(vec3 ro, vec3 rd, float px, float sg, float top){
  vec3 acc = vec3(0.0);
  float L = uLength;
  if (fxNear(ro, rd, 0.0, L) > 16.0) return acc;
  float grow = uAge >= 2000.0 ? 1.0 - smoothstep(0.0, 0.7, uAge - 2000.0) : smoothstep(0.2, 1.4, uAge);
  float red = sg*smoothstep(-3.0, L*0.6, gk_surge_y());
  vec3 inner = mix(GK_SAPPHIRE*0.7, GK_CRIMSON*0.9, red);
  vec3 rim = mix(GK_SKY*1.3, GK_RED*1.3, red);
  acc += (0.42 + 0.3*sg)*rxAuraShell(ro, rd, -0.5, (L + 0.3)*grow, (3.4 + 0.5*sg)*grow, 3.0, inner, rim);
  // the streaks of ki rushing up through it, as an aura is drawn: long thin lines of light
  float s;
  for (int i = 0; i < 16; i++){
    float k = float(i);
    float q = fract(tCalm()*(0.5 + 0.3*h11(k*2.7)) + h11(k*6.1));
    float a = h11(k*4.9 + floor(tCalm()*(0.5 + 0.3*h11(k*2.7)) + h11(k*6.1)));
    float r = (1.2 + 2.6*h11(k*3.3))*grow;
    float y = L*(0.05 + 0.85*q);
    vec3 p0 = around(r, a, y);
    vec3 p1 = around(r, a, min(y + 3.5, L));
    float d = raySeg(ro, rd, p0, p1, s);
    float life = sin(3.1416*q);
    vec3 c = mix(GK_WHITE, GK_GOLD, gk_red_at(y, sg));
    acc += (c*halo(d, max(0.04, px))*1.1*sin(3.1416*s) + mix(GK_SKY, GK_RED, gk_red_at(y, sg))*halo(d, 0.25)*0.25)*life*roomAt(p0);
  }
  return acc;
}

// effect 4: threads of blue lightning crawling up through the aura now and then
vec3 gk_threads(vec3 ro, vec3 rd, float px, float sg){
  vec3 acc = vec3(0.0);
  float L = uLength, f;
  if (fxNear(ro, rd, 0.0, L) > 16.0) return acc;
  // (update 16) blue lightning over the whole blade to its point, a steady crackle, and threads
  // climbing up through the aura about once a second each
  float top = gk_top();
  vec3 c = mix(GK_SKY, GK_GOLD, sg);
  acc += 1.3*shBolts(ro, rd, px, 0.8, 0.4, top - 0.15, 3.0, 7.0, 4.333, 0.35, 61.0, GK_WHITE, c);
  for (int i = 0; i < 5; i++){
    float k = float(i);
    float cyc = uClock*(1.1 + 0.4*h11(k*3.1)) + h11(k*5.7);
    float n = floor(cyc), q = fract(cyc);
    if (q > 0.4) continue;
    float life = smoothstep(0.0, 0.04, q)*(1.0 - smoothstep(0.15, 0.4, q));
    float y0 = L*(0.06 + 0.6*h11(n*1.7 + k));
    float a = h11(n*4.3 + k);
    // (update 18) a new crooked path many times a second, and a fork off its middle
    float cr = floor(uClock*15.0 + k*0.31);
    vec3 p0 = around(0.8, a, y0), p1 = around(2.6, a + 0.14, min(y0 + 8.0, L - 0.3));
    vec3 pf = around(3.3, a - 0.03, y0 + 3.0 + 3.0*h11(n*2.3 + k));
    float d = min(bolt3(ro, rd, p0, p1, n*13.7 + k + cr, 0.22, px), bolt3(ro, rd, mix(p0, p1, 0.42), pf, n*7.1 + k + cr, 0.26, px) + 0.012);
    acc += (GK_WHITE*halo(d, max(0.06, px))*2.4 + c*halo(d, 0.32)*1.0 + c*halo(d, 1.1)*0.22)*life;
  }
  return acc;
}

// let go, the scatter of sparkles; a clash, a blue-white flash
vec3 gk_moments(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float t;
  if (uAge >= 2000.0 && uAge < 2001.2){
    float q = (uAge - 2000.0)/1.2;
    for (int i = 0; i < 16; i++){
      float k = float(i);
      vec3 p = around(0.6 + 2.5*q*h11(k*4.1), h11(k*6.3), uLength*0.9*h11(k*2.9) + 1.5*q);
      acc += (GK_WHITE*halo(rayPt(ro, rd, p, t), max(0.07, px))*1.5 + GK_SKY*halo(rayPt(ro, rd, p, t), 0.3)*0.4)*(1.0 - q)*roomAt(p);
    }
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) acc += (GK_WHITE*halo(rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), t), 1.0)*1.4 + GK_SKY*halo(rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), t), 3.0)*0.4)*cl;
  return acc;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float top = gk_top();
  float after = gk_after();
  float sg = gk_surge();
  if (part(1.0) > 0.5 && (after > 0.01 || uAge >= 2000.0)) fx += gk_crown(ro, rd, px, sg, top);
  // effect 3: sparkles, round points of god ki rising through the flames, red-gold in the surge
  // effect 3 (update 16, in place of the small sparkles Creyon found cheap): the star streams, the
  // Star Forge's suns drawn into the blade in curving streams of plasma (`gk_forge`)
  if (part(2.0) > 0.5 && after > 0.01) fx += gk_forge(ro, rd, px, sg, true)*after;
  if (part(3.0) > 0.5 && after > 0.01) fx += gk_threads(ro, rd, px, sg)*after;
  fx += gk_moments(ro, rd, px);
  return fx*uFx;
}


vec3 sig_vol_raw(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(5.0) > 0.5) fx += x14_rakata_infinite(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  return fx*hiltFade(ro, rd);
}

// The Star Forge (update 16, Creyon: Rakata looked cheap; its own identity: the Rakata built the Star
// Forge, which fed on a star): effect 6, three small suns hang round the blade, each a white-hot
// heart in a turning corona of sky-blue flares, drifting slowly round it; effect 3, each pours
// itself into the blade in a curving stream of plasma, pulses of light running down the stream into
// the blade. A sun is drawn in for twelve seconds, shrinking and dimming as it feeds the blade,
// and a new one kindles elsewhere. Red-gold in the limit surge.
vec3 gk_sun(float k, out float life){
  float per = 7.5;
  float cyc = uClock/per + k/6.0 + uSeed;
  float n = floor(cyc), q = fract(cyc);
  life = smoothstep(0.0, 0.12, q)*(1.0 - smoothstep(0.8, 1.0, q))*(1.0 - 0.45*q);
  float a = h11(n*3.7 + k) + 0.035*uClock;
  return around(3.2 + 1.2*h11(n*5.3 + k), a, uLength*(0.14 + 0.74*h11(n*7.1 + k*2.3)));
}
vec3 gk_forge(vec3 ro, vec3 rd, float px, float sg, bool streams){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return acc;
  float t, s;
  vec3 hot = mix(GK_WHITE, GK_GOLD, sg), c = mix(GK_SKY, GK_RED, sg);
  for (int i = 0; i < 6; i++){
    float k = float(i);
    float life;
    vec3 p = gk_sun(k, life);
    if (life <= 0.0) continue;
    if (!streams){
      float sz = (0.28 + 0.14*h11(k*3.3))*life + 0.08;
      acc += fxFlare(ro, rd, p, px, sz, 1.4*life + 0.4, 6.0, uClock*0.7 + k, c)*life*1.2;
      acc += (hot*halo(rayPt(ro, rd, p, t), sz*2.0)*0.8 + c*halo(rayPt(ro, rd, p, t), 1.6)*0.25)*life;
    } else {
      // its stream: a curve from the sun to the blade, bowing up and round
      vec3 foot = around(0.5, h11(k*9.1) + 0.035*uClock + 0.15, p.y - 3.0);
      vec3 ctl = mix(p, foot, 0.5) + vec3(0.0, 2.2, 0.0) + normalize(vec3(p.x, 0.0, p.z) + vec3(1e-4))*0.8;
      float bWs = 0.0;
      float best = 1e3, bf = 0.0;
      vec3 prev = p;
      for (int j = 1; j <= 8; j++){
        float g = float(j)/8.0;
        vec3 q = mix(mix(p, ctl, g), mix(ctl, foot, g), g);
        float d = raySeg(ro, rd, prev, q, s);
        { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(j) - 1.0 + s)/8.0 - bf)*wN/bWs; }
        prev = q;
      }
      float pulse = pow(0.5 + 0.5*sin(bf*18.0 - uClock*15.0 + k*2.0), 6.0);
      float w = mix(0.14, 0.05, bf);
      acc += (hot*halo(best, max(w, px))*(0.6 + 1.4*pulse) + c*halo(best, w*4.0 + 0.15)*0.45)*life*smoothstep(0.0, 0.1, bf);
    }
  }
  return acc;
}
vec3 x14_rakata_infinite(vec3 ro, vec3 rd, float px){
  return gk_forge(ro, rd, px, gk_surge(), false);
}
// (update 26) the body starts softly in the emitter, no hard end above the hilt
vec4 sig_solid(vec4 o, vec2 b, float px){
  vec4 r = sig_solid_raw(o, b, px);
  float j = smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, r.rgb, j), mix(o.a, r.a, j));
}
// nothing over the point (update 26)
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  float s;
  raySeg(ro, rd, vec3(0.0), vec3(0.0, uLength + 4.0, 0.0), s);
  return sig_vol_raw(ro, rd, px)*(1.0 - smoothstep(uLength + 0.2, uLength + 2.0, s*(uLength + 4.0)));
}
// (update 29, Creyon: nothing past the tip) every effect ends at the blade's end
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  // (update 31, Creyon) under an RGB all its effects turn with the blade
  uTurnFx += sig_vol_tc(ro, rd, px)*tipFade(ro, rd);
  return vec3(0.0);
}
vec3 sig_glow_34(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(0.200, 0.700, 1.000);
const vec3 SX_DEEP = vec3(0.000, 0.250, 0.850);
const vec3 SX_PALE = vec3(0.750, 0.950, 1.000);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += syInflow(ro, rd, px, 12.0, 6.0, L*0.1, L*0.92, 0.2, 71.0, vec3(1.0), SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_d(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.7*sxFlames(b, 0.85, 2.8, 1.9, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34

// >>> dim: less glow, so its effects and body can be seen
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){ return sig_glow_d(g, b, r, px)*0.5; }
vec3 sig_core(vec3 c, vec2 b, float px){ return sig_core_d(c, b, px)*0.6; }
// <<< dim

// >>> update 52: this blade's great effects in the air
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 prevT = uTurnFx;
  vec3 base = sig_vol_p1(ro, rd, px)*0.5;
  uTurnFx = prevT + (uTurnFx - prevT)*0.5;
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  for (int i = 0; i < 8; i++){
    float k = float(i);
    float run = C*(0.38 + 0.08*h11(k + 401.0)) + k*0.29, cyc = floor(run), ph = fract(run);
    float y = L*((k + h11(k*2.3 + cyc*1.3))/8.0)*0.86 + 0.3;
    a += 1.0*sin(3.1416*ph)*syLick(ro, rd, px, y, h11(k*5.7 + cyc*0.7 + 401.0) + 0.08*ph, 0.8 + 1.0*h11(k*3.1 + cyc), 4.5 + 3.5*h11(k*4.3 + cyc), 0.72, k*3.0 + cyc + 401.0, vec3(0.85, 0.95, 1.0), vec3(0.2, 0.55, 1.0), vec3(0.03, 0.12, 0.5));
  }
  a += 1.2*xDust(ro, rd, 1.4, 4.0, 1.6, C*0.1, L*0.03, L*0.97, 0.9, 402.0, vec3(0.9, 0.97, 1.0), vec3(0.3, 0.6, 1.0));

  uTurnFx += a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return base;
}
// <<< update 52
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(5.0) > 0.5){
    a += 1.3*vec3(0.0);
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
vec3 sig_vol_m26(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.43*gxKin(ro, rd, 3.0, 8.0, 3.00, 0.060, 0.14, 0.08, 0.96, 1.0, 9289.0, vec3(0.750, 0.950, 1.000), vec3(0.200, 0.700, 1.000));
  a += 1.05*gxFlares(ro, rd, px, 5.0, 5.4, 201.0, vec3(0.750, 0.950, 1.000), vec3(0.200, 0.700, 1.000));
  a += 1.4*gxInfinity(ro, rd, px, 24.0, 4.6, 1.0, vec3(0.750, 0.950, 1.000), vec3(0.200, 0.700, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> its own moment
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_m26(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float m = rxCycle(58.0, 22.0, 31.0, 37.0, 47.0), L = uLength;
  a += m*1.5*gxInfinity(ro, rd, px, 24.0, 4.6 + 1.2*m, 1105.0, vec3(0.750, 0.950, 1.000), vec3(0.200, 0.700, 1.000));
  a += m*1.2*gxHelix(ro, rd, px, 18.0, 1.5, 3.0, 0.07, 1106.0, vec3(0.750, 0.950, 1.000), vec3(0.200, 0.700, 1.000));
  return base*(1.0 + 0.35*m) + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

