// Rodia Toxic Bloom (rebuilt 2/11/2026, update 22; Creyon: this one is bad): the swamps of Rodia in one
// blade. The blade is a thick poison-green stalk, ribbed and wet, purple veins climbing it. Along it grow
// pods that swell slowly, bulging out of the stalk, glowing from within until they split and let go a
// cloud of yellow-green spores, then wither back to grow again. Sticky drops hang from the stalk and
// the spores drift off slowly, twinkling.
// @part Living stalk
// @part Swelling pods
// @part Spore clouds
// @part Drifting spores
// @raycache fast
// @trail plague
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.3,0.55,0.0],"corona.rim_hot":[0.7,1.0,0.1],"corona.inner":[0.95,1.0,0.65],"core.white":[1.0,1.0,0.85],"core.fringe_cool":[0.45,0.75,0.05],"core.fringe_hot":[0.85,1.0,0.35],"light.color":[0.65,1.0,0.1],"trail":[0.65,1.0,0.1]}
// @speed 1.0

const vec3 RB_GREEN = vec3(0.3, 0.72, 0.1);
const vec3 RB_DARK = vec3(0.04, 0.14, 0.03);
const vec3 RB_PURPLE = vec3(0.55, 0.12, 0.7);
const vec3 RB_YELLOW = vec3(0.92, 1.0, 0.25);
const vec3 RB_PALE = vec3(0.9, 1.0, 0.7);

