// Wayland Gravity Press (Legendary, rebuilt on Sol's glow and core 3/11/2026): The weight hidden under Mount Tantiss on Wayland: a heavy blade of liquid violet metal, its gold veins pressed slowly toward the hand. Torn stones lift off the ground and rise past it, grit pressed down by the weight falls past them and gold dust sinks. Every so often the press comes down, and everything falls.
// @part Pressed metal
// @part Rising stones
// @part Falling grit
// @part Sinking dust
// @part The press
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail press
// @palette {"corona.rim_cool":[0.12,0.08,0.45],"corona.rim_hot":[0.45,0.35,1.0],"corona.inner":[0.8,0.75,1.0],"core.white":[0.95,0.92,1.0],"core.fringe_cool":[0.25,0.18,0.7],"core.fringe_hot":[0.6,0.5,1.0],"light.color":[0.4,0.3,0.95],"trail":[0.4,0.3,0.95]}
// @speed 1.0

const vec3 V = vec3(0.460, 0.270, 1.000);
const vec3 Dk = vec3(0.020, 0.012, 0.060);
const vec3 Au = vec3(1.000, 0.760, 0.380);
const vec3 H = vec3(0.960, 0.930, 1.000);
float wg_top(){ return uLength*rxLife(0.0, 0.8, 0.5); }
float wg_press(){ return part(4.0) > 0.5 ? rxCycle(17.0, 10.0, 12.2, 13.2, 16.2) : 0.0; }
float wg_half(float y){
  float top = wg_top();
  if (y > top) return 0.0;
  float L = max(uLength, 1.0), pr = wg_press();
  float w = mix(0.72, 0.38, smoothstep(0.0, L, y)) + 0.045*sin(y*0.55 + uClock*0.9);
  w *= 1.0 + 0.18*pr*sin(3.1416*clamp(y/L, 0.0, 1.0));
  float e = 1.0 - clamp((top - y)/3.2, 0.0, 1.0);
  return w*sqrt(max(1.0 - e*e, 0.0));
}
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  float top = wg_top(), yc = clamp(b.y, 0.0, top), pr = wg_press();
  float d = max(length(vec2(b.x, b.y - yc)) - wg_half(yc), 0.0);
  vec3 col = g*(0.62 + 0.25*sin(b.y*0.3 + uClock*1.3) + 0.5*pr + 0.7*clamp(uClash, 0.0, 1.0));
  col += V*(halo(d, 0.5)*0.42 + halo(d, 2.0)*(0.17 + 0.2*pr) + halo(d, 4.8)*0.07);
  return col*tipFadeB(b)*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = wg_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  float pr = wg_press();
  vec3 col = Dk*2.0 + V*0.4;
  if (part(0.0) > 0.5){
    // dark liquid metal running toward the hand, gold in its cracks; a slow weight of light presses down it
    float C = uClock;
    float f = rxFlow(vec2(b.x*1.5 + 0.2*sin(b.y*0.2), b.y*0.075 + C*0.17));
    float f2 = rxFlow(vec2(b.x*2.2 + 4.0, b.y*0.1 + C*0.2));
    float s = rxFlow(vec2(b.x*0.9 + 3.0, b.y*0.04 + C*0.1));
    float weight = pow(0.5 + 0.5*sin(b.y*0.42 + C*1.05), 5.0);
    col = mix(Dk*1.6, V*0.62, smoothstep(0.3, 0.8, f));
    col += H*pow(max(nz*(0.5 + 0.5*sin(s*12.0)), 0.0), 7.0)*0.4;
    float vein = max(1.0 - smoothstep(0.0, 0.05 + 0.03*weight, abs(f - 0.5)), 0.75*(1.0 - smoothstep(0.0, 0.03 + 0.02*weight, abs(f2 - 0.5))));
    col += mix(Au, H, pr)*vein*(0.85 + 1.2*pr + 0.7*weight);
    col += V*weight*0.5;
    // the first light going up it as it lights
    col += mix(Au, H, 0.4)*(1.0 - smoothstep(0.0, 3.0, wg_top() - b.y))*(1.0 - rxLife(0.3, 1.2, 0.5))*1.4;
  }
  col *= 0.55 + 0.7*nz;
  col += V*pow(1.0 - nz, 3.0)*0.85;
  col = mix(col, H, 0.45*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
// A stone's place: it never leaves. Each keeps its own slow path round the blade: its own height, its own way round, a long
// lean in and out and a slow rise and sink; they lift out of the hilt as it lights and fall when it goes out, and the press pushes them down.
vec3 wg_stone(float k, out float life, out float R){
  float L = uLength, up = rxLife(0.0, 2.0, 0.6);
  float h1 = h11(k*3.1), h2 = h11(k*7.7), h3 = h11(k*1.7), h4 = h11(k*9.3);
  float dir = mod(k, 3.0) < 1.5 ? 1.0 : -1.0;
  life = 1.0;
  R = 0.38 + 0.62*h11(k*5.3);
  float rad = (2.2 + 2.4*h1 + (0.5 + 0.5*h3)*sin(uClock*(0.1 + 0.1*h3) + k*2.1))*(0.45 + 0.55*up);
  float ang = h2 + dir*uClock*(0.012 + 0.018*h4) + 0.03*sin(uClock*0.17 + k);
  float y = L*(0.07 + 0.82*fract(k*0.618 + 0.13)) + (0.8 + 1.1*h4)*sin(uClock*(0.14 + 0.1*h1) + k*1.3);
  return around(rad, ang, y*up*up - 1.8*wg_press());
}
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.3, 1.4, 0.5);
  if (aft <= 0.01) return fx;
  float pr = wg_press();
  if (part(1.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 7.0){
    float t;
    for (int i = 0; i < 11; i++){
      float k = float(i), life, R;
      vec3 c = wg_stone(k, life, R);
      fx += (V*0.8 + Au*0.3)*halo(rayPt(ro, rd, c, t), R*2.1)*0.42*life;
      // the grit that pours off a stone as it is lifted: a thin gold thread hanging under it
      float tail = 1.5 + 2.2*h11(k*2.3);
      float dt = raySeg(ro, rd, c - vec3(0.0, R*0.5, 0.0), c - vec3(0.0, R*0.5 + tail, 0.0), t);
      fx += Au*(halo(dt, 0.05 + 0.1*t)*0.5*(1.0 - t)*(1.0 - t))*life;
    }
  }
  // grit pressed down by the weight, falling slowly past the stones
  if (part(2.0) > 0.5) fx += (1.0 + 0.6*pr)*syPetalsAir(ro, rd, 10.0, 5.0, 0.03, 170.0, mix(H, Au, 0.35), V*0.5);
  if (part(3.0) > 0.5){
    fx += 1.8*xDust(ro, rd, 3.6, 1.5, 0.7, -C*0.018, L*0.03, L*0.97, 1.1, 171.0, Au, Au*0.5);
    fx += 0.75*syMistSpin(ro, rd, 0.3, L*0.96, 1.2, 5.0, 0.008, 172.0, V*0.3, Dk*5.0);
  }
  if (part(4.0) > 0.5 && pr > 0.01) fx += pr*(0.9*syMist(ro, rd, 0.2, L*0.42, 4.2, 0.5, 173.0, Au*0.3, V*0.25) + 1.3*vec3(0.0));
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx <= 0.0 || part(1.0) < 0.5 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  for (int i = 0; i < 11; i++){
    float k = float(i), life, R;
    vec3 c = wg_stone(k, life, R);
    vec4 r = syRock(ro, rd, px, c, R, k, uClock*0.12, vec3(0.09, 0.07, 0.13), vec3(0.5, 0.44, 0.56), mix(Au, V, 0.25)*1.1);
    r.a *= life;
    if (r.a > o.a) o = r;
  }
  o.a *= uFx*rxLife(0.3, 1.4, 0.5)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return o;
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  if (part(4.0) > 0.5){
    a += 1.3*vec3(0.0);
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
vec3 sig_vol_r24(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  // its great moment: the weight grows until it pulls everything straight in to the blade, then lets it fall away
  float wl = rxCycle(58.0, 22.0, 31.0, 35.0, 43.0);
  if (wl > 0.0){
    for (int i = 0; i < 14; i++){
      float k = float(i), h0 = h11(k*2.7 + 9.0), h1 = h11(k*6.1 + 3.0), h2 = h11(k*9.1 + 6.0);
      float e0 = smoothstep(h0*0.25, 0.75 + h0*0.25, wl);
      vec3 p = around(mix(5.0 + 1.2*h1, 0.8, e0*e0), h2, L*(0.1 + 0.82*h1) - 1.2*(1.0 - e0)*(1.0 - wl));
      a += 1.35*gxPiece(ro, rd, px, p, 0.3 + 0.26*h0, k + 90.0, 0.0, vec3(1.0, 0.86, 0.55), vec3(0.5, 0.3, 0.95))*smoothstep(0.0, 0.18, wl);
    }
    a += wl*1.2*gxRain(ro, rd, px, 14.0, 4.6, 92.0, vec3(1.0, 0.86, 0.55), vec3(0.5, 0.3, 0.95));
    float tm, mid = raySegRaw(ro, rd, vec3(0.0, L*0.08, 0.0), vec3(0.0, L*0.92, 0.0), tm);
    a += (vec3(1.0, 0.82, 0.42)*halo(mid, 0.7)*0.45 + vec3(0.5, 0.3, 0.95)*halo(mid, 2.4)*0.3)*wl*wl;
  }
  a += 1.59*vec3(0.0);
  a += 1.05*gxFlares(ro, rd, px, 4.0, 5.0, 211.0, vec3(0.800, 0.750, 1.000), vec3(0.450, 0.350, 1.000));
  a += 1.25*gxVortex(ro, rd, px, 14.0, 5.8, 11.0, vec3(0.800, 0.750, 1.000), vec3(0.450, 0.350, 1.000));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

// >>> how it answers: as it lights, in a swing, in a clash
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_r24(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float ig = uAge < 2000.0 ? 1.0 - smoothstep(0.3, 3.0, uAge) : 0.0, sw = clamp(uSwing, 0.0, 1.0);
  a += 1.4*gxBurst(ro, rd, px, 11.0, 4.4, 0.0, 0.38, 319.0, vec3(1.00, 0.86, 0.55), vec3(0.50, 0.30, 0.95));
  a += ig*1.1*gxRain(ro, rd, px, 12.0, 4.6, 320.0, vec3(1.00, 0.86, 0.55), vec3(0.50, 0.30, 0.95));
  return base + a*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

