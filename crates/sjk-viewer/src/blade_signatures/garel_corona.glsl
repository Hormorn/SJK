// Garel Corona: the corona of Garel's sun caught round a blade: soft gold fire round it, long
// ribbons of sunlight sweeping round it, motes of light drifting in it and heat rising. No figures, no rings.
// @part Sun corona
// @part Corona fire
// @part Chakra motes
// @part Rising heat
// @raycache fast
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.55,0.341,0.066],"corona.rim_hot":[1,0.682,0.132],"corona.inner":[1,0.81,0.56],"core.white":[1.0,0.95,0.75],"core.fringe_cool":[0.8,0.496,0.096],"core.fringe_hot":[1,1,0.444],"light.color":[1.0,0.62,0.12],"trail":[1.0,0.62,0.12]}
// @speed 2.0

const vec3 C_MAIN = vec3(1.000, 0.620, 0.120);
const vec3 C_ACC = vec3(1.000, 0.250, 0.050);
const vec3 C_HOT = vec3(1.000, 0.950, 0.750);

vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float y = clamp(b.y, 0.0, L);
  float d = length(vec2(b.x, b.y - y));
  vec3 col = vec3(0.0);
  if (part(0.0) > 0.5){
    float fl = rxFlow(vec2(b.x*3.0, b.y*0.5 - tCalm()*2.0));
      col = mix(C_MAIN, C_ACC, fl)*halo(d, 0.5 + 0.25*fl)*1.0 + C_HOT*halo(d, 0.15)*0.5;
  } else {
    col = C_MAIN*halo(d, 0.55)*0.9 + C_HOT*halo(d, 0.15)*0.5;
  }
  col += lvl(g)*C_MAIN*(1.0 - smoothstep(0.3, 2.0, abs(b.y)))*0.4;
  // nothing over the point (Creyon): what it draws fades out just past its tip
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }

vec3 sig_vol_base(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength;
  float aft = rxLife(0.2, 1.0, 0.6);
  if (aft <= 0.01 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  if (part(1.0) > 0.5){
    fx += 1.4*shFire(ro, rd, 1.0, 1.0, L - 0.6, 1.8, 3.0, 1.0, C_ACC*0.4, C_MAIN, C_HOT);
    for (int ri = 0; ri < 4; ri++){
      float k = float(ri);
      float q = fract(tCalm()*0.08 + h11(k*4.7 + 3.0));
      float y0 = uLength*(0.06 + 0.6*h11(k*2.7 + floor(tCalm()*0.08 + h11(k*4.7 + 3.0))));
      float rr = 2.0 + 1.5*h11(k*6.9);
      fx += (1.2)*sin(3.1416*q)*xBeads(ro, rd, rr, rr + 0.6, 0.32, k/4.0 + 0.0048*tCalm(), y0, min(y0 + uLength*0.23, uLength*0.95), 0.4, k + 3.0, C_HOT, C_MAIN);
    }
  }
  if (part(2.0) > 0.5){
    fx += 1.65*rxStarCloud(ro, rd, px, 24.0, 3.2, 0.8, L*0.95, 0.078, 4.0, C_HOT, C_MAIN);
  }
  if (part(3.0) > 0.5){
    fx += 1.05*syMist(ro, rd, 0.5, L*0.95, 3.4, 0.5, 5.0, C_MAIN, C_ACC*0.3);
  }
  return fx*aft*uFx;
}

// ---- update 13: its floating layer (tools/sw_extras.py) ----
const vec3 XM = vec3(1.0, 0.62, 0.12);
const vec3 XH = vec3(1.0, 0.867, 0.692);
const vec3 XA = vec3(1.0, 0.682, 0.132);
const vec3 XD = vec3(0.3, 0.186, 0.036);
vec3 sw_extra(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  for (int ci = 0; ci < 2; ci++){
    float k = float(ci);
    fx += 1.0*rxComet(ro, rd, px, 2.4 + 0.8*k, uLength*(0.12 + 0.22*k), uLength*0.35, 10.0 + 3.0*k, k + 117.0, XH, XA);
  }
  fx += 1.0*shFlakes(ro, rd, 2.4, 1.5, uLength - 0.6, 12.0, 2.8, -0.7, 0.02, 0.16, 0.5, 0.45, 118.0, XH, XA);
  fx += 1.3*vec3(0.0);
  return fx;
}

vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_base(ro, rd, px);
  if (uFx > 0.0 && part(1.0) > 0.5) fx += sw_extra(ro, rd, px)*uFx*rxLife(0.3, 1.0, 0.6);
  return fx*hiltFade(ro, rd);
}
// tip_cap (update 29, Creyon: every tip solid, never see-through): the top of the blade closes into
// an opaque, rounded end in the blade's own colours; light drifts slowly through it. The rest of the
// blade stays as it was.
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength, top = L + 0.2;
  float fade = smoothstep(L - 6.5, L - 2.6, b.y);
  if (fade <= 0.0 || b.y > top) return o;
  float e0 = 1.0 - clamp((top - b.y)/1.14, 0.0, 1.0);
  float half = 0.76*sqrt(max(1.0 - e0*e0, 0.0));
  float cov = fillD(abs(b.x) - half, px)*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(abs(b.x)/max(half, 0.01), 0.0, 1.0);
  float sw = 0.5 + 0.5*sin(b.x*3.4 + b.y*0.55 - uClock*0.8 + 1.6*sin(b.y*0.21 - uClock*0.3));
  vec3 col = mix(vec3(1.00, 0.96, 0.86), vec3(1.00, 0.76, 0.36), smoothstep(0.25, 1.0, u));
  col = mix(col, vec3(0.90, 0.42, 0.10), 0.4*sw*sw*smoothstep(0.1, 0.9, u));
  float a = cov*fade;
  float keep = max(o.a, a);
  return vec4(mix(o.rgb, col, a/max(keep, 0.001)), keep);
}
// (update 29, Creyon: nothing past the tip) every effect ends at the blade's end
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_tc(ro, rd, px);
  float s;
  raySeg(ro, rd, vec3(0.0), vec3(0.0, uLength + 4.0, 0.0), s);
  return fx*tipFade(ro, rd);
}
vec3 sig_glow_34(vec3 g, vec2 b, float r, float px){
  return sig_glow_tc(g, b, r, px)*tipFadeB(b);
}

// >>> update 34: this blade's signature piece
const vec3 SX_HOT = vec3(1.000, 0.682, 0.132);
const vec3 SX_DEEP = vec3(0.550, 0.341, 0.066);
const vec3 SX_PALE = vec3(1.000, 0.810, 0.560);
vec3 sig_glow_t1(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_34(g, b, r, px);
  if (uFx > 0.0) col += (0.4*sxFlames(b, 0.65, 1.2, 1.2, SX_HOT, SX_DEEP*0.5))*rxLife(0.35, 1.3, 0.35)*tipFadeB(b)*uFx;
  return col;
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.6*gxKin(ro, rd, 3.0, 7.0, 2.60, 0.070, 0.18, 0.08, 0.96, 1.0, 7377.0, vec3(1.000, 0.810, 0.560), vec3(1.000, 0.682, 0.132));
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(1.000, 0.682, 0.132)*wv*(halo(d, 0.9)*0.21 + halo(d, 2.6)*0.13)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(3.0) > 0.5){
    a += 1.4*vec3(0.0);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
