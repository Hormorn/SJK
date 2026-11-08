// Fondor Mass Driver (its own blade since 3/11/2026; Hyperlane stays as it is in the client): the great
// guns of the Fondor shipyards. The blade is no beam but a gun: two parallel steel rails with an empty
// gap between them and capacitor rungs across it at intervals. Between the rails blue-white slugs of
// plasma are fired again and again from the hand to the point, leaving a fading wake, each lighting the
// rungs as it passes. The rails are charged: blue light runs along their inner faces and thin arcs jump
// across the gap. Every few shots a heavy round goes up and flashes from the muzzle, a burst of light
// and rails shaking.
// @part Steel rails
// @part Plasma slugs
// @part Charged arcs
// @part Rungs
// @part Muzzle flash
// @raycache fast
// @room 9
// @solid 1
// @moments own
// @trail storm
// @palette {"corona.rim_cool":[0.1,0.2,0.6],"corona.rim_hot":[0.45,0.65,1.0],"corona.inner":[0.85,0.92,1.0],"core.white":[1,1,1],"core.fringe_cool":[0.35,0.55,1.0],"core.fringe_hot":[0.8,0.9,1.0],"light.color":[0.4,0.6,1.0],"trail":[0.4,0.6,1.0]}
// @speed 1.0

const vec3 MD_STEEL = vec3(0.62, 0.66, 0.74);
const vec3 MD_DARK = vec3(0.16, 0.17, 0.2);
const vec3 MD_BLUE = vec3(0.2, 0.55, 1.0);
const vec3 MD_WHITE = vec3(0.9, 0.96, 1.0);
const float MD_GAP = 0.34;
const float MD_RAIL = 0.17;

