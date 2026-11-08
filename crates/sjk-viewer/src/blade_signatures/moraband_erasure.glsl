// Moraband Erasure: the power to erase, as the old Sith Lords dreamed it on Moraband.
// Its own body is a window into erased space: a slim blade of black-violet void full of tiny drifting
// stars and slow swirls of nebula, its edges burning away like paper held to a flame, glowing
// violet-white where they burn, flecks of it streaming up and vanishing. Round it stands a calm,
// violet haze with long ribbons of violet and magenta light sweeping round it, and dark ribbons of
// void smoke curl slowly up it.
// Its colour drifts slowly from violet through magenta to a deep blue and back. Every 22 seconds
// the erasure: a wave of destruction climbs the blade from the emitter, the void inside it blazing
// white-violet as it passes, the aura flaring tall, and all along the blade a storm of glowing
// motes bursts outward and dissolves into nothing; then it settles. It forms from motes of light
// drawn in from all round and goes out from the point down, burning away into dust. Swung, it
// sheds a wake of burning dust. A clash flares violet and scatters motes. Its trail is the
// erasure (`@trail erasure`). No figures, no rings.
// @part Erased space
// @part Burning edges
// @part Violet ribbons
// @part Void smoke
// @part The erasure
// @part Unmade space
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail erasure
// @palette {"corona.rim_cool":[0.2,0.0,0.4],"corona.rim_hot":[0.6,0.2,1.0],"corona.inner":[0.85,0.7,1.0],"core.white":[0.95,0.9,1.0],"core.fringe_cool":[0.4,0.1,0.8],"core.fringe_hot":[0.75,0.5,1.0],"light.color":[0.55,0.2,0.95],"trail":[0.55,0.2,0.95]}
// @speed 2.0

const vec3 ME_VIOLET = vec3(0.55, 0.18, 0.95);
const vec3 ME_MAGENTA = vec3(0.85, 0.15, 0.75);
const vec3 ME_BLUE = vec3(0.2, 0.2, 0.95);
const vec3 ME_DEEP = vec3(0.16, 0.0, 0.32);
const vec3 ME_VOID = vec3(0.03, 0.0, 0.06);
const vec3 ME_LILAC = vec3(0.85, 0.72, 1.0);
const vec3 ME_WHITE = vec3(0.97, 0.94, 1.0);

float me_life(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, 1.3, uAge - 2000.0);
  return smoothstep(0.05, 1.0, uAge);
}
float me_after(){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, 0.6, uAge - 2000.0);
  return smoothstep(0.8, 2.0, uAge);
}
float me_top(){ return (uLength + 0.4)*me_life(); }
float me_flow(vec2 p){
  float v = sin(p.x + 1.5*sin(p.y*0.8 + 0.6));
  v += 0.8*sin(p.y*1.2 + 1.3*sin(p.x*1.3 + 2.0));
  v += 0.5*sin((p.x + p.y)*1.9 + 1.1*sin(p.x*2.2 - p.y*0.6 + 1.4));
  return 0.5 + v/4.6;
}
// its colour, drifting slowly from violet through magenta to deep blue and back
vec3 me_hue(float y){
  float w = 0.5 + 0.5*sin((uTime*0.4)*0.12 + y*0.05 + uSeed*4.0);
  float w2 = 0.5 + 0.5*sin((uTime*0.4)*0.07 + 1.7 + uSeed*2.0);
  return mix(mix(ME_VIOLET, ME_MAGENTA, smoothstep(0.55, 1.0, w)), ME_BLUE, smoothstep(0.6, 1.0, w2)*0.7);
}
// The erasure on its 22-second cycle: where its wave is up the blade (units, below -5 when none)
// and how far its burst of motes has gone (0 to 1, below 0 when none).
float me_wave(out float burst){
  burst = -1.0;
  if (part(4.0) < 0.5) return -10.0;
  float c = mod((uTime*0.5) + uSeed*5.0, 22.0);
  float on = me_after();
  if (on < 0.5) return -10.0;
  if (c >= 15.0 && c < 18.5) burst = (c - 15.0)/3.5;
  float rise = clamp((c - 12.0)/3.2, 0.0, 1.0);
  rise = rise*rise*(3.0 - 2.0*rise);
  return (c > 12.0 && c < 15.6) ? rise*(uLength + 2.0) - 1.0 : -10.0;
}
float me_half(float y, float top){
  float s = clamp(y/max(uLength, 1.0), 0.0, 1.0);
  float w = 0.6 + 0.1*smoothstep(0.0, 0.15, s) - 0.08*s;
  float tip = clamp((top - y)/3.2, 0.0, 1.0);
  return w*sqrt(tip);
}
float me_out(vec2 b, float top, out float half){
  float y = clamp(b.y, 0.0, max(top, 0.0));
  half = me_half(y, top);
  return length(vec2(max(abs(b.x) - half, 0.0), b.y - y));
}

