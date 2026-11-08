// Angel blade: A pure off-white saber with nothing added to it, until the soft white clouds of heaven come out of it, slowly, the whole bank after some eight seconds. Then a line of gold runs up its outline, a large golden halo opens above its point and the gold fades; a few feathers and small clouds come away, and every swing leaves a mist of cloud.
// @part Heavenly clouds
// @part Angel feathers
// @part Golden halo
// @raycache fast
// @petals {"part":1,"kind":"feather","per_unit":0.006,"life":4.2,"size":2.6,"fall":13.0,"drift":9.0,"rise":0.0,"moving":0.03}
// @room 10
// @solid 1
// @solidnear 1
// @solidpast 1
// @nearpull 3
// @moments own
// @trail cloud
// @palette {"corona.rim_cool":[0.62,0.6,0.56],"corona.rim_hot":[0.92,0.9,0.85],"corona.inner":[1.0,0.99,0.95],"core.white":[1.0,0.985,0.95],"core.fringe_cool":[0.86,0.85,0.8],"core.fringe_hot":[0.98,0.97,0.93],"light.color":[1.0,0.98,0.92],"trail":[0.95,0.95,0.93]}
// @speed 1.0

const vec3 Wh = vec3(1.000, 0.995, 0.975);
const vec3 Sh = vec3(0.840, 0.850, 0.875);
const vec3 Lo = vec3(0.500, 0.520, 0.600);
// how far it has charged: nothing for a moment, the full bank after five seconds; thins fast as it goes out
float an_charge(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, 0.5, uAge - 2000.0);
  return smoothstep(0.6, 8.0, uAge);
}
float an_final(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, 0.4, uAge - 2000.0);
  return smoothstep(9.5, 11.0, uAge);
}
float an_gold(float y){
  if (uAge >= 2000.0) return 0.0;
  float front = smoothstep(8.0, 10.6, uAge)*(uLength + 6.0);
  return (1.0 - smoothstep(front - 6.0, front, y))*(1.0 - smoothstep(11.6, 14.5, uAge));
}
float an_top(){ return uLength*rxLife(0.0, 0.5, 0.32); }
vec3 sig_glow_e3(vec3 g, vec2 b, float r, float px){
  float top = an_top(), yc = clamp(b.y, 0.0, top);
  float d = length(vec2(b.x, b.y - yc));
  vec3 mist = vec3(0.0);
  vec3 gold_ = vec3(1.0, 0.8, 0.36)*(halo(d, 0.14)*1.1 + halo(d, 0.8)*0.6 + halo(d, 2.4)*0.18)*an_gold(b.y)*part(2.0);
  return gold_ + (Wh*(halo(d, 0.7)*0.42 + halo(d, 2.4)*0.2) + vec3(1.0, 0.94, 0.8)*halo(d, 5.0)*0.05*an_charge() + g*0.25)*smoothstep(-8.0, 6.0, b.y)*growGlow(b) + mist*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
// Round billows: how high the cloud stands at p, the bumps of one size, each swelling slowly as a cloud does.
float an_bumps(vec2 p){
  vec2 i0 = floor(p), f0 = fract(p);
  float sum = 0.0;
  for (int y = -1; y <= 1; y++) for (int x = -1; x <= 1; x++){
    vec2 c0 = vec2(float(x), float(y));
    vec2 h0 = h22(i0 + c0);
    vec2 o0 = 0.5 + 0.38*sin(uClock*(0.1 + 0.08*h0.yx) + 6.2832*h0);
    vec2 d0 = c0 + o0 - f0;
    sum += exp(-4.2*dot(d0, d0));
  }
  return clamp(sum*0.86, 0.0, 1.0);
}
// The cloud's height: large billows, smaller ones heaped on them; slowly rising and spreading off the blade.
float an_height(vec2 p, float fine){
  vec2 q = p + vec2(0.0, -0.045*uClock);
  q += 0.28*vec2(vn(q*0.7 + 3.0) - 0.5, vn(q*0.7 + 8.0) - 0.5);
  float h = 0.66*an_bumps(q*0.3) + 0.28*an_bumps(q*0.72 + 4.0);
  if (fine > 0.01) h += fine*0.13*an_bumps(q*2.4 + 9.0);
  return h;
}
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength, top = an_top();
  vec4 res = o;
  float ch = an_charge();
  // the blade: thin and white, under the clouds, so the bank hides it (Creyon: you should hardly see it)
  if (b.y > -0.3 && b.y < top + 0.3){
    float e = 1.0 - clamp((top - b.y)/1.0, 0.0, 1.0);
    float half = 0.19*sqrt(max(1.0 - e*e, 0.0));
    float cov = (1.0 - smoothstep(half - 0.06 - px, half + 0.04 + px, abs(b.x)))*smoothstep(0.02, 0.1 + px, half)*smoothstep(-0.25, 0.3, b.y);
    vec3 col = mix(vec3(1.0, 0.985, 0.95), vec3(1.0), 1.0 - smoothstep(0.0, 0.8, abs(b.x)/max(half, 0.01)));
    col = mix(col, vec3(1.0), 0.5*clamp(uClash, 0.0, 1.0));
    res = vec4(mix(res.rgb, col, cov), max(res.a, cov));
  }
  // not when the eye is pressed against it; this fills the screen there
  if (part(0.0) > 0.5 && ch > 0.0 && uFx > 0.0 && b.y > -0.1 && b.y < L + 4.0){
    float x = abs(b.x), side = b.x > 0.0 ? 2.0 : 6.0;
    // the bank's shape: wide in slow swells, thin at the hand and the point, spreading off the blade as it charges
    float wide = 5.0 + 4.3*rxFlow(vec2(b.y*0.12 + 0.025*uClock, side));
    // it charges from the hand to the point: a thin first vapour along the blade, filling and spreading for five seconds
    float along = clamp(ch*1.35 - 0.35*b.y/max(L, 1.0), 0.0, 1.0);
    // narrow by the hand and up to the wearer's head height, so the bank does not swallow their head
    wide *= (0.12 + 0.88*along*along*(3.0 - 2.0*along))*(0.18 + 0.82*smoothstep(3.0, 16.0, b.y));
    // towards the point the bank rounds off like the top of a cloud, closing a little above it, not in straight slopes
    float crown = clamp((b.y - (L - 6.0))/9.0, 0.0, 1.0);
    wide *= 0.15 + 0.85*sqrt(max(1.0 - crown*crown, 0.0));
    float q = x/max(wide, 0.01);
    if (q < 1.25){
      float fine = 1.0 - smoothstep(0.015, 0.06, px);
      // (from behind a player the blade's height is foreshortened: the billows keep their shape on the screen)
      float view = sqrt(clamp(dot(bbRd.xz, bbRd.xz), 0.03, 1.0));
      vec2 bq = vec2(b.x, b.y*view);
      float h = an_height(bq, fine);
      float wisp = vn(bq*1.9 + vec2(side, 0.07*uClock));
      float body = h + 0.03 - 0.78*q*q - 0.4*(1.0 - ch) + 0.16*(wisp - 0.5) + 0.26*halo(x, 1.2);
      // the bank closes over the point in a round, billowed cap standing a little above it, lower at its sides,
      // never along a straight line across the blade (Creyon: up close the top of the point was cut flat).
      // Above the point the blade's picture keeps its height on the screen, below it a blade seen along its
      // length is squeezed into a few pixels: a cap below the point stood there as a flat line.
      // the camera right by the blade (a close third person, the wearer's own face): the bank thins away
      // rather than fill the view and swallow the wearer's head (Creyon, 6/11/2026)
      float camd = length(vec3(bbRo.x, bbRo.y - clamp(bbRo.y, 0.0, L), bbRo.z));
      float cap = L + 0.2 + 2.8*(1.0 - min(q*q, 1.0)) + 1.0*(h - 0.55);
      float cov = smoothstep(-0.03, 0.46, body)*smoothstep(0.05, 0.4, ch)*smoothstep(-0.1, 1.2/max(view*view, 0.25), b.y)*(1.0 - smoothstep(cap - 1.6, cap, b.y))*smoothstep(0.26, 0.5, view)*smoothstep(7.0, 17.0, camd);
      if (cov > 0.0){
        float hl = an_height(bq + vec2(0.1, 0.3), 0.0);
        float slope = clamp((h - (fine > 0.01 ? fine*0.065 : 0.0) - hl)*3.6, -1.0, 1.0);
        float lit = 0.74 - 0.72*slope + 0.3*(h - 0.6);
        vec3 col = mix(mix(Lo, Sh, 0.45), mix(Sh, Wh*0.96, smoothstep(0.42, 0.9, lit)), smoothstep(0.08, 0.46, lit));
        // heaven's light from the blade shines through the bank
        col = min(col + vec3(1.0, 0.92, 0.72)*halo(x, 2.2)*0.2*ch, vec3(1.0));
        res = vec4(mix(res.rgb, col, cov), max(res.a, cov));
      }
    }
  }
  return res;
}
// A feather: long and curved, a fine quill, soft barbs, rocking as it falls; it grows out of nothing at the top of its fall and melts at the end.
vec4 an_feather(vec3 ro, vec3 rd, float px, float k, vec3 e1, vec3 e2){
  float L = uLength;
  float run = uClock*(0.05 + 0.014*h11(k*1.9)) + k*0.137, cyc = floor(run), q = fract(run);
  float env = smoothstep(0.0, 0.12, q)*(1.0 - smoothstep(0.82, 1.0, q));
  if (env <= 0.0) return vec4(0.0);
  float a0 = h11(k*2.3 + cyc), R0 = 1.4 + 2.6*h11(k*5.1 + cyc);
  float y0 = L*(0.3 + 0.7*h11(k*3.7 + cyc));
  vec3 c = around(R0 + 0.5*sin(q*9.0 + k), a0 + 0.08*sin(q*6.0 + k), y0 - 9.0*q);
  float t;
  float d0 = rayPtRaw(ro, rd, c, t);
  float s = 0.52 + 0.3*h11(k*6.3 + cyc);
  if (d0 > s*1.25) return vec4(0.0);
  vec3 w0 = ro + rd*t - c;
  vec2 u = rot(0.9*sin(uClock*0.7 + k*2.1) + k*1.3)*vec2(dot(w0, e1), dot(w0, e2))/s;
  float x = u.x, y = u.y - 0.26*(x*x - 0.3);
  float ext = max(abs(x) - 1.0, 0.0);
  float wid = 0.27*pow(max(1.0 - x*x, 0.0), 0.55)*(1.0 - 0.3*smoothstep(-0.2, 1.0, x));
  float edge = abs(y) - wid*(1.0 - 0.14*smoothstep(0.4, 0.9, vn(vec2(x*9.0 + k, y > 0.0 ? 1.0 : 3.0))));
  float cov = (1.0 - smoothstep(-0.02 - px/s, 0.03 + px/s, edge))*(1.0 - step(0.001, ext));
  float quill = 1.0 - smoothstep(0.0, 0.02 + px/s, abs(y));
  float stem = (1.0 - smoothstep(0.012, 0.03 + px/s, abs(y)))*step(x, -0.8)*step(-1.25, x);
  float barbs = 0.9 + 0.1*sin((x*3.0 - abs(y)*7.0)*18.0);
  vec3 col = mix(Sh, Wh, 0.55 + 0.45*barbs*(1.0 - 0.6*abs(y)/max(wid, 0.01)));
  col = mix(col, Sh, quill*0.4);
  return vec4(col, max(cov, stem*0.7)*env*(1.0 - behindBlade(ro, rd, t)));
}
// The feathers, in the air in front of the bank.
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return o;
  vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
  if (part(1.0) > 0.5 && an_final() > 0.0){
    for (int i = 0; i < 0; i++){
      vec4 f = an_feather(ro, rd, px, float(i), e1, e2);
      if (f.a > 0.0){
        f.a *= an_final();
        // a feather behind a thick cloud shows through it only faintly
        o = vec4(mix(o.rgb, f.rgb, f.a), max(o.a, f.a));
      }
    }
  }
  o.a *= uFx*hiltFade(ro, rd)*tipFade(ro, rd);
  return o;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_e3(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  // the light of heaven coming through the bank, in slow soft shafts

  return col;
}

