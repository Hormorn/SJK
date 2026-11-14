// DOOM: the third Unique blade (Sol, 13/11/2026), a thank-you given by hand, its name always
// written in capitals and in red. A blood-red blade with a black heart, beating as a heart does:
// a black core cracked with red fire runs up it, a dark marbling crawls through its light and
// every beat throbs up it from the hilt. A death aura hangs round it: black smoke rolls up it in
// a red glow, red sparks rising through it. Red lightning crackles off it all the time, and now
// and then thunder: a great bolt leaps far out from it with a flash, more of them as it swings
// and a burst of them on a clash. The doomed circle it, red-eyed skulls rising slowly round it
// and fading into the smoke, and its swings shed black cinders still burning red at their edges
// (`@petals`, `saber_petals.rs`).
// @part Black heart
// @part Death aura
// @part Red lightning
// @part Doomed souls
// @part Cinders
// @arcs Red lightning
// @petals {"part":4,"kind":"doom","per_unit":0.06,"life":2.4,"size":1.1,"fall":0.0,"drift":10.0,"rise":16.0}
// @room 10
// @solid 1
// @palette {"corona.rim_cool":[0.55,0.0,0.01],"corona.rim_hot":[1.0,0.08,0.04],"corona.inner":[1.0,0.36,0.3],"core.white":[1.0,0.82,0.78],"core.fringe_cool":[0.85,0.02,0.02],"core.fringe_hot":[1.0,0.3,0.22],"light.color":[1.0,0.08,0.05],"trail":[0.9,0.04,0.03]}

const vec3 DO_RED = vec3(1.0, 0.05, 0.03);
const vec3 DO_HOT = vec3(1.0, 0.42, 0.32);
const vec3 DO_DEEP = vec3(0.42, 0.0, 0.01);
const vec3 DO_BLACK = vec3(0.012, 0.004, 0.004);

// the heartbeat: a heavy double thump (lub-dub) every 1.6 seconds, 1 at its peak
float do_beat(){
  float t = mod(uTime + uSeed*3.0, 1.6);
  return exp(-t*8.0) + 0.65*step(0.26, t)*exp(-(t - 0.26)*8.0);
}

// the beat's wave running up the blade from the hilt, at y
float do_wave(float y){
  float t = mod(uTime + uSeed*3.0, 1.6);
  float at = t*uLength*1.6;
  float w = exp(-pow((y - at)/2.2, 2.0))*(1.0 - smoothstep(0.4, 0.8, t));
  return w;
}

// how awake the doom is: it gathers as the blade lights and dies as it goes out
float do_wake(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(2000.0, 2000.6, uAge);
  return smoothstep(0.15, 1.1, uAge);
}

// the blade: Sol's glow from the file in blood red, a dark marbling crawling up through its
// light, throbbing with every beat and with the wave the beat sends up it
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float l = lvl(g);
  vec3 col = recol(g, DO_RED)*1.25;
  float n = fbm3(vec2(b.x*3.2/max(r, 0.2) + 7.0, b.y*0.22 - uTime*0.55));
  float dark = smoothstep(0.42, 0.68, n)*smoothstep(0.3, 1.4, b.y);
  col *= 1.0 - 0.6*dark*part(0.0);
  float beat = do_beat();
  col *= 0.85 + 0.35*beat;
  col += l*DO_HOT*(0.3*do_wave(b.y) + 0.12*beat)*part(0.0);
  // a hard flicker now and then, as if the light fought the dark in it
  col *= 0.92 + 0.08*step(0.5, vn(vec2(uTime*9.0, 3.0)));
  return col;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*vec3(1.0, 0.5, 0.45)*(0.8 + 0.3*do_beat()); }

// the black heart: a narrow black core up the middle of the blade, ragged at its edges, cracked
// with red fire that flares with each beat
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < 0.5 || b.y > L*0.97) return o;
  float half = 0.17 + 0.06*vn(vec2(b.y*1.3 - uTime*0.4, 2.0)) + 0.04*do_wave(b.y);
  float cov = 1.0 - smoothstep(half - px, half + 0.04 + px, abs(b.x));
  cov *= smoothstep(0.5, 1.4, b.y)*(1.0 - smoothstep(L*0.88, L*0.97, b.y))*part(0.0)*do_wake();
  if (cov <= 0.0) return o;
  float f1 = rxFlow(vec2(b.x*6.0 + 0.3, b.y*0.35 - uTime*0.3));
  float f2 = rxFlow(vec2(b.x*9.0 + 4.0, b.y*0.6 + uTime*0.25));
  float crack = max(1.0 - smoothstep(0.0, 0.05, abs(f1 - 0.5)), 0.7*(1.0 - smoothstep(0.0, 0.04, abs(f2 - 0.5))));
  vec3 col = DO_BLACK + DO_RED*crack*(0.7 + 1.1*do_beat() + 0.8*do_wave(b.y));
  return vec4(mix(o.rgb, col, cov*0.92), max(o.a, cov*0.92));
}