// effect 3: the god aura, tall curtains of light standing round the blade, wavering slowly
// upward like an aurora's (smooth sheets, never flame), flaring tall in the erasure
vec3 me_aura(vec2 b, float d, float top, float flare){
  if (d > 4.0) return vec3(0.0);
  float sheet = me_flow(vec2(b.x*1.1 + 0.6*sin(b.y*0.08 + (uTime*0.4)*0.25), b.y*0.05 - (uTime*0.4)*0.18));
  float folds = pow(0.5 + 0.5*sin(b.x*2.4 + 3.0*sheet + (uTime*0.4)*0.4), 3.0);
  float reach = (1.1 + 1.3*sheet)*(1.0 + 0.5*flare);
  float body = (1.0 - smoothstep(0.3*reach, reach, d))*smoothstep(0.1, 0.8, d + 0.3);
  float rise = 0.7 + 0.3*sin(b.y*0.12 - (uTime*0.4)*0.6 + sheet*3.0);
  float span = smoothstep(-0.5, 2.5, b.y)*(1.0 - smoothstep(top - 1.0 + 3.0*flare, top + 2.0 + 4.0*flare, b.y));
  vec3 hue = me_hue(b.y);
  return (hue*folds*0.85 + ME_DEEP*0.55 + ME_LILAC*folds*folds*0.25)*body*rise*span*(1.0 + 0.8*flare);
}

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = me_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float half;
    float d = me_out(b, top, half);
    float burst;
    float wy = me_wave(burst);
    float flare = burst >= 0.0 ? sin(3.1416*min(burst*1.4, 1.0)) : 0.0;
    vec3 hue = me_hue(b.y);
    col = hue*halo(d, 0.5)*0.85 + ME_DEEP*halo(d, 1.5)*0.65 + ME_LILAC*halo(d, 0.15)*0.45;
    if (part(2.0) > 0.5 && uFx > 0.0) col += me_aura(b, d, top, flare)*me_after()*uFx;
    // the erasure's wave: the light round the blade swallowed below it, a soft blaze at it
    if (wy > -5.0){
      col *= 1.0 - 0.45*smoothstep(0.0, 4.0, wy - b.y);
      col += (ME_WHITE*halo(abs(b.y - wy), 1.8)*halo(d, 1.4)*0.8 + hue*halo(abs(b.y - wy), 3.5)*halo(d, 2.8)*0.6);
    }
  }
  col += lvl(g)*ME_VIOLET*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.6;
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core_d(vec3 c, vec2 b, float px){
  return c*vec3(0.7, 0.5, 1.0)*0.3*(1.0 - smoothstep(0.3, 2.0, b.y));
}

