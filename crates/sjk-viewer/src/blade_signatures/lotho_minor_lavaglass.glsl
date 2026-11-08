// Lotho Minor Lavaglass (new 2/11/2026, update 22, in place of Lotho Minor Furnace; Lotho Minor is a junk
// world whose fires never go out): a tall tube of dark glass with molten rock in it, a lamp of lava. Soft
// round blobs of orange-gold melt rise slowly up it, stretch, join when they touch and part again,
// cool to deep red at the top and sink back. The glass is clear, a thin bright line down its edge,
// and heat shimmers off it.
// @part Dark glass
// @part Rising melt
// @part Hot base
// @part Heat
// @raycache fast
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.6,0.15,0.0],"corona.rim_hot":[1.0,0.5,0.1],"corona.inner":[1.0,0.8,0.5],"core.white":[1.0,0.93,0.8],"core.fringe_cool":[0.85,0.3,0.05],"core.fringe_hot":[1.0,0.65,0.25],"light.color":[1.0,0.45,0.1],"trail":[1.0,0.45,0.1]}
// @speed 1.0

const vec3 LG_GLASS = vec3(0.03, 0.02, 0.03);
const vec3 LG_RED = vec3(0.8, 0.09, 0.02);
const vec3 LG_ORANGE = vec3(1.0, 0.42, 0.05);
const vec3 LG_GOLD = vec3(1.0, 0.8, 0.3);

