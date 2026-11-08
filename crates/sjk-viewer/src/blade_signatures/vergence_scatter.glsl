// Vergence Scatter (rebuilt 3/11/2026): A knot in the Force where the galaxy runs together: the blade is a window into deep space, with lips of blue and gold. Great clouds of nebula turn round it, long loops of light rise and land, far light winds in, and now and then it lets go in bursts of stars.
// @part Deep space
// @part Nebula clouds
// @part Loops of light
// @part Far light
// @part Wormhole
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail nebula
// @palette {"corona.rim_cool":[0.539,0.44,0.248],"corona.rim_hot":[0.98,0.8,0.45],"corona.inner":[1,0.98,0.88],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.784,0.64,0.36],"core.fringe_hot":[1,0.98,0.88],"light.color":[0.98,0.8,0.45],"trail":[0.98,0.8,0.45]}
// @speed 1.0

const vec3 B = vec3(0.190, 0.460, 1.000);
const vec3 D = vec3(0.004, 0.012, 0.050);
const vec3 Au = vec3(1.000, 0.760, 0.350);
const vec3 Vi = vec3(0.620, 0.280, 0.950);
const vec3 H = vec3(1.000, 0.985, 0.920);
float ve_top(){ return uLength*rxLife(0.0, 0.5, 0.32); }
float ve_burst(){ return rxCycle(20.0, 9.0, 11.2, 12.3, 15.3); }
float ve_half(float y){
  float top = ve_top();
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/1.4, 0.0, 1.0);
  return 0.62*sqrt(max(1.0 - e*e, 0.0));
}
float ve_stars(vec2 p, float c, float size){
  vec2 q = p*c, i = floor(q), f = fract(q) - 0.5;
  float h = h11(dot(i, vec2(3.7, 9.1)));
  if (h < 0.84) return 0.0;
  vec2 o = vec2(fract(h*13.0), fract(h*29.0)) - 0.5;
  float tw = 0.65 + 0.35*sin(uClock*(1.0 + h*3.0) + h*40.0);
  return exp(-dot(f - o*0.6, f - o*0.6)/size)*tw;
}
vec3 sig_glow_e1(vec3 g, vec2 b, float r, float px){
  float top = ve_top();
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - ve_half(yc), 0.0);
  float st = ve_burst();
  float tone = rxFlow(vec2(b.y*0.05 - uClock*0.04, 3.0));
  vec3 col = mix(B, Au, smoothstep(0.42, 0.74, tone))*(halo(d, 0.26)*0.9 + halo(d, 1.3)*(0.34 + 0.25*st));
  col += B*halo(d, 4.2)*(0.12 + 0.14*st);
  col += lvl(g)*B*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = ve_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  // a window into deep space: two layers of stars and a nebula, moving at different speeds
  float neb = rxFlow(vec2(b.x*1.1 + 0.3*sin(b.y*0.1), b.y*0.07 - uClock*0.05));
  float neb2 = rxFlow(vec2(b.x*1.7 + 4.0, b.y*0.11 + uClock*0.03));
  vec3 col = D + B*smoothstep(0.35, 0.8, neb)*0.6 + Vi*smoothstep(0.5, 0.85, neb2)*0.45;
  if (part(0.0) > 0.5){
    col += H*(ve_stars(vec2(b.x, b.y - uClock*0.12), 3.0, 0.02)*1.5 + 0.8*ve_stars(vec2(b.x + 3.0, b.y - uClock*0.05), 5.5, 0.03));
  }
  float tone = rxFlow(vec2(b.y*0.05 - uClock*0.04, 3.0));
  float lip = smoothstep(0.72, 1.0, abs(u));
  col = mix(col, mix(mix(B, H, 0.45), mix(Au, H, 0.4), smoothstep(0.42, 0.74, tone)), lip);
  col = mix(col, H, 0.45*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
float ve_open(){ return rxLife(0.8, 2.2, 0.35); }
vec3 ve_mouth(float k){ return around(4.1 + 0.2*sin(uClock*0.2 + k*3.0), (k < 0.5 ? 0.08 : 0.58) + uClock*0.004, uLength*(k < 0.5 ? 0.74 : 0.27) + 0.5*sin(uClock*0.17 + k*2.0)); }
vec3 sig_vol_e1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.25, 1.0, 0.35);
  if (aft <= 0.01) return fx;
  float st = ve_burst();
  if (part(1.0) > 0.5){
    fx += 1.3*syMistSpin(ro, rd, L*0.02, L*0.97, 1.2, 5.6, 0.01, 71.0, B*0.55, D*8.0);
    fx += 1.05*syMistSpin(ro, rd, L*0.1, L*0.9, 1.6, 6.2, -0.006, 72.0, Vi*0.28 + Au*0.22, D*4.0);
  }
  if (part(2.0) > 0.5){
    fx += vec3(0.0);
    // the arms of a galaxy winding slowly round it, each a broad glow with its own stars strung along it
    fx += 1.6*xDust(ro, rd, 1.7, 4.4, 1.0, uClock*0.016, L*0.03, L*0.97, 0.75, 77.0, H, B);
    fx += 1.6*xDust(ro, rd, 4.2, 1.8, -0.85, -uClock*0.013 + 0.45, L*0.03, L*0.97, 0.7, 78.0, H, Au);
  }
  if (part(3.0) > 0.5){
    fx += 1.5*vec3(0.0);
    fx += (1.25 - 0.8*st)*syInflow(ro, rd, px, 12.0, 6.0, L*0.1, L*0.92, 0.12, 74.0, H, B);
  }
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// >>> e1: the great effect round this blade (4/11/2026)
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the wormhole: two mouths in the air, their light turning in spirals; what falls into one comes out of the other
  if (part(4.0) > 0.5){
    vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
    for (int i = 0; i < 2; i++){
      float k = float(i), tp;
      vec3 c = ve_mouth(k);
      float R = 1.9*ve_open();
      if (R < 0.05 || rayPtRaw(ro, rd, c, tp) > R*1.3) continue;
      vec3 w0 = ro + rd*tp - c;
      vec2 u = vec2(dot(w0, e1), dot(w0, e2))/R;
      float r0 = length(u), th = atan(u.y, u.x);
      float spin = (k < 0.5 ? 1.0 : -1.0)*(3.0*th + 6.0*log(r0 + 0.08)) - C*1.1*(k < 0.5 ? -1.0 : 1.0);
      float arm = pow(0.5 + 0.5*sin(spin), 2.2);
      float env = exp(-pow((r0 - 0.5)/0.34, 2.0));
      a += mix(k < 0.5 ? B*1.15 : Au, H, arm*arm*0.55)*arm*env*1.25*(1.0 - behindBlade(ro, rd, tp));
      a += H*exp(-pow((r0 - 0.27)/0.05, 2.0))*0.75*(0.7 + 0.3*sin(th*2.0 + C))*(1.0 - behindBlade(ro, rd, tp));
    }
    // comets through it
    for (int i = 0; i < 4; i++){
      float k = float(i), run = C*0.1 + k*0.25, cyc = floor(run), q = fract(run), s;
      float ang = h11(k*3.1 + cyc), hgt = L*(0.15 + 0.7*h11(k*5.1 + cyc));
      vec3 far0 = around(5.2, ang, hgt);
      vec3 h1, t1;
      float g, en;
      if (q < 0.5){
        g = q*2.0; g = g*g;
        h1 = mix(far0, ve_mouth(0.0), g); t1 = mix(far0, ve_mouth(0.0), max(g - 0.25, 0.0)); en = smoothstep(0.0, 0.15, g)*(1.0 - smoothstep(0.86, 1.0, g));
      } else {
        g = (q - 0.5)*2.0; g = 1.0 - (1.0 - g)*(1.0 - g);
        h1 = mix(ve_mouth(1.0), far0.zyx*vec3(1.0, 0.0, 1.0) + vec3(0.0, L - hgt, 0.0), g); t1 = mix(ve_mouth(1.0), far0.zyx*vec3(1.0, 0.0, 1.0) + vec3(0.0, L - hgt, 0.0), max(g - 0.25, 0.0)); en = smoothstep(0.0, 0.14, g)*(1.0 - smoothstep(0.8, 1.0, g));
      }
      float dc = raySeg(ro, rd, t1, h1, s);
      a += (H*halo(dc, max(0.03 + 0.05*s*s, px))*(0.4 + 1.5*s*s) + mix(B, Au, mod(k, 2.0))*(halo(dc, 0.12 + 0.12*s)*(0.7 + 0.5*s)))*s*en*ve_open();
    }
  }
  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_e1(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  // the glow of the knot: lobes of nebula light opening slowly to each side
  float lob = rxFlow(vec2(b.y*0.14 - C*0.05 + b.x*0.08, b.x*0.18 + 3.0));
  if (part(1.0) > 0.5) col += mix(B, Vi*0.8 + Au*0.3, smoothstep(0.35, 0.7, rxFlow(vec2(b.y*0.08 + C*0.03, 9.0))))*lob*lob*halo(d, 4.6)*0.62*on;

  return col;
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  // the dark throat of each mouth
  if (uFx > 0.0 && part(4.0) > 0.5 && fxNear(ro, rd, 0.0, uLength) < 6.5){
    for (int i = 0; i < 2; i++){
      float tp, R = 1.9*ve_open();
      float d = rayPtRaw(ro, rd, ve_mouth(float(i)), tp);
      float cov = (1.0 - smoothstep(R*0.14, R*0.3, d))*(1.0 - behindBlade(ro, rd, tp));
      if (cov > o.a) o = vec4(D*0.3, cov*uFx*tipFade(ro, rd)*hiltFade(ro, rd));
    }
  }
  return o;
}
vec3 sig_vol_own(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_w2(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 col = sig_glow_w2(g, b, r, px);
  float d = max(abs(b.x) - 0.5, 0.0), C = uClock, on = rxLife(0.3, 1.3, 0.35)*uFx*tipFadeB(b)*growGlow(b)*roomFade(abs(b.x))*smoothstep(0.0, 4.0, b.y);
  if (part(4.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.52*syWisps(ro, rd, px, 5.0, 2.10, 3.89, 0.42, 910.0, vec3(1.000, 0.980, 0.880), vec3(0.980, 0.800, 0.450));
  a += 1.1*syRibbon(ro, rd, px, 1.7, 4.4, 1.0, uClock*0.016, L*0.02, L*0.98, 0.9, 75.0, B*0.6, B*0.62);
  a += 1.0*syRibbon(ro, rd, px, 4.2, 1.8, -0.85, -uClock*0.013 + 0.45, L*0.02, L*0.98, 0.8, 76.0, Au*0.5, Au*0.42 + Vi*0.1);
  a += 1.2*gxVortex(ro, rd, px, 12.0, 5.0, 62.0, vec3(1.000, 0.980, 0.880), vec3(0.980, 0.800, 0.450));
  a += 1.0*gxPlanets(ro, rd, px, 2.0, 4.4, 63.0, vec3(1.000, 0.980, 0.880), vec3(0.980, 0.800, 0.450));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

