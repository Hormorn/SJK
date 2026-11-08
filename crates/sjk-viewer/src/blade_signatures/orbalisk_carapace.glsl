// Orbalisk Carapace (rebuilt 3/11/2026): The orbalisks that armoured Darth Bane: dark shells packed over a burning body, the light running through the seams between them while each shell's eye of heat opens in turn. Lumps crawl along its outline, and lightning crawls over them as they feed.
// @part Living shells
// @part Feeding lightning
// @part Heat motes
// @part Parasite breath
// @raycache fast
// @room 10
// @solid 1
// @moments own
// @trail carapace
// @palette {"corona.rim_cool":[0.55,0.341,0.11],"corona.rim_hot":[1.0,0.62,0.2],"corona.inner":[1,0.9,0.65],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.8,0.496,0.16],"core.fringe_hot":[1,0.9,0.65],"light.color":[1,0.62,0.2],"trail":[1,0.62,0.2]}
// @speed 1.0

const vec3 Sh = vec3(0.095, 0.075, 0.060);
const vec3 O = vec3(1.000, 0.440, 0.070);
const vec3 Y = vec3(1.000, 0.780, 0.260);
const vec3 H = vec3(1.000, 0.950, 0.780);
float ob_top(){ return uLength*rxLife(0.0, 0.45, 0.32); }
float ob_feed(){ return rxCycle(17.0, 8.0, 10.2, 12.1, 15.1); }
float ob_half(float y, float sd){
  float top = ob_top();
  if (y > top) return 0.0;
  float e = 1.0 - clamp((top - y)/1.2, 0.0, 1.0);
  float lumps = 0.24*vn(vec2(y*0.8, sd*3.0)) + 0.1*vn(vec2(y*2.1, sd*7.0));
  float crawl = 0.0;
  for (int i = 0; i < 3; i++){
    float k = float(i);
    float yk = uLength*fract(k*0.37 + uClock*(0.012 + 0.006*k)*(k == 1.0 ? -1.0 : 1.0));
    if ((k == 1.0 ? -1.0 : 1.0) == sd) crawl += 0.3*exp(-(y - yk)*(y - yk)/1.1);
  }
  return (0.54 + lumps + crawl)*sqrt(max(1.0 - e*e, 0.0))*smoothstep(-0.25, 0.8, y);
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){
  float top = ob_top(), sd = b.x < 0.0 ? -1.0 : 1.0;
  float yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - ob_half(yc, sd), 0.0);
  float st = ob_feed();
  vec3 col = O*(halo(d, 0.28)*0.9 + halo(d, 1.3)*(0.34 + 0.3*st) + halo(d, 3.8)*(0.1 + 0.12*st));
  col += lvl(g)*O*(1.0 - smoothstep(0.3, 1.5, abs(b.y)))*0.25;
  col *= tipFadeB(b);
  return col*roomFade(abs(b.x))*(1.0 + 0.8*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float sd = b.x < 0.0 ? -1.0 : 1.0;
  float half = ob_half(b.y, sd);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  // shells packed together: the nearest and next-nearest cell centre
  vec2 p = vec2(b.x*1.25, b.y*0.72);
  vec2 ip = floor(p), fp = fract(p);
  float d1 = 9.0, d2 = 9.0, id = 0.0;
  vec2 near = vec2(0.0);
  for (int j = -1; j <= 1; j++) for (int i = -1; i <= 1; i++){
    vec2 c = vec2(float(i), float(j));
    float h = h11(dot(ip + c, vec2(7.3, 3.1)));
    vec2 v = c + vec2(h, fract(h*7.7)) - fp;
    float dd = dot(v, v);
    if (dd < d1){ d2 = d1; d1 = dd; id = h; near = v; } else if (dd < d2){ d2 = dd; }
  }
  d1 = sqrt(d1); d2 = sqrt(d2);
  float seam = 1.0 - smoothstep(0.02, 0.2, d2 - d1);
  float dome = clamp(1.0 - d1*1.45, 0.0, 1.0);
  float light = clamp(0.5 - 0.7*near.x + 0.45*near.y, 0.0, 1.0);
  vec3 col = Sh*(0.6 + 1.5*dome*dome)*(0.55 + 1.1*light) + vec3(0.42, 0.36, 0.3)*pow(dome, 6.0)*light*0.5;
  float st = ob_feed();
  if (part(0.0) > 0.5){
    float run = pow(0.5 + 0.5*sin(b.y*0.34 - uClock*1.7 + id*1.2), 3.0);
    col += mix(O, Y, run)*seam*(0.8 + 2.0*run + 1.4*st);
    col += H*seam*seam*run*0.8;
    // each shell's own eye of heat, opening in turn
    float open = smoothstep(0.65, 1.0, sin(uClock*0.9 + id*40.0));
    col += O*pow(dome, 3.0)*open*(1.1 + 1.5*st);
    col += H*pow(dome, 10.0)*open*1.2;
  }
  col = mix(col, H, 0.4*clamp(uClash, 0.0, 1.0));
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
vec3 sig_vol_e1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.2, 0.9, 0.35);
  if (aft <= 0.01) return fx;
  float st = ob_feed();
  if (part(1.0) > 0.5) fx += vec3(0.0);
  if (part(2.0) > 0.5) fx += (0.5 + 1.1*st)*rxBorn(ro, rd, px, 14.0, L*0.05, L*0.95, 0.5, 3.4, 0.5, 0.06, 32.0, H, O);
  if (part(3.0) > 0.5) fx += 0.8*syMistSpin(ro, rd, 0.3, L*0.7, 1.0, 3.2, 0.02, 33.0, O*0.28, Sh*1.2);
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// The orbalisks themselves: shells clinging in the air close round it, each creeping its own way along the blade, its eye of heat opening in turn.
vec3 ob_at(float k, out float R){
  float L = uLength;
  R = 0.3 + 0.28*h11(k*5.3);
  return around(1.0 + 0.55*h11(k*3.1) + 0.14*sin(uClock*0.3 + k), h11(k*7.7) + (mod(k, 2.0) < 0.5 ? 1.0 : -1.0)*uClock*0.012, L*(0.06 + 0.84*fract(k*0.618)) + 0.9*sin(uClock*0.11 + k*2.1));
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx <= 0.0 || part(0.0) < 0.5 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  for (int i = 0; i < 9; i++){
    float k = float(i), R;
    vec3 c = ob_at(k, R);
    // a domed shell, its long eye of heat opening and closing
    float tm, dd = rayPtRaw(ro, rd, c, tm);
    vec4 q = vec4(0.0);
    if (dd < R*1.6){
      vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1), w0 = ro + rd*tm - c;
      vec2 u = rot(k*1.3)*vec2(dot(w0, e1), dot(w0, e2))/R;
      float o0 = length(vec2(u.x*0.78, u.y*1.2));
      float nz0 = sqrt(max(1.0 - o0*o0, 0.0));
      float eye = pow(0.5 + 0.5*sin(uClock*0.8 + k*2.3), 3.0);
      vec3 sc = Sh*(0.8 + 2.4*nz0*(0.55 + 0.45*u.y)) + mix(O, Y, eye)*(1.0 - smoothstep(0.0, 0.05 + 0.13*eye, abs(u.y)))*(1.0 - smoothstep(0.35, 0.78, abs(u.x)))*(0.3 + 1.4*eye);
      q = vec4(sc, (1.0 - smoothstep(0.92 - px/R, 1.0 + px/R, o0))*(1.0 - behindBlade(ro, rd, tm)));
    }
    if (q.a > o.a) o = q;
  }
  o.a *= uFx*rxLife(0.3, 1.2, 0.35)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return o;
}
// >>> e1: the great effect round this blade (4/11/2026)
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  float t;
  if (part(0.0) > 0.5 && fxNear(ro, rd, 0.0, L) < 4.8) for (int i = 0; i < 9; i++){
    float k = float(i), R;
    vec3 c = ob_at(k, R);
    float pulse = pow(0.5 + 0.5*sin(C*0.8 + k*2.3), 3.0);
    a += (Y*halo(rayPt(ro, rd, c, t), R*1.1)*0.5 + O*halo(rayPt(ro, rd, c, t), R*3.0)*0.42)*pulse;
  }
  // the burning below the shells rising past them as heat and sparks
  if (part(3.0) > 0.5) for (int i = 0; i < 6; i++){
    float k = float(i), run = C*(0.2 + 0.05*h11(k)) + k*0.27, cyc = floor(run), ph = fract(run), en = sin(3.1416*ph);
    a += en*en*syLick(ro, rd, px, L*(0.05 + 0.75*h11(k*2.3 + cyc)), h11(k*5.7 + cyc), 1.2 + 1.0*h11(k*3.1 + cyc), 3.2 + 2.6*h11(k*4.3 + cyc), 0.46, k*3.0 + cyc, Y, O*0.85, Sh*2.0);
  }
  if (part(2.0) > 0.5){
    a += 1.5*gxKin(ro, rd, 6.0, 7.0, 1.60, 0.060, 0.40, 0.08, 0.96, 1.0, 7845.0, H, O);
    a += 1.3*shFlakes(ro, rd, 3.4, 0.5, L - 0.5, 14.0, 2.8, 0.7, 0.03, 0.09, 0.5, 0.0, 202.0, Y, O*0.6);
  }

  return base + a*rxLife(0.3, 1.2, 0.4)*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
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
  if (part(3.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.20*gxKin(ro, rd, 1.0, 7.0, 2.20, 0.080, 0.20, 0.08, 0.96, -1.0, 9272.0, vec3(1.000, 0.900, 0.650), vec3(1.000, 0.620, 0.200));
  a += 1.2*gxSwarm(ro, rd, px, 12.0, 4.0, 60.0, vec3(1.000, 0.900, 0.650), vec3(1.000, 0.620, 0.200));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