// Once the clouds have come, a golden halo opens slowly above the point, a large ring lying in the air in three dimensions,
// giving off a little glow. And now and then a small cloud comes away from the bank.
float an_halo(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, 0.4, uAge - 2000.0);
  return smoothstep(10.4, 12.2, uAge);
}
vec3 sig_vol_r24(vec3 ro, vec3 rd, float px){
  if (uFx <= 0.0) return vec3(0.0);
  vec3 acc = vec3(0.0);
  if (part(0.0) > 0.5) acc += 1.3*gxCloudlets(ro, rd, px, 6.0, 7.4, 5.0, vec3(1.0, 0.995, 0.975), vec3(0.72, 0.73, 0.78))*an_final()*hiltFade(ro, rd)*tipFade(ro, rd);
  float on = an_halo();
  if (part(2.0) < 0.5 || on <= 0.0) return acc;
  float RING = 2.5*mix(0.45, 1.0, on*on*(3.0 - 2.0*on));
  float y0 = uLength + 2.0, tm, best = 1e3, ba = 0.0;
  if (raySegRaw(ro, rd, vec3(0.0, y0 - 0.8, 0.0), vec3(0.0, y0 + 0.8, 0.0), tm) > RING + 3.4) return acc;
  vec3 tilt = vec3(0.1*sin(uTime*0.2), 0.0, 0.1*cos(uTime*0.2));
  vec3 prev = vec3(RING, y0 + tilt.x*RING, 0.0);
  for (int i = 1; i <= 28; i++){
    float a = 6.2832*float(i)/28.0, cx = cos(a), sx = sin(a);
    vec3 p = vec3(RING*cx, y0 + RING*(tilt.x*cx + tilt.z*sx), RING*sx);
    float d = raySegRaw(ro, rd, prev, p, tm);
    if (d < best){ best = d; ba = a; }
    prev = p;
  }
  float catch_ = 0.8 + 0.2*sin(ba*2.0 + uTime*0.6);
  float bloom = 1.0 + 1.6*sin(3.1416*clamp(on, 0.0, 1.0));
  return acc + bloom*(vec3(1.0, 0.88, 0.5)*halo(best, max(0.09, px))*1.5 + vec3(1.0, 0.7, 0.24)*(halo(best, 0.42)*0.5 + halo(best, 1.3)*0.16))*catch_*smoothstep(0.0, 0.5, on);
}

// >>> how it answers: as it lights, in a swing, in a clash
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_r24(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float ig = uAge < 2000.0 ? 1.0 - smoothstep(0.3, 3.0, uAge) : 0.0, sw = clamp(uSwing, 0.0, 1.0);
  a += 1.2*gxBurst(ro, rd, px, 7.0, 4.6, 1.0, 0.3, 304.0, vec3(1.0, 0.995, 0.97), vec3(0.8, 0.8, 0.84));
  return base + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