// thunder: every few seconds a great bolt leaps out from the blade to a point far off it, with a
// flash where it leaves; each one somewhere else round it
vec3 do_thunder(vec3 ro, vec3 rd, float px, float slot_rate, float seed, float reach){
  float L = uLength;
  float clock = uTime*slot_rate + seed;
  float k = floor(clock), q = fract(clock);
  if (h11(k*3.1 + seed) < 0.15) return vec3(0.0);
  float life = (1.0 - q)*(1.0 - q)*smoothstep(0.0, 0.02, q);
  if (life <= 0.01) return vec3(0.0);
  float flick = 0.6 + 0.4*step(0.5, fract(q*14.0));
  float a = h11(k*5.7 + seed);
  float y = L*(0.25 + 0.65*h11(k*2.3 + seed));
  vec3 from = around(0.2, a, y);
  vec3 to = around(min(reach, 0.62*uReach), a + 0.08*(h11(k*7.1) - 0.5), y + (h11(k*9.7 + seed) - 0.3)*6.0);
  float t;
  vec3 acc = DO_RED*halo(rayPtRaw(ro, rd, from, t), 2.4)*1.1*life;
  float d = bolt3(ro, rd, from, to, k + seed*1.7, 0.45, px);
  vec3 fork_end = mix(from, to, 0.55) + around(2.4, a + 0.2, 1.5*(h11(k*4.4) - 0.5));
  float d2 = bolt3(ro, rd, mix(from, to, 0.45), fork_end, k*1.9 + seed, 0.55, px);
  acc += (arcGlow(d, DO_RED*2.2, px) + vec3(1.0, 0.75, 0.7)*halo(d, max(0.06, px*0.9))*1.8 + DO_RED*halo(d, 0.9)*0.5 + arcGlow(d2, DO_RED*1.6, px)*0.8)*life*flick;
  return acc;
}

// where a doomed soul is: three skulls rising slowly round the blade, each in turn
vec4 do_soul(float i){
  float L = uLength;
  float rise = fract(uTime*0.045*(0.85 + 0.3*h11(i*3.9)) + i/3.0);
  float a = h11(i*4.7 + floor(uSeed*5.0)) + uTime*0.03*(i < 1.5 ? 1.0 : -1.0);
  float R = 4.2 + 1.2*h11(i*2.2) + 0.3*sin(uTime*0.5 + i*2.0);
  vec3 p = around(R, a, mix(L*0.1, L*1.05, rise) + 0.3*sin(uTime*0.8 + i*1.7));
  float fade = smoothstep(0.0, 0.18, rise)*(1.0 - smoothstep(0.65, 1.0, rise));
  return vec4(p, fade);
}

// a skull seen from the front in its own plane (x across, y up, about 2 tall): the cranium, the
// cheekbones and jaw, two deep eye sockets, the nose, a row of teeth; rgb and how much it covers
vec4 do_skull(vec2 q, float px, float glow){
  if (abs(q.x) > 1.1 || abs(q.y) > 1.2) return vec4(0.0);
  float head = sdEll(q - vec2(0.0, 0.2), vec2(0.72, 0.7));
  float jaw = sdBox(q - vec2(0.0, -0.45), vec2(0.36, 0.26)) - 0.08;
  float cheek = sdEll(q - vec2(0.0, -0.12), vec2(0.62, 0.32));
  float skull = min(min(head, jaw), cheek);
  vec2 e = vec2(abs(q.x) - 0.27, q.y - 0.02);
  float eye = sdEll(e, vec2(0.2, 0.17));
  float nose = sdEll(q - vec2(0.0, -0.22), vec2(0.07, 0.12));
  float teeth = abs(fract((q.x + 0.3)*7.0) - 0.5)/7.0 - 0.012;
  teeth = max(teeth, abs(q.y + 0.47) - 0.1);
  teeth = max(teeth, abs(q.x) - 0.3);
  float cov = fillD(skull, px);
  float hole = max(fillD(eye, px), fillD(nose, px));
  float gap = fillD(teeth, px);
  // bone dark with old blood, lit faintly red from the blade, shadowed toward its rim
  vec3 bone = mix(vec3(0.16, 0.05, 0.05), vec3(0.36, 0.14, 0.12), smoothstep(0.0, -0.45, skull));
  vec3 col = mix(bone, DO_BLACK, max(hole, gap*0.8));
  // the eyes burn red in their sockets
  float burn = exp(-pow(length(e)/0.11, 2.0))*glow;
  col += DO_RED*burn*1.6 + DO_HOT*exp(-pow(length(e)/0.045, 2.0))*glow;
  return vec4(col, cov);
}

