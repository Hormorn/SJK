// Moon blade (blade 270, rebuilt 4/11/2026): The Sun's night sister and Creyon's own blade: a blade of pure moonlight, pearl white with soft grey seas drifting through it. A slow tide of silver rises and falls along it, veils of moonlight wind round it, stars glitter in its mist and falling stars streak past. In its great moment the Earth's shadow takes it: a black blade in a blazing silver corona.
// @part Moonlight
// @part Moon glitter
// @part Silver tide
// @part Falling stars
// @part Night mist
// @part Lunar eclipse
// @wisps Moon glitter
// @petals {"part":1,"kind":"moon","per_unit":0.6,"life":3.4,"size":1.1,"fall":0.0,"drift":20.0,"rise":30.0,"moving":0.8}
// @extras {"card":"sjk_alpha_002","card_name":"SJK Alpha 002 (Creyon)","finish":"solar"}
// @extra {"id":"moonshadow_trail","name":"Moonshadow trail","trail":"eclipse"}
// @extra {"id":"starfall_trail","name":"Starfall trail","trail":"starfall"}
// @extra {"id":"moonlit_wake","name":"Moonlit wake","wake":true}
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail moonlight
// @palette {"corona.rim_cool":[0.34,0.42,0.62],"corona.rim_hot":[0.78,0.84,1.0],"corona.inner":[0.95,0.97,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.72,0.78,0.92],"core.fringe_hot":[0.94,0.96,1.0],"light.color":[0.85,0.9,1.1],"trail":[0.8,0.86,1.0]}
// @speed 1.0