float rb_top(){ return (uLength + 0.2)*rxLife(0.0, 0.6, 0.5); }
// pod k: its height, side, how swollen (0 to 1), and whether it has burst (burst: 0 to 1 afterwards)
vec3 rb_pod(float k, out float burst){
  float cyc = uClock*(0.085 + 0.03*h11(k*2.3)) + h11(k*7.1);
  float n = floor(cyc), q = fract(cyc);
  float y = uLength*(0.1 + 0.8*fract(k*0.618 + 0.2));
  float side = mod(k, 2.0) < 0.5 ? -1.0 : 1.0;
  float swell = smoothstep(0.0, 0.62, q)*(1.0 - smoothstep(0.66, 0.74, q)*0.8)*(1.0 - smoothstep(0.86, 1.0, q)*0.2);
  burst = q > 0.64 ? clamp((q - 0.64)/0.3, 0.0, 1.0) : -1.0;
  return vec3(y, side, swell);
}
float rb_half(float y0, float top, float xSign){
  float y = clamp(y0, 0.0, top);
  float w = 0.46 + 0.07*sin(y*2.4) + 0.05*sin(y*0.7 + uClock*0.5);
  if (part(1.0) > 0.5){
    for (int i = 0; i < 6; i++){
      float burst;
      vec3 p = rb_pod(float(i), burst);
      if (p.y*xSign > 0.0) w += p.z*0.75*exp(-(y - p.x)*(y - p.x)/1.2);
    }
  }
  return rxHalf(y, top, w, 1.0);
}
vec3 sig_glow_tc(vec3 g, vec2 b, float r, float px){
  float top = rb_top();
  vec3 col = vec3(0.0);
  if (top > 0.15){
    float yc = clamp(b.y, 0.0, top);
    float d = max(length(vec2(b.x, b.y - yc)) - rb_half(yc, top, b.x), 0.0);
    col = RB_GREEN*(halo(d, 0.16)*0.28 + halo(d, 0.8)*0.06);
    if (part(2.0) > 0.5 && uFx > 0.0){
      // the clouds each pod lets go, soft and spreading, thinning away
      for (int i = 0; i < 6; i++){
        float burst;
        vec3 p = rb_pod(float(i), burst);
        if (burst < 0.0) continue;
        vec2 at = vec2(p.y*(0.7 + 1.5*burst), p.x + 2.2*burst);
        float n = rxFlow((b - at)*0.9 + vec2(float(i)*3.0, uClock*0.3));
        col += mix(RB_YELLOW, RB_GREEN, burst)*halo(length(b - at), 0.5 + 1.9*burst)*(0.5 + 0.7*n)*(1.0 - burst)*(1.0 - burst)*1.15*uFx;
      }
    }
  }
  col += lvl(g)*RB_GREEN*(1.0 - smoothstep(0.3, 1.4, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid_raw(vec4 o, vec2 b, float px){
  float top = rb_top();
  if (top < 0.15 || b.y < -0.2 || b.y > top + 0.2) return o;
  float half = rb_half(b.y, top, b.x);
  float x = abs(b.x);
  float cov = fillD(x - half, min(px, 0.05))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(x/max(half, 0.01), 0.0, 1.0);
  float nz = sqrt(max(1.0 - u*u, 0.0));
  float ribs = 0.5 + 0.5*sin(b.x*13.0 + 1.2*sin(b.y*0.5));
  float wet = pow(max(dot(normalize(vec3(b.x/max(half, 0.01), 0.0, nz + 0.1)), normalize(vec3(-0.5, 0.2, 0.8))), 0.0), 14.0);
  vec3 col = mix(RB_DARK, RB_GREEN, (0.25 + 0.75*nz)*(0.55 + 0.45*ribs));
  // purple veins climbing
  float v = 1.0 - smoothstep(0.0, 0.06, abs(rxFlow(vec2(b.x*2.6 + 3.0, b.y*0.36 - uClock*0.12)) - 0.5));
  col = mix(col, RB_PURPLE, v*0.85);
  // the pods glowing from within as they swell
  if (part(1.0) > 0.5){
    for (int i = 0; i < 6; i++){
      float burst;
      vec3 p = rb_pod(float(i), burst);
      if (p.y*b.x <= 0.0) continue;
      float dd = exp(-(b.y - p.x)*(b.y - p.x)/1.2)*smoothstep(0.3, 1.0, u);
      col = mix(col, mix(RB_PURPLE*0.9, RB_YELLOW*1.15, p.z*p.z)*(0.7 + 0.3*sin(b.y*5.0)), dd*p.z);
    }
  }
  col += RB_PALE*wet*0.7;
  col += RB_PALE*halo(max(b.y, 0.0), 0.4)*0.25;
  col = mix(col, RB_PALE, 0.45*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_tc(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  if (uFx <= 0.0) return fx;
  float L = uLength, t;
  float aft = rxLife(0.3, 1.0, 0.4);
  if (part(3.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 6.0){
    for (int i = 0; i < 26; i++){
      float k = float(i);
      float cyc = uClock*(0.06 + 0.04*h11(k*2.9)) + h11(k*7.3);
      float n = floor(cyc), q = fract(cyc);
      vec3 p = around(0.9 + 3.6*sqrt(q)*(0.5 + 0.5*h11(n*3.1 + k)), h11(n*5.3 + k) + 0.2*q, L*(0.05 + 0.86*h11(n*1.9 + k)) + 3.0*q + 0.25*sin(uClock*0.9 + k));
      float d = rayPt(ro, rd, p, t);
      float tw = 0.7 + 0.3*sin(uClock*2.3 + k*2.0);
      fx += (RB_YELLOW*halo(d, max(0.06, px))*1.2 + RB_GREEN*halo(d, 0.24)*0.28)*sin(3.1416*q)*tw*roomAt(p);
    }
  }
  float cl = clamp(uClash, 0.0, 1.0);
  if (cl > 0.01) fx += (RB_PALE*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 0.8) + RB_YELLOW*halo(rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), t), 2.4)*0.3)*cl;
  return fx*aft*uFx*hiltFade(ro, rd);
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
const vec3 SX_HOT = vec3(0.700, 1.000, 0.100);
const vec3 SX_DEEP = vec3(0.300, 0.550, 0.000);
const vec3 SX_PALE = vec3(0.950, 1.000, 0.650);
vec3 sig_vol_t1(vec3 ro, vec3 rd, float px){
  vec3 fx = sig_vol_34(ro, rd, px);
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, uLength) > 16.0) return fx;
  float L = uLength, C = uClock;
  vec3 a = vec3(0.0);
  a += sxDrops(ro, rd, px, 8.0, 1.3, 0.07, 308.0, SX_PALE, SX_HOT);a += 1.0*gxKin(ro, rd, 5.0, 6.0, 2.60, 0.050, 0.45, 0.08, 0.96, 1.0, 7949.0, SX_PALE, SX_HOT);
  return fx + a*rxLife(0.35, 1.3, 0.35)*uFx*tipFade(ro, rd)*hiltFade(ro, rd);
}
// <<< update 34
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_t1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  if (part(3.0) > 0.5){
  a += 1.5*shFlakes(ro, rd, 3.71, 0.5, L - 0.5, 15.2, 2.9, -0.37, 0.033, 0.11, 0.55, 0.0, 521.0, vec3(0.950, 1.000, 0.650), vec3(0.700, 1.000, 0.100));
  a += 0.95*syMistSpin(ro, rd, 0.3, L*0.96, 1.36, 5.18, 0.013, 524.0, vec3(0.700, 1.000, 0.100)*0.24, vec3(0.300, 0.550, 0.000)*0.3);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float wv = pow(0.5 + 0.5*sin(b.y*0.22 - uClock*0.70), 3.0);
  float d = max(abs(b.x) - 0.5, 0.0);
  return sig_glow_t1(g, b, r, px) + vec3(0.700, 1.000, 0.100)*wv*(halo(d, 0.9)*0.28 + halo(d, 2.6)*0.17)*rxLife(0.3, 1.2, 0.35)*uFx*tipFadeB(b)*smoothstep(0.0, 4.0, b.y)*roomFade(abs(b.x));
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(3.0) > 0.5){
    a += 1.3*syPetalsAir(ro, rd, 8.0, 4.19, 0.059, 1477.0, vec3(0.950, 1.000, 0.650), vec3(0.700, 1.000, 0.100));
    a += 1.4*vec3(0.0);
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  a += 1.1*gxTumble(ro, rd, px, 9.0, 4.0, 2.0, 1.0, 0.26, 81.0, vec3(0.950, 1.000, 0.650), vec3(0.700, 1.000, 0.100));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