// effects 1 and 2: the blade, a window into erased space (black-violet void with tiny drifting
// stars and slow nebula swirls), its edges burning away like paper, glowing where they burn
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = me_top();
  if (top < 0.15 || b.y < -0.3 || b.y > top + 0.5) return o;
  float half;
  me_out(b, top, half);
  float x = abs(b.x);
  // the burning edge: eaten back softly by a slow field, more as it goes out
  float eat = 0.0;
  if (part(1.0) > 0.5){
    float f = me_flow(vec2(b.y*0.9 - (uTime*0.4)*0.35 + clamp(b.x*2.0, -1.0, 1.0)*7.0, x*2.0 + (uTime*0.4)*0.2));
    eat = smoothstep(0.45, 0.95, f)*0.3*half;
  }
  if (uAge >= 2000.0) eat += half*0.5*smoothstep(0.0, 1.0, uAge - 2000.0);
  float edge = half - eat;
  float cov = fillD(x - edge, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(edge, 0.01), 0.0, 1.0);
  float burst;
  float wy = me_wave(burst);
  vec3 hue = me_hue(b.y);
  vec3 col = mix(ME_VOID, ME_DEEP, smoothstep(0.55, 0.95, u));
  if (part(0.0) > 0.5){
    // slow swirls of nebula drifting in the void
    float neb = me_flow(vec2(b.x*2.2 + 1.0, b.y*0.22 - (uTime*0.4)*0.12));
    col += hue*smoothstep(0.55, 0.95, neb)*0.35*(1.0 - u*0.5);
    // tiny stars drifting slowly up in it (round points, each in its own cell, kept inside it)
    vec2 g = vec2(b.x*5.0, b.y*1.6 - (uTime*0.4)*0.35);
    vec2 c0 = floor(g);
    vec2 ctr = c0 + 0.25 + 0.5*h22(c0);
    float star = halo(length(g - ctr), 0.07)*step(0.62, h21(c0 + 3.0));
    float tw = 0.6 + 0.4*sin((uTime*0.4)*2.5 + h21(c0)*30.0);
    col += ME_WHITE*star*tw*0.9*(1.0 - smoothstep(0.7, 1.0, u));
  }
  // the burning rim, violet-white
  col += mix(hue, ME_WHITE, 0.4)*smoothstep(0.75, 1.0, u)*(0.6 + 1.8*eat/max(half*0.3, 0.01));
  // the erasure blazing white-violet through the void as its wave passes
  if (wy > -5.0) col += mix(ME_WHITE, hue, 0.3)*halo(abs(b.y - wy), 1.4)*1.3;
  col += ME_LILAC*halo(top - b.y, 0.9)*0.4 + ME_WHITE*halo(max(b.y, 0.0), 0.9)*0.7;
  col = mix(col, ME_WHITE, 0.5*clamp(uClash, 0.0, 1.0));
  float a = cov;
  return vec4(mix(o.rgb, col, a), max(o.a, a));
}

// effect 2 (and 9): flecks of the burning edge streaming up off the blade and vanishing; swung,
// a thicker wake of them
vec3 me_dust(vec3 ro, vec3 rd, float px, float wake){
  vec3 acc = vec3(0.0);
  float L = uLength;
  if (fxNear(ro, rd, 0.0, L + 6.0) > 7.5) return acc;
  float t;
  for (int i = 0; i < 26; i++){
    float k = float(i);
    float more = step(18.0, k);
    if (more > 0.5 && wake < 0.01) continue;
    float cyc = (uTime*0.4)*(0.2 + 0.12*h11(k*2.3) + 0.3*more) + h11(k*7.1);
    float n = floor(cyc), q = fract(cyc);
    float a = h11(n*3.7 + k*0.9);
    float y0 = L*(0.05 + 0.92*h11(n*5.1 + k));
    vec3 p = around(0.6 + 1.2*q + 0.5*q*q, a + 0.1*q, y0 + 4.5*q*q + 0.8*q);
    float d = rayPt(ro, rd, p, t);
    float life = smoothstep(0.0, 0.05, q)*(1.0 - smoothstep(0.35, 1.0, q))*mix(1.0, wake, more);
    float size = 0.06 + 0.05*h11(k*4.4);
    vec3 hue = me_hue(p.y);
    acc += (mix(ME_WHITE, hue, q)*halo(d, max(size, px))*1.6 + hue*halo(d, size*4.0)*0.35)*life*roomAt(p);
  }
  return acc;
}

// effect 4: void smoke, three dark ribbons of violet smoke curling slowly up round the blade
// (smooth curves of many pieces, wide and soft), drawn as a deep glow
vec3 me_smoke(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float L = uLength;
  if (fxNear(ro, rd, 0.0, L) > 16.0) return acc;
  float s;
  for (int k = 0; k < 3; k++){
    float fk = float(k);
    float bWs = 0.0;
    float best = 1e3, bf = 0.0;
    vec3 prev = vec3(0.0);
    for (int i = 0; i <= 18; i++){
      float f = float(i)/18.0;
      float y = mix(0.8, L*0.95, f);
      float R = 1.6 + 0.7*sin(f*4.5 - (uTime*0.4)*0.5 + fk*2.1) + 0.3*sin(f*9.0 + (uTime*0.4)*0.3 + fk);
      vec3 p = around(R, fk/3.0 + f*0.7 + (uTime*0.4)*0.04, y);
      if (i > 0){
        float d = raySeg(ro, rd, prev, p, s);
        { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(i) - 1.0 + s)/18.0 - bf)*wN/bWs; }
      }
      prev = p;
    }
    float w = 0.35 + 0.6*bf;
    float ends = smoothstep(0.0, 0.12, bf)*(1.0 - smoothstep(0.75, 1.0, bf));
    vec3 hue = me_hue(L*bf);
    acc += (ME_DEEP*halo(best, w*1.4)*0.9 + hue*halo(best, w*0.35)*0.35)*ends;
  }
  return acc;
}