const vec3 Wh = vec3(0.985, 0.985, 1.000);
const vec3 Pe = vec3(0.880, 0.905, 0.960);
const vec3 Se = vec3(0.520, 0.570, 0.700);
const vec3 Si = vec3(0.700, 0.780, 1.000);
const vec3 Bl = vec3(0.280, 0.360, 0.640);
const vec3 Nt = vec3(0.030, 0.045, 0.110);
float mo_top(){ return uLength*rxLife(0.0, 0.9, 0.55); }
// the eclipse: 0 none, 1 total; every 26 seconds, slowly in and slowly out
float mo_ecl(){ return part(5.0) > 0.5 ? rxCycle(56.0, 14.0, 22.0, 42.0, 51.0) : 0.0; }
float mo_shadow(float y){ float e = mo_ecl(); return e <= 0.0 ? 0.0 : smoothstep(uLength*(1.5 - 2.0*e) - 7.0, uLength*(1.5 - 2.0*e) + 7.0, y); }
// the tide: a long swell of light going slowly up the blade and sinking back
float mo_tide(float y){
  float h = uLength*(0.5 + 0.55*sin(uClock*0.42));
  float dy = (y - h)/5.5;
  return part(2.0) > 0.5 ? exp(-dy*dy) : 0.0;
}
float mo_half(float y){
  float top = mo_top();
  if (y > top) return 0.0;
  float L = max(uLength, 1.0);
  float w = mix(0.62, 0.46, smoothstep(0.0, L, y))*(0.4 + 0.6*rxLife(0.0, 1.1, 0.45)) + 0.05*mo_tide(y);
  float e = 1.0 - clamp((top - y)/3.0, 0.0, 1.0);
  return w*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  float top = mo_top(), yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - mo_half(yc), 0.0);
  float sh = mo_shadow(b.y), ecl = mo_ecl(), tide = mo_tide(b.y);
  vec3 col = g*(0.34 + 0.34*tide)*(1.0 - 0.9*sh);
  col += (Wh*halo(d, 0.07)*0.5 + Si*(halo(d, 0.42)*(0.26 + 0.26*tide) + halo(d, 1.9)*(0.13 + 0.14*tide)) + Bl*halo(d, 4.6)*0.1)*(1.0 - 0.92*sh);
  // moonbeams: broad soft shafts of light leaning off the blade, slowly passing
  if (part(2.0) > 0.5){
    float bm = rxFlow(vec2(b.y*0.42 + abs(b.x)*0.16 - uClock*0.09, b.x > 0.0 ? 2.0 : 6.0));
    col += Si*bm*bm*bm*halo(d, 3.4)*(0.5 + 0.5*tide)*(1.0 - sh);
  }
  // silver light round the night
  if (ecl > 0.0){
    float st = 0.6 + 0.4*sin(b.y*0.9 + uClock*0.5 + 2.0*sin(b.y*0.27 - uClock*0.3));
    col += sh*ecl*(Wh*halo(d, 0.05)*0.7 + Si*(halo(d, 0.32)*0.42*st + halo(d, 1.8)*0.3*st) + Bl*halo(d, 4.4)*0.2);
  }
  // moonrise: the silver point climbing as it lights
  float rise = 1.0 - smoothstep(0.5, 1.0, rxLife(0.0, 0.95, 0.5));
  col += (Wh*halo(length(vec2(b.x, b.y - top)), 0.8)*1.3 + Si*halo(length(vec2(b.x, b.y - top)), 2.6)*0.6)*rise;
  return (col + g*0.7*clamp(uClash, 0.0, 1.0))*tipFadeB(b)*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = mo_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  vec3 col = Pe;
  float tide = mo_tide(b.y);
  if (part(0.0) > 0.5){
    // moonlight with the moon's face in it: soft grey seas drifting, a pearl sheen, the faintest craters up close
    float a = asin(u);
    vec2 p = vec2(a*0.75 + 0.03*sin(b.y*0.4), b.y*0.62 - 0.05*uClock);
    float see = 1.0 - smoothstep(0.02, 0.08, px);
    float sea = smoothstep(0.42, 0.72, vn(p*0.85 + 4.0)*0.66 + vn(p*2.1 + 9.0)*0.34);
    col = mix(Wh, Se, sea*0.72);
    col *= 0.96 + 0.08*vn(p*4.5);
    for (int i = 0; i < 2; i++){
      float s0 = i == 0 ? 1.3 : 3.3;
      vec2 g0 = p*s0 + float(i)*7.3, i0 = floor(g0), f0 = fract(g0) - 0.5;
      float h = h11(i0.x*7.1 + i0.y*3.3 + float(i)*11.0);
      if (h < 0.55) continue;
      vec2 c0 = 0.28*(vec2(fract(h*17.0), fract(h*31.0)) - 0.5);
      float rc = 0.1 + 0.16*fract(h*47.0), dc = length(f0 - c0);
      float side = dot((f0 - c0)/max(dc, 0.001), vec2(0.45, 0.89));
      col *= 1.0 - see*(1.0 - smoothstep(rc*0.25, rc, dc))*(0.08 + 0.07*side);
      col += Wh*see*(1.0 - smoothstep(0.0, 0.05, abs(dc - rc)))*(0.03 + 0.08*side);
    }
  }
  // lit from within: a pale heart, silver toward its edges
  col *= 0.8 + 0.26*nz;
  float tipk = smoothstep(0.0, 3.2, mo_top() - b.y);
  col = mix(col, Si, pow(1.0 - nz, 2.0)*0.75*tipk);
  col += Wh*tide*0.2;
  float sh = mo_shadow(b.y);
  // night comes up it: not black but deep blue sky with stars in it, its edges still silver
  if (sh > 0.0){
    float sw0 = clamp(uSwing, 0.0, 1.0);
    vec2 fl = vec2(b.x*1.4 + 0.4*sin(b.y*0.2 + uClock*0.3), b.y*0.3 - 0.16*uClock);
    float neb = vn(fl + 0.6*(vn(fl*1.7 + 3.0) - 0.5));
    vec3 sky = mix(vec3(0.03, 0.05, 0.16), vec3(0.14, 0.2, 0.46), neb)*(1.0 + 0.7*sw0);
    for (int i0 = 0; i0 < 2; i0++){
      float f0 = float(i0);
      vec2 sq = vec2(b.x*(3.2 + 1.6*f0), b.y*(1.4 + 0.7*f0) - (0.34 - 0.14*f0)*uClock) + f0*7.0, si = floor(sq), sp = si + 0.25 + 0.5*h22(si), ds = sq - sp;
      sky += Wh*halo(length(vec2(ds.x, ds.y/(1.0 + 3.0*sw0))), 0.09)*step(0.6, h21(si))*(0.6 + 0.4*sin(uClock*(0.8 + h21(si)) + h21(si)*20.0));
    }
    sky += Si*pow(1.0 - nz, 2.0)*0.28*tipk;
    col = mix(col, sky, sh*0.98);
  }
  // it goes out as the moon sets: from the point, a shadow coming down
  col = mix(col, Wh, 0.45*clamp(uClash, 0.0, 1.0));
  cov *= smoothstep(-0.25, 0.3, b.y)*(1.0 - smoothstep(uLength - 0.9, uLength + 0.1, b.y));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