float md_top(){ return (uLength + 0.1)*rxLife(0.0, 0.3, 0.4); }
// slug i's height, its age; the seventh round is the heavy one
float md_slug(float k, out float heavy){
  float cyc = uClock*1.05 + k/3.0;
  float n = floor(cyc), q = fract(cyc);
  heavy = mod(n + k*3.0, 7.0) < 0.5 ? 1.0 : 0.0;
  return q*q*(uLength + 8.0) - 2.0;
}
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  float top = md_top();
  vec3 col = vec3(0.0);
  float x = abs(b.x);
  if (top > 0.15 && b.y < top + 2.0){
    float yc = clamp(b.y, 0.0, top);
    float d = length(vec2(max(x - (MD_GAP + MD_RAIL), 0.0), b.y - yc));
    col = MD_BLUE*(halo(d, 0.16)*0.14 + halo(d, 0.7)*0.04);
    if (part(1.0) > 0.5){
      for (int i = 0; i < 3; i++){
        float heavy;
        float y = md_slug(float(i), heavy);
        float dy = b.y - y;
        float w = dy < 0.0 ? 7.0 : 0.6;
        col += MD_BLUE*halo(abs(dy), w)*halo(x, MD_GAP + 0.6)*(0.42 + 0.6*heavy)*step(y, top + 3.0);
      }
    }
  }
  if (part(4.0) > 0.5){
    float heavy;
    for (int i = 0; i < 3; i++){
      float y = md_slug(float(i), heavy);
      float over = clamp((y - uLength)/6.0, 0.0, 1.0);
      if (heavy > 0.5 && y > uLength - 1.0) col += (MD_WHITE*halo(length(vec2(b.x, b.y - uLength)), 0.5 + 1.8*over)*1.2 + MD_BLUE*halo(length(vec2(b.x, b.y - uLength)), 2.5)*0.5)*(1.0 - over);
    }
  }
  col += lvl(g)*MD_BLUE*(1.0 - smoothstep(0.3, 1.4, abs(b.y)))*0.25;
  col *= 1.0 - smoothstep(uLength + 0.5, uLength + 2.2, b.y);
  return col*roomFade(abs(b.x))*(1.0 + 0.5*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float top = md_top();
  if (top < 0.15 || b.y < -0.2 || b.y > top + 0.1) return o;
  float x = abs(b.x);
  // shake a little when a heavy round fires
  float inner = MD_GAP, outer = MD_GAP + MD_RAIL;
  float rail = fillD(inner - x, min(px, 0.03))*fillD(x - outer, min(px, 0.03));
  vec4 res = o;
  // the slugs and their wake between the rails (solid so they read against any wall)
  if (part(1.0) > 0.5 && x < inner){
    for (int i = 0; i < 3; i++){
      float heavy;
      float y = md_slug(float(i), heavy);
      float dy = b.y - y;
      float size = 0.3 + 0.25*heavy;
      float head = 1.0 - smoothstep(0.0, 1.0, length(vec2(b.x/size, dy/(size*2.2))));
      float wake = (dy < 0.0 ? exp(dy*0.3) : 0.0)*(1.0 - smoothstep(0.0, 0.28, x));
      float a = clamp(head*3.0 + wake*0.8, 0.0, 1.0)*step(y, top + 3.0);
      if (a > 0.0) res = vec4(mix(res.rgb, mix(MD_BLUE*1.1, MD_WHITE*1.25, clamp(head*3.0, 0.0, 1.0)), a), max(res.a, a));
    }
  }
  // the rungs
  if (part(3.0) > 0.5 && x < inner){
    float v = fract(b.y/2.6) - 0.5;
    float rung = 1.0 - smoothstep(0.02, 0.05, abs(v));
    if (rung > 0.0){
      float lit = 0.0;
      for (int i = 0; i < 3; i++){
        float heavy;
        lit += halo(abs(b.y - md_slug(float(i), heavy)), 1.6);
      }
      res = vec4(mix(res.rgb, MD_DARK*2.2 + MD_BLUE*lit*1.3, rung), max(res.a, rung));
    }
  }
  if (rail <= 0.0) return res;
  float across = (x - inner)/MD_RAIL;
  float hi = pow(1.0 - abs(across - 0.45)*2.0, 3.0);
  vec3 col = part(0.0) > 0.5 ? mix(MD_DARK, MD_STEEL, 0.2 + 0.8*clamp(hi, 0.0, 1.0))*(0.92 + 0.14*rxFlow(vec2(x*25.0, b.y*0.2))) : MD_DARK*2.0;
  // the charge along their inner faces, climbing
  col += MD_BLUE*(1.0 - smoothstep(0.0, 0.3, across))*(0.35 + 0.65*pow(0.5 + 0.5*sin(b.y*0.7 - uClock*6.0), 4.0));
  // collars along them
  col = mix(col, MD_STEEL*1.25, 1.0 - smoothstep(0.03, 0.06, abs(fract(b.y/2.6) - 0.5)));
  col = mix(col, MD_WHITE, 0.45*clamp(uClash, 0.0, 1.0));
  return vec4(mix(res.rgb, col, rail), max(res.a, rail));
}
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t;
  // arcs jumping the gap, each a short crooked bridge that lives a moment
  if (part(2.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 3.0){
    for (int i = 0; i < 6; i++){
      float k = float(i);
      float cyc = uClock*(0.9 + 0.4*h11(k*3.1)) + h11(k*5.7);
      float n = floor(cyc), q = fract(cyc);
      if (q > 0.35) continue;
      float y = L*(0.04 + 0.92*h11(n*1.7 + k));
      float a = h11(n*3.3 + k);
      vec3 p0 = around(0.42, a, y), p1 = around(0.42, a + 0.5, y + 0.7*(h11(n*2.1 + k) - 0.5));
      float d = bolt3(ro, rd, p0, p1, n*4.7 + k, 0.2, px);
      fx += (MD_WHITE*halo(d, max(0.035, px))*1.5 + MD_BLUE*halo(d, 0.18)*0.7)*(1.0 - q/0.35);
    }
  }
  // Update 27 (Creyon: too empty): the gun's field wound round the rails, its charge leaping off them, heat
  // and sparks shed along the barrel and a wake of light behind each round
  if (fxNear(ro, rd, 0.0, L) < 8.0){
    float aft = rxLife(0.2, 0.7, 0.3);
    vec3 add = vec3(0.0);
    if (part(2.0) > 0.5){
      add += 1.3*fxCoils(ro, rd, px, 0.8, L - 0.8, 1.15, 0.35, 0.3, 3.0, MD_BLUE);
      add += 1.3*fxArcs(ro, rd, px, L*0.08, L*0.97, 5.0, 2.8, 0.7, 13.0, MD_BLUE*1.2);
      add += 0.5*syMistSpin(ro, rd, L*0.1, L*0.97, 1.0, 3.2, 0.02, 5.0, MD_BLUE*0.5, MD_DARK);
    }
    if (part(1.0) > 0.5){
      add += 1.3*shStreaks(ro, rd, 1.4, 0.5, L, 10.0, 3.2, 14.0, 0.0, 17.0, MD_WHITE, MD_BLUE);
      add += 1.3*rxBorn(ro, rd, px, 18.0, 0.5, L - 0.5, 0.7, 2.6, -1.3, 0.04, 9.0, MD_WHITE, MD_BLUE);
    }
    fx += add*aft;
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += (MD_WHITE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 0.8)*1.2 + MD_BLUE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 2.4)*0.35)*cl;
  return fx*rxLife(0.2, 0.6, 0.3)*uFx*hiltFade(ro, rd);
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(4.0) > 0.5){
  a += 1.5*gxKin(ro, rd, 8.0, 7.0, 2.40, 0.200, 0.70, 0.08, 0.96, 1.0, 7351.0, vec3(0.850, 0.920, 1.000), vec3(0.450, 0.650, 1.000));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.38, 4.81, -0.012, 653.0, vec3(0.450, 0.650, 1.000)*0.24, vec3(0.100, 0.200, 0.600)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.450, 0.650, 1.000)*wv*(halo(d, 0.9)*0.33 + halo(d, 2.6)*0.20)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(4.0) > 0.5){
    a += 1.3*syPetalsAir(ro, rd, 10.0, 3.04, 0.051, 1139.0, vec3(0.850, 0.920, 1.000), vec3(0.450, 0.650, 1.000));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  if (part(4.0) > 0.5){
    col += vec3(0.0);
  }
  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.42*gxKin(ro, rd, 3.0, 8.0, 2.00, 0.200, 0.20, 0.08, 0.96, 1.0, 9119.0, vec3(0.850, 0.920, 1.000), vec3(0.450, 0.650, 1.000));
  a += 1.2*gxHelix(ro, rd, px, 16.0, 2.2, 1.2, 0.08, 58.0, vec3(0.850, 0.920, 1.000), vec3(0.450, 0.650, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