// the souls, drawn facing the camera, fading as they rise into the smoke
vec4 do_souls(vec3 ro, vec3 rd, float px){
  vec4 res = vec4(0.0);
  if (fxNear(ro, rd, 0.0, uLength + 4.0) > 9.5) return res;
  for (int i = 0; i < 3; i++){
    float fi = float(i);
    vec4 s = do_soul(fi);
    float size = 2.8*s.w;
    if (size < 0.05) continue;
    float t;
    float d = rayPt(ro, rd, s.xyz, t);
    if (d > size*1.6) continue;
    vec3 fu = normalize(cross(rd, abs(rd.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
    vec3 fw = cross(fu, rd);
    vec3 q3 = ro + rd*t - s.xyz;
    vec2 q = rot(0.15*sin(uTime*0.6 + fi*2.0))*vec2(dot(q3, fu), dot(q3, fw))/size;
    vec4 c = do_skull(q, px/size, 0.7 + 0.5*do_beat());
    float a = c.a*min(s.w*1.4, 1.0)*0.95*roomAt(s.xyz);
    res = vec4(mix(res.rgb, c.rgb, a), max(res.a, a));
  }
  return res;
}

// the death aura's light, the red sparks rising through it, the crackling lightning and the
// thunder, more of it on a swing and a burst of it on a clash
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength;
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float wake = do_wake();
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float beat = do_beat();
  if (part(1.0) > 0.5){
    fx += fxAura(ro, rd, 0.0, L, 4.2, DO_DEEP*1.6)*(0.8 + 0.6*beat);
    fx += fxMotes(ro, rd, px, 0.5, L, 0.6, 4.0, 32.0, 0.25, 13.0, DO_RED*1.4);
  }
  if (part(2.0) > 0.5){
    fx += fxArcs(ro, rd, px, L*0.05, L*0.95, 5.0 + 3.0*sw + 2.0*cl, 5.5 + 2.0*cl, 1.8 + 1.2*sw, 7.0, DO_RED*2.0);
    fx += do_thunder(ro, rd, px, 0.45, 3.0, 12.0);
    fx += do_thunder(ro, rd, px, 0.35, 41.0, 10.0);
    fx += do_thunder(ro, rd, px, 0.9, 77.0, 8.0)*sw;
  }
  return fx*wake*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}

// the death aura's black smoke rolling up the blade
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  if (uFx <= 0.0 || part(1.0) < 0.5 || fxNear(ro, rd, 0.0, uLength) > 16.0) return vec4(0.0);
  vec4 s = rxSmoke(ro, rd, 0.4, uLength + 1.0, 5.0, 1.3, 9.0, DO_BLACK*2.0, 1.4);
  s.a *= 0.92*uFx*do_wake()*tipFade(ro, rd)*hiltFade(ro, rd);
  return s;
}

// the souls' red eyes glowing round them
vec3 sig_orbit(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  if (uFx <= 0.0 || part(3.0) < 0.5) return acc;
  for (int i = 0; i < 3; i++){
    vec4 s = do_soul(float(i));
    float t;
    acc += DO_RED*halo(rayPtRaw(ro, rd, s.xyz, t), 3.0)*0.3*s.w;
  }
  return acc*do_wake()*uFx;
}

// the doomed souls rising round the blade, in three dimensions
vec4 sig_orbit_solid(vec3 ro, vec3 rd, float px){
  if (uFx <= 0.0 || part(3.0) < 0.5) return vec4(0.0);
  return do_souls(ro, rd, px)*do_wake()*uFx;
}