// The moon herself, hanging high beside the blade and moving slowly round it. The night passing over the blade passes over her too.
vec3 mo_moon(){ return around(4.0 + 0.2*sin(uClock*0.09), 0.11 + uClock*0.005, uLength*0.74 + 0.45*sin(uClock*0.13)); }
// A falling star: rare, far out, a long soft stroke crossing the blade's air; never the same place twice.
vec3 mo_star(vec3 ro, vec3 rd, float px, float k){
  float L = uLength;
  float run = uClock*(0.075 + 0.015*h11(k*1.9)) + k*0.37, cyc = floor(run), q = fract(run);
  float go = clamp(q/0.5, 0.0, 1.0);
  if (q > 0.5) return vec3(0.0);
  float a = h11(k*2.3 + cyc*1.7), R = 3.1 + 2.3*h11(k*5.1 + cyc);
  float y0 = L*(0.55 + 0.42*h11(k*3.7 + cyc)), run_y = 7.0 + 6.0*h11(k*6.3 + cyc), turn = (0.09 + 0.08*h11(k*8.1 + cyc))*(mod(cyc + k, 2.0) < 0.5 ? 1.0 : -1.0);
  float hf = go*(0.6 + 0.4*go);
  float tf = max(hf - 0.42, 0.0);
  vec3 h0 = around(R, a + turn*hf, y0 - run_y*hf), t0 = around(R + 0.3, a + turn*tf, y0 - run_y*tf);
  float s;
  float d = raySeg(ro, rd, t0, h0, s);
  float env = smoothstep(0.0, 0.06, q)*(1.0 - smoothstep(0.34, 0.5, q));
  float w = 0.02 + 0.045*s*s;
  return (Wh*halo(d, max(w*0.6, px))*(0.5 + 1.7*s*s) + Si*(halo(d, w*2.4)*(0.5 + 0.7*s) + halo(d, 0.5)*0.12*s))*s*env;
}
vec3 sig_vol_r24(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.35, 1.5, 0.5);
  if (aft <= 0.01) return fx;
  float ecl = mo_ecl(), day = 1.0 - 0.75*ecl;
  if (part(1.0) > 0.5){
    // star dust: a few bright stars that twinkle and a fine glitter round them
    fx += (1.5 + 1.2*ecl)*xDust(ro, rd, 2.2, 3.8, 1.1, C*0.03, L*0.03, L*0.97, 1.5, 271.0, Wh, Si);
    fx += (1.4 + 1.6*ecl)*shMotes(ro, rd, 4.4, 0.5, L - 0.5, 24.0, 2.4, 0.05, 0.01, 0.05, 0.55, 0.0, 272.0, Wh, Si);
    fx += 1.2*shMotes(ro, rd, 2.8, 0.5, L - 0.5, 22.0, 2.0, -0.32, 0.016, 0.035, 0.6, 0.0, 276.0, Wh, Si*0.6);
  }
  if (part(2.0) > 0.5){
    // the tide's veil: a silver mist turning slowly, no lines in it
    fx += day*0.8*syMistSpin(ro, rd, 0.3, L*0.97, 1.6, 4.4, 0.01, 273.0, Si*0.32, Bl*0.3);
  }
  if (part(3.0) > 0.5) for (int i = 0; i < 3; i++) fx += (1.0 + 0.8*ecl)*mo_star(ro, rd, px, float(i));
  // at night its light turns to stars: a whole sky comes out round it, shining points of four rays and fine dust
  if (ecl > 0.01){
    fx += ecl*1.9*syGlints(ro, rd, px, 14.0, 5.0, 279.0, Wh, Si);
    fx += ecl*1.6*syGlints(ro, rd, px, 14.0, 3.2, 280.0, Wh, Si*0.7);
    fx += ecl*2.0*shMotes(ro, rd, 3.6, 0.5, L - 0.5, 30.0, 1.6, 0.0, 0.004, 0.03, 0.75, 0.3, 281.0, Wh, Si);
  }
  if (part(0.0) > 0.5){
    // her light on the air, and thin cloud crossing before her
    float tm, dm = rayPtRaw(ro, rd, mo_moon(), tm);
    float on = rxLife(0.6, 2.0, 0.4);
    fx += (Wh*halo(dm, 1.7)*0.4 + Si*(halo(dm, 3.0)*0.24))*on*(1.0 - 0.97*ecl);
    // in the night her dark face keeps a thin white rim
    fx += Wh*(halo(abs(dm - 1.5), max(0.022, px*0.7))*0.75 + halo(abs(dm - 1.5), 0.11)*0.12)*on*ecl;
  }
  if (part(4.0) > 0.5) fx += 1.2*syMistSpin(ro, rd, 0.3, L*0.97, 1.4, 5.4, 0.016, 275.0, Si*0.26, Nt*1.4);
  // splinters of moon rock turning slowly round it, silver by day and catching the stars at night
  fx += (1.15 + 0.6*ecl)*gxTumble(ro, rd, px, 12.0, 5.6, 0.0, 0.0, 0.32, 281.0, Wh, Si);
  // by day: pearls of moonlight climbing round it in two strands, a shoal of silver lights, a slow great star now and then
  fx += day*1.2*gxHelix(ro, rd, px, 18.0, 2.5, 1.7, 0.035, 283.0, Wh, Si);
  fx += day*1.1*gxSwarm(ro, rd, px, 10.0, 4.8, 284.0, Wh, Si);
  fx += day*1.05*gxFlares(ro, rd, px, 4.0, 5.2, 285.0, Wh, Si);
  // the corona's great stars opening while the shadow is on it
  if (ecl > 0.02) fx += ecl*1.5*gxFlares(ro, rd, px, 6.0, 5.4, 282.0, Wh, Si*0.7);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx <= 0.0 || part(0.0) < 0.5 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  float tm, R = 1.5*rxLife(0.6, 2.0, 0.4);
  vec3 c = mo_moon();
  float d = rayPtRaw(ro, rd, c, tm);
  if (R < 0.05 || d > R + 0.1) return o;
  vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
  vec3 w0 = ro + rd*tm - c;
  vec2 u = vec2(dot(w0, e1), dot(w0, e2))/R;
  float r0 = length(u), nz = sqrt(max(1.0 - r0*r0, 0.0));
  float tn = uClock*0.012, cs = cos(tn), sn = sin(tn);
  vec3 nn = vec3(u, nz), n0 = vec3(cs*nn.x + sn*nn.z, nn.y, -sn*nn.x + cs*nn.z);
  // height of her ground: broad seas low, fine pits, and craters of two sizes with raised rims
  float sea = smoothstep(0.42, 0.66, 0.68*gx3(n0*2.1 + 4.0) + 0.32*gx3(n0*5.0 + 9.0));
  float h0 = 0.7*gx3(n0*11.0) + 0.3*gx3(n0*24.0 + 3.0) - 0.5*sea;
  float rim = 0.0, pit = 0.0;
  for (int i0 = 0; i0 < 2; i0++){
    float sc = i0 == 0 ? 3.4 : 7.5;
    vec3 g0 = n0*sc + float(i0)*5.0, c1 = floor(g0), f0 = g0 - c1 - 0.5;
    float hc = h11(c1.x*7.1 + c1.y*3.3 + c1.z*5.7);
    if (hc < 0.5) continue;
    float rc = 0.14 + 0.2*fract(hc*47.0), dc = length(f0 - 0.2*(vec3(fract(hc*17.0), fract(hc*31.0), fract(hc*53.0)) - 0.5));
    pit = max(pit, (1.0 - smoothstep(rc*0.4, rc, dc)));
    rim = max(rim, 1.0 - smoothstep(0.0, 0.06, abs(dc - rc)));
  }
  vec3 sun_ = normalize(vec3(-0.55, 0.45, 0.7));
  float e = mo_ecl();
  vec3 dirn = normalize(vec3(sin(e*3.1), 0.2, cos(e*3.1)));
  float grain = gx3(n0*40.0);
  float lam = clamp(dot(nn, sun_)*0.5 + 0.62, 0.0, 1.0);
  vec3 ground = mix(vec3(0.95, 0.95, 0.93), vec3(0.42, 0.44, 0.5), sea*0.85)*(0.8 + 0.3*h0 + 0.08*grain);
  ground = ground*(1.0 - 0.34*pit) + vec3(0.32)*rim*(1.0 - 0.4*sea);
  vec3 col = ground*(0.42 + 0.74*lam)*(0.72 + 0.3*nz);
  // the night's shadow goes across her as a real phase, with a soft edge; what is dark keeps a faint earthshine
  float lit = smoothstep(-0.2, 0.26, dot(nn, dirn));
  vec3 night_ = ground*vec3(0.05, 0.065, 0.12);
  col = mix(night_, col, lit);
  float cov = (1.0 - smoothstep(0.88 - px/R, 1.0 + px/R, r0))*(1.0 - behindBlade(ro, rd, tm));
  return vec4(col, cov*uFx*tipFade(ro, rd)*hiltFade(ro, rd));
}

// >>> how it answers: as it lights, in a swing, in a clash
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_r24(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float ig = uAge < 2000.0 ? 1.0 - smoothstep(0.3, 3.0, uAge) : 0.0, sw = clamp(uSwing, 0.0, 1.0);
  a += ig*1.25*gxHelix(ro, rd, px, 18.0, 2.4, 1.6, 0.14, 301.0, Wh, Si);
  a += 1.3*gxBurst(ro, rd, px, 12.0, 5.0, 2.0, 0.0, 302.0, Wh, Si);
  a += sw*1.1*gxFlares(ro, rd, px, 4.0, 4.0, 303.0, Wh, Si);
  return base + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