// effect 5: the erasure's burst, a storm of glowing motes bursting outward all along the blade
// and dissolving into nothing
vec3 me_burst(vec3 ro, vec3 rd, float px, float burst){
  if (burst < 0.0) return vec3(0.0);
  vec3 acc = vec3(0.0);
  float L = uLength, t;
  float e = 1.0 - (1.0 - burst)*(1.0 - burst);
  for (int i = 0; i < 28; i++){
    float k = float(i);
    vec3 p = around(0.6 + 6.0*e*(0.5 + 0.5*h11(k*4.1)), h11(k*6.3) + 0.05*e, L*(0.08 + 0.9*h11(k*2.9)) + 2.0*e*(h11(k*1.1) - 0.3));
    float d = rayPt(ro, rd, p, t);
    float life = smoothstep(0.0, 0.05, burst)*(1.0 - smoothstep(0.3, 1.0, burst));
    float size = 0.08 + 0.1*h11(k*1.7)*(1.0 - burst);
    vec3 hue = me_hue(p.y);
    acc += (ME_WHITE*halo(d, max(size, px))*1.5 + hue*halo(d, size*4.0)*0.5)*life*roomAt(p);
  }
  return acc;
}

// effects 6 and 7: formed from motes of light drawn in from all round onto the blade's line;
// going out, it burns away from the point down, its dust rising
vec3 me_forming(vec3 ro, vec3 rd, float px){
  vec3 acc = vec3(0.0);
  float t;
  if (uAge < 1.4){
    float q = clamp(uAge/1.2, 0.0, 1.0);
    float e = q*q*(3.0 - 2.0*q);
    for (int i = 0; i < 24; i++){
      float k = float(i);
      float y = uLength*h11(k*2.9);
      vec3 p = around((5.0 + 3.0*h11(k*4.1))*(1.0 - e), h11(k*6.3) + 0.2*e, y);
      float d = rayPt(ro, rd, p, t);
      float life = smoothstep(0.0, 0.2, q)*(1.0 - smoothstep(0.85, 1.0, q));
      acc += (ME_LILAC*halo(d, max(0.08, px))*1.5 + ME_VIOLET*halo(d, 0.4)*0.4)*life*roomAt(p);
    }
  } else if (uAge >= 2000.0 && uAge < 2002.2){
    float g = uAge - 2000.0;
    for (int i = 0; i < 24; i++){
      float k = float(i);
      float y = uLength*(1.0 - 0.95*h11(k*2.9));
      float since = g - (uLength - y)/uLength*1.3;
      if (since < 0.0 || since > 1.0) continue;
      vec3 p = around(0.5 + 1.5*since, h11(k*6.3) + 0.1*since, y + 3.0*since*since);
      float d = rayPt(ro, rd, p, t);
      float life = smoothstep(0.0, 0.1, since)*(1.0 - since);
      acc += (ME_LILAC*halo(d, max(0.08, px))*1.4 + ME_VIOLET*halo(d, 0.4)*0.4)*life*roomAt(p);
    }
  }
  return acc;
}