float lg_top(){ return (uLength + 0.2)*rxLife(0.0, 0.5, 0.4); }
float lg_half(float y0, float top){ return rxHalf(clamp(y0, 0.0, top), top, 0.72, 1.0); }
// the melt's field at b: above 1 inside a blob; `height` how far up the hottest one is
float lg_melt(vec2 b, out vec3 tint){
  float L = uLength, f = 0.0;
  tint = vec3(0.0);
  for (int i = 0; i < 9; i++){
    float k = float(i);
    float cyc = uClock*(0.05 + 0.025*h11(k*2.3)) + h11(k*6.7);
    float q = fract(cyc);
    // up slowly and down slowly, a little off its path
    float tri = 1.0 - abs(2.0*q - 1.0);
    float y = L*(tri*tri*(3.0 - 2.0*tri))*0.98 + 0.2;
    float x = 0.28*sin(uClock*0.45 + k*2.3) + 0.12*sin(y*0.5 + k);
    float R = 0.34 + 0.2*h11(k*3.1);
    vec2 d = (b - vec2(x, y))/vec2(1.0, 1.7 + 0.5*h11(k*8.3));
    float fi = R*R/(dot(d, d) + 0.02);
    f += fi;
    // (update 30, Creyon) under an RGB each blob has its own random colour, new at each trip up
    tint += hsv(h11(k*13.7 + floor(cyc)*3.1) + 0.25*uTurn, 0.9, 1.0)*fi;
  }
  return f;
}
vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = lg_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float yc = clamp(b.y, 0.0, top);
    float d = max(length(vec2(b.x, b.y - yc)) - lg_half(yc, top), 0.0);
    float s = b.y/max(uLength, 1.0);
    col = mix(LG_ORANGE, LG_RED, s)*(halo(d, 0.14)*0.34 + halo(d, 0.7)*0.08);
    if (part(3.0) > 0.5 && uFx > 0.0){
      float w = rxFlow(vec2(b.x*2.4, b.y*0.5 - uClock*0.9));
      col += LG_ORANGE*smoothstep(0.55, 0.95, w)*halo(d, 1.2)*smoothstep(0.08, 0.4, d)*0.2*uFx;
    }
  }
  col += lvl(g)*LG_ORANGE*(1.0 - smoothstep(0.3, 1.4, abs(b.y)))*0.25;
  col *= 1.0 - smoothstep(uLength + 0.2, uLength + 1.5, b.y);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = lg_top();
  if (top < 0.15 || b.y < -0.2 || b.y > top + 0.2) return o;
  float half = lg_half(b.y, top);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.03))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float s = clamp(b.y/max(uLength, 1.0), 0.0, 1.0);
  bool turned = uTurn != 0.0;
  vec3 hue = hsv(uTurn, 0.88, 1.0);
  if (turned) uOwn = 1.0;
  vec3 lgRed = turned ? hue*0.8 : LG_RED, lgOrange = turned ? hue : LG_ORANGE, lgGold = turned ? mix(hue, vec3(1.0), 0.55) : LG_GOLD;
  vec3 col = part(0.0) > 0.5 ? LG_GLASS + lgRed*0.1*(1.0 - s) : lgRed*0.15;
  // the light of the base climbing through the liquid
  if (part(2.0) > 0.5) col += lgOrange*exp(-b.y*0.28)*0.8;
  if (part(1.0) > 0.5){
    vec3 tint;
    float f = lg_melt(vec2(clamp(b.x/max(half, 0.01), -1.0, 1.0)*0.72, b.y), tint);
    float m = smoothstep(0.85, 1.25, f);
    float hot = smoothstep(1.2, 3.4, f);
    vec3 melt = mix(mix(LG_ORANGE, LG_RED, smoothstep(0.35, 1.0, s)), LG_GOLD*1.15, hot);
    if (turned){
      tint /= max(f, 0.001);
      melt = mix(tint*mix(1.0, 0.72, smoothstep(0.35, 1.0, s)), mix(tint, vec3(1.0), 0.5)*1.15, hot);
    }
    col = mix(col, melt, m);
    col += (turned ? hue*0.7 : LG_RED)*smoothstep(0.45, 0.9, f)*(1.0 - m)*0.5;
  }
  // the glass: a thin bright line down one side, a soft edge on the other
  col += vec3(0.9)*halo(abs(b.x/max(half, 0.01) + 0.62), 0.05)*0.55*(1.0 - smoothstep(0.0, 1.0, u)*0.2);
  col += lgGold*smoothstep(0.88, 1.0, u)*0.3;
  col = mix(col, lgGold, 0.45*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  float t;
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl <= 0.01) return vec3(0.0);
  return (LG_GOLD*halo(rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), t), 0.8)*1.2 + LG_ORANGE*halo(rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), t), 2.4)*0.35)*cl*hiltFade(ro, rd);
}
// (update 26) the body starts softly in the emitter, no hard end above the hilt
vec4 sig_solid(vec4 o, vec2 b, float px){
  vec4 r = sig_solid_raw(o, b, px);
  float j = smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, r.rgb, j), mix(o.a, r.a, j));
}
// (update 29, Creyon: nothing past the tip) every effect ends at the blade's end
vec3 sig_vol_34(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_tc(ro, rd, px);
  float s;
  raySeg(ro, rd, vec3(0.0), vec3(0.0, uLength + 4.0, 0.0), s);
  return fx*tipFade(ro, rd);
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(1.000, 0.500, 0.100);
const vec3 SX_DEEP = vec3(0.600, 0.150, 0.000);
const vec3 SX_PALE = vec3(1.000, 0.800, 0.500);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += sxDrops(ro, rd, px, 6.0, 1.0, 0.05, 311.0, SX_PALE, SX_HOT);a += 0.9*xPuffs(ro, rd, 1.9, 1.3, 1.0, C*0.01, L*0.06, L*0.96, 0.46, 361.0, SX_PALE, SX_HOT*0.5);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.6*gxKin(ro, rd, 2.0, 6.0, 1.80, 0.100, 0.25, 0.08, 0.96, -1.0, 7624.0, vec3(1.000, 0.800, 0.500), vec3(1.000, 0.500, 0.100));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.32, 4.33, -0.012, 857.0, vec3(1.000, 0.500, 0.100)*0.24, vec3(0.600, 0.150, 0.000)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.500, 0.100)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(3.0) > 0.5){
    a += 1.4*gxKin(ro, rd, 1.0, 8.0, 2.60, 0.120, 0.14, 0.08, 0.96, 1.0, 7637.0, vec3(1.000, 0.800, 0.500), vec3(1.000, 0.500, 0.100));
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
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  a += 1.1*vec3(0.0);
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