// effect 8: the clash, a violet flare scattering motes
vec3 me_clash(vec3 ro, vec3 rd, float px){
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl <= 0.01) return vec3(0.0);
  vec3 at = vec3(0.0, uLength*0.6, 0.0);
  float t;
  if (rayPtRaw(ro, rd, at, t) > 7.0) return vec3(0.0);
  vec3 acc = ME_WHITE*halo(rayPt(ro, rd, at, t), 0.9)*cl*1.2;
  float age = 1.0 - cl;
  for (int i = 0; i < 10; i++){
    float k = float(i);
    vec3 p = at + around(0.6 + 3.5*age, k/10.0 + 0.05, (h11(k*3.3) - 0.5)*3.0*age);
    acc += (ME_LILAC*halo(rayPt(ro, rd, p, t), 0.1)*1.4 + ME_VIOLET*halo(rayPt(ro, rd, p, t), 0.4)*0.4)*cl;
  }
  return acc;
}

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float after = me_after();
  float wake = part(1.0) > 0.5 ? smoothstep(0.2, 0.8, clamp(uSwing, 0.0, 1.0)) : 0.0;
  float burst;
  me_wave(burst);
  if (part(1.0) > 0.5 && after > 0.01) fx += me_dust(ro, rd, px, wake)*after;
  if (part(3.0) > 0.5 && after > 0.01) fx += me_smoke(ro, rd, px)*after;
  fx += me_burst(ro, rd, px, burst);
  fx += me_forming(ro, rd, px) + me_clash(ro, rd, px);
  // round it (1/11/2026, Creyon: a figure showed in the old aurora curtains and it did not look
  // mythical): a violet haze turning slowly round it, and long sweeping ribbons of violet and
  // magenta light rising and curving round the blade, each living about twenty seconds; in the
  // erasure they flare and draw in close
  if (part(2.0) > 0.5 && after > 0.01 && fxNear(ro, rd, 0.0, uLength) < 9.0){
    float flare = burst >= 0.0 ? sin(3.1416*burst) : 0.0;
    float L = uLength;
    fx += (0.5 + 0.5*flare)*shSheath(ro, rd, 1.9, 1.0, L - 0.6, 1.4, 0.08, 1.4, 0.5, 0.0, me_hue(6.0), ME_DEEP)*after;
    for (int j = 0; j < 3; j++){
      float k = float(j);
      float q = fract(tCalm()*0.075 + h11(k*4.3));
      float y0 = L*(0.05 + 0.6*h11(k*2.9 + floor(tCalm()*0.075 + h11(k*4.3))));
      float r = mix(2.3 + 1.5*h11(k*6.7), 1.2, flare);
      vec3 col = mod(k, 2.0) < 0.5 ? me_hue(y0) : ME_MAGENTA;
      fx += (1.1 + 1.0*flare)*sin(3.1416*q)*rxRibbon(ro, rd, r, r + 0.7, 0.33, k/4.0 + 0.0048*tCalm(), y0, min(y0 + L*0.25, L*0.95), 0.42, k + 80.0, ME_WHITE, col)*after;
    }
    // the erased: fragments of the world round it drawn slowly in to the blade, each a soft
    // violet-white glint that brightens as it nears and vanishes into the blade with a spark
    float t;
    for (int i = 0; i < 12; i++){
      float k = float(i);
      float cyc = tCalm()*(0.18 + 0.1*h11(k*2.3)) + h11(k*7.1);
      float n = floor(cyc), q = fract(cyc);
      float a = h11(n*3.7 + k) + 0.15*q;
      float r = mix(4.2 + 0.8*h11(n*2.9 + k), 0.4, q*q);
      vec3 pp = around(r, a, uLength*(0.08 + 0.85*h11(n*5.1 + k)) + 1.2*q);
      float dd = rayPt(ro, rd, pp, t);
      float life = smoothstep(0.0, 0.2, q)*(1.0 - smoothstep(0.9, 1.0, q));
      fx += (ME_WHITE*halo(dd, max(0.05 + 0.05*q, px))*1.4 + me_hue(pp.y)*halo(dd, 0.35)*0.3)*life*roomAt(pp)*after;
    }
  }
  return fx*uFx;
}

// (1/11/2026, update 13, Creyon: no tiny grey symbols in a blade: the cracks in space are gone)


vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  return fx*hiltFade(ro, rd);
}

// effect 6 (1/11/2026, update 14): unmade space, the erasure's mark on the world round it: holes
// of nothing open in the air beside the blade, blacker than the night, their torn edges seething
// with violet static, swallow the light behind them and close again
vec4 x14_moraband_erasure(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  for (int i = 0; i < 4; i++){
    float k = float(i);
    float cyc = uTime/(6.0 + 1.5*h11(k*2.1)) + h11(k*8.3);
    float n = floor(cyc), q = fract(cyc);
    float open = smoothstep(0.0, 0.3, q)*(1.0 - smoothstep(0.65, 1.0, q));
    if (open <= 0.0) continue;
    // tall narrow tears standing clear of the blade, hidden behind it by raySeg
    vec3 c = around(2.6 + 1.6*h11(n*3.1 + k), h11(n*5.9 + k*1.3), uLength*(0.2 + 0.6*h11(n*7.7 + k)));
    float R = (0.32 + 0.3*h11(n*1.9 + k))*open, H = 2.6 + 2.2*h11(n*4.3 + k);
    float s;
    float d = raySeg(ro, rd, c - vec3(0.0, H, 0.0), c + vec3(0.0, H, 0.0), s);
    if (d > R*2.5 + 0.4) continue;
    float taper = sqrt(max(1.0 - (2.0*s - 1.0)*(2.0*s - 1.0), 0.0));
    float rag = max(R*taper*(0.7 + 0.5*vn(vec2(s*7.0 + n*3.0, uTime*0.5 + k))), 0.03);
    float cov = (1.0 - smoothstep(rag*0.45, rag, d))*taper;
    float crackle = smoothstep(0.5, 0.85, vn(vec2(s*14.0 + k*5.0, uTime*3.0 + n)));
    float rim = exp(-pow((d - rag*0.9)/(0.05 + 0.1*R), 2.0))*crackle*taper;
    vec3 col = vec3(0.0, 0.0, 0.008) + vec3(0.8, 0.5, 1.0)*rim*1.3;
    float a = max(cov, rim*0.8);
    o = vec4(mix(o.rgb, col, a), max(o.a, a));
  }
  return o;
}
vec4 sig_vol_solid_34(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx > 0.0 && part(5.0) > 0.5){
    vec4 h = x14_moraband_erasure(ro, rd, px);
    h.a *= uFx*rxLife(0.3, 1.0, 0.6)*hiltFade(ro, rd);
    o = vec4(mix(o.rgb, h.rgb, h.a), max(o.a, h.a));
  }
  return o;
}
// (update 26) the body starts softly in the emitter, no hard end above the hilt
vec4 sig_solid(vec4 o, vec2 b, float px){
  vec4 r = sig_solid_raw(o, b, px);
  float j = smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, r.rgb, j), mix(o.a, r.a, j));
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
const vec3 SX_HOT = vec3(0.600, 0.200, 1.000);
const vec3 SX_DEEP = vec3(0.200, 0.000, 0.400);
const vec3 SX_PALE = vec3(0.850, 0.700, 1.000);
vec3 sig_vol_p1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
   a += 1.0*xDust(ro, rd, 5.0, 0.9, 0.9, C*0.015, L*0.12, L*0.95, 0.35, 32.0, SX_PALE, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
vec3 sig_glow_d(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.55*sxFlames(b, 0.75, 1.5, 1.4, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = sig_vol_solid_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  float L = uLength, C = uClock;
  vec4 n = (vec4(0.0))*rxLife(0.45, 1.3, 0.4)*uFx*hiltFade(ro, rd);
  return vec4(mix(o.rgb, n.rgb, n.a), max(o.a, n.a));
}
// <<< update 34

// >>> dim: less glow, so its effects and body can be seen
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){ return sig_glow_d(g, b, r, px)*0.6; }
vec3 sig_core(vec3 c, vec2 b, float px){ return sig_core_d(c, b, px)*1.0; }
// <<< dim

// >>> update 52: this blade's great effects in the air
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 a = vec3(0.0);
  vec3 base = sig_vol_p1(ro, rd, px);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  for (int i = 0; i < 7; i++){
    float k = float(i);
    float run = C*(0.27 + 0.08*h11(k + 441.0)) + k*0.29, cyc = floor(run), ph = fract(run);
    float y = L*((k + h11(k*2.3 + cyc*1.3))/7.0)*0.86 + 0.3;
    a += 1.15*sin(3.1416*ph)*syLick(ro, rd, px, y, h11(k*5.7 + cyc*0.7 + 441.0) + 0.08*ph, 1.0 + 1.5*h11(k*3.1 + cyc), 5.0 + 4.0*h11(k*4.3 + cyc), 0.62, k*3.0 + cyc + 441.0, vec3(0.95, 0.75, 1.0), vec3(0.55, 0.12, 1.0), vec3(0.12, 0.01, 0.3));
  }
  a += 1.25*xDust(ro, rd, 1.6, 4.2, 1.4, C*0.08, L*0.03, L*0.97, 0.9, 442.0, vec3(1.0, 0.75, 1.0), vec3(0.6, 0.2, 1.0));

  uTurnFx += a*rxLife(0.3, 1.0, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return base;
}
// <<< update 52
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
