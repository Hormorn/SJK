// Amphistaff Serpent (rebuilt 4/11/2026): A Yuuzhan Vong amphistaff, alive and awake: a straight blade of black-green chitin plates with acid running in the seams. Two serpents of the same living stuff slide slowly round it in the air, venom drips from it and a poisoned breath hangs about it. When it strikes they whip fast and spit.
// @part Chitin plates
// @part Venom
// @part Spit
// @part Serpent breath
// @part Living coils
// @raycache fast
// @room 8
// @solid 1
// @moments own
// @trail serpent
// @palette {"corona.rim_cool":[0.099,0.429,0.127],"corona.rim_hot":[0.18,0.78,0.23],"corona.inner":[0.92,1.0,0.7],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.144,0.624,0.184],"core.fringe_hot":[0.92,1.0,0.7],"light.color":[0.18,0.78,0.23],"trail":[0.18,0.78,0.23]}
// @speed 1.0

const vec3 M = vec3(0.180, 0.780, 0.230);
const vec3 D = vec3(0.012, 0.080, 0.040);
const vec3 A = vec3(0.680, 1.000, 0.200);
const vec3 H = vec3(0.920, 1.000, 0.700);
float am_top(){ return uLength*rxLife(0.0, 0.6, 0.35); }
float am_strike(){ return rxCycle(15.0, 6.0, 8.5, 9.5, 12.5); }
float am_half(float y){
  float top = am_top();
  if (y > top) return 0.0;
  float tip = 1.0 - clamp((top - y)/3.4, 0.0, 1.0);
  return (mix(0.56, 0.42, y/max(uLength, 1.0)) + 0.025*sin(y*0.5 - uClock*1.1))*sqrt(max(1.0 - tip*tip, 0.0));
}
vec3 sig_glow_grown(vec3 g, vec2 b, float r, float px){
  float top = am_top(), yc = clamp(b.y, 0.0, top);
  float d = max(length(vec2(b.x, b.y - yc)) - am_half(yc), 0.0);
  float st = am_strike();
  float pulse = pow(0.5 + 0.5*sin(b.y*0.3 - uClock*1.4), 3.0);
  vec3 col = g*(0.4 + 0.3*pulse + 0.4*st + 0.7*clamp(uClash, 0.0, 1.0));
  col += M*(halo(d, 0.2)*(0.5 + 0.5*pulse) + halo(d, 1.1)*(0.26 + 0.25*st) + halo(d, 3.2)*(0.07 + 0.1*st));
  return col*tipFadeB(b)*roomFade(abs(b.x));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*0.0; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float L = uLength;
  if (b.y < -0.3 || b.y > L + 0.3) return o;
  float half = am_half(b.y);
  float cov = (1.0 - smoothstep(half - 0.05 - px, half + 0.03 + px, abs(b.x)))*smoothstep(0.02, 0.12 + px, half);
  if (cov <= 0.0) return o;
  float u = clamp(b.x/max(half, 0.01), -1.0, 1.0), nz = sqrt(max(1.0 - u*u, 0.0));
  vec3 col = D*2.5;
  if (part(0.0) > 0.5){
    // long overlapping plates of chitin, dark and glossy, parted by thin seams of acid that wander along the blade
    float ang = asin(u);
    float v1 = rxFlow(vec2(ang*1.25 + 0.18*sin(b.y*0.35), b.y*0.11));
    float v2 = rxFlow(vec2(ang*2.1 + 5.0, b.y*0.2 + 3.0));
    float plate = 0.5 + 0.5*sin(b.y*2.4 + 2.2*v1 + ang*1.5);
    col = mix(D*1.5, mix(vec3(0.05, 0.2, 0.12), M*0.5, smoothstep(0.35, 0.8, v2)), smoothstep(0.1, 0.9, plate));
    col += H*pow(plate, 7.0)*0.16*nz;
    float run = 0.3 + 0.7*pow(0.5 + 0.5*sin(b.y*0.3 - uClock*1.4), 3.0);
    float seam = max(1.0 - smoothstep(0.0, 0.035, abs(v1 - 0.5)), 0.7*(1.0 - smoothstep(0.0, 0.025, abs(v2 - 0.5))));
    col = mix(col, mix(A, H, run*0.4)*(0.55 + 1.0*run + 0.8*am_strike()), seam);
  }
  col *= 0.55 + 0.6*nz;
  col += M*pow(1.0 - nz, 3.0)*0.75;
  col = mix(col, H, 0.45*clamp(uClash, 0.0, 1.0));
  col *= smoothstep(-0.25, 0.3, b.y);
  return vec4(mix(o.rgb, col, cov), max(o.a, cov));
}
// A serpent's place at f along its way up the blade.
vec3 am_at(float f, float k, float st){
  float dir = k < 0.5 ? 1.0 : -1.0;
  float wig = sin(f*13.0 - uClock*1.1 + k*3.0);
  return around(1.5 + 0.26*wig + 0.5*k, dir*(1.5*f + 0.012*uClock) + k*0.46, uLength*0.96*f + 0.12*cos(f*13.0 - uClock*1.1));
}
// The nearest the ray comes to serpent k, how far along it (0 its tail, 1 its nose); false when it isn't near.
float am_trace(vec3 ro, vec3 rd, float k, float st, int n, out float at){
  float head = (0.04 + 1.34*fract(uClock*0.028*(1.0 + 0.2*k) + k*0.5))*rxLife(0.2, 1.6, 0.4);
  float f0 = clamp(head - 0.52, 0.01, 0.98), f1 = clamp(head, 0.01, 0.98);
  at = 0.0;
  if (f1 - f0 < 0.01) return 1e3;
  float bWs = 0.0;
  float best = 1e3, s;
  vec3 prev = am_at(f0, k, st);
  for (int j = 1; j <= n; j++){
    float q = float(j)/float(n), f = mix(f0, f1, q);
    vec3 p = am_at(f, k, st);
    float d = raySeg(ro, rd, prev, p, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); at += (clamp((mix(f0, f1, (float(j) - 1.0 + s)/float(n)) - (head - 0.52))/0.52, 0.0, 1.0) - at)*wN/bWs; }
    prev = p;
  }
  return best;
}
float am_w(float at){ return 0.23*pow(max(sin(3.1416*at*0.94 + 0.08), 0.0), 0.7)*(1.0 + 0.3*smoothstep(0.7, 0.9, at)) + 0.004; }
vec3 sig_vol_e1(vec3 ro, vec3 rd, float px){
  vec3 fx = vec3(0.0);
  float L = uLength;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return fx;
  float aft = rxLife(0.3, 1.0, 0.35);
  if (aft <= 0.01) return fx;
  float st = am_strike();
  if (part(1.0) > 0.5) fx += sxDrops(ro, rd, px, 11.0, 1.5, 0.12, 61.0, H, A);
  if (part(3.0) > 0.5){
    fx += 1.0*syMistSpin(ro, rd, 0.3, L*0.9, 1.2, 4.0, 0.018, 63.0, M*0.32, D*3.0);
    fx += 1.3*gxKin(ro, rd, 2.0, 7.0, 1.80, 0.160, 0.22, 0.08, 0.96, -1.0, 7026.0, H, A);
  }
  if (part(4.0) > 0.5) for (int i = 0; i < 2; i++){
    float k = float(i), at;
    float d = am_trace(ro, rd, k, st, 6, at);
    fx += M*(halo(d, 0.55)*0.3 + halo(d, 1.3)*0.1)*sin(3.1416*at);
    // what it spits from its nose as it strikes
    if (part(2.0) > 0.5){
      float run = uClock/15.0 + 0.533, q = fract(run)*5.0;
      float nose = clamp(0.04 + 1.34*fract(uClock*0.028*(1.0 + 0.2*k) + k*0.5), 0.01, 0.98);
      fx += 1.4*rxSparkShower(ro, rd, px, am_at(nose, k, st), q, 15.0, 3.8, floor(run) + k*3.0, H, A);
    }
  }
  return fx*aft*uFx*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  vec4 o = vec4(0.0);
  if (uFx <= 0.0 || part(4.0) < 0.5 || fxNear(ro, rd, 0.0, uLength) > 16.0) return o;
  float st = am_strike();
  for (int i = 0; i < 2; i++){
    float k = float(i), at;
    float d = am_trace(ro, rd, k, st, 18, at);
    float w = am_w(at);
    float cov = 1.0 - smoothstep(w - 0.025 - px, w + 0.025 + px, d);
    if (cov <= 0.0) continue;
    float side = clamp(d/w, 0.0, 1.0), nz = sqrt(max(1.0 - side*side, 0.0));
    // fine dark scales with a wet sheen and a thin acid line down its spine
    float sc = vn(vec2(at*120.0, side*5.0 + at*30.0));
    vec3 col = mix(D*1.3, M*0.55, smoothstep(0.3, 0.75, sc))*(0.3 + 0.8*nz);
    col += H*pow(nz, 12.0)*0.3*(0.5 + 0.5*sin(at*40.0 - uClock*2.0)) + A*(1.0 - smoothstep(0.0, 0.16, side))*0.5 + M*pow(side, 3.0)*0.65;
    if (cov > o.a) o = vec4(col, cov);
  }
  o.a *= uFx*rxLife(0.3, 1.0, 0.35)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
  return o;
}
vec3 sig_glow_w2(vec3 g, vec2 b, float r, float px){ return sig_glow_grown(g, b, r, px)*growGlow(b); }
// >>> e1: the great effect round this blade (4/11/2026)
vec3 sig_vol_w2(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_e1(ro, rd, px), a = vec3(0.0);
  float L = uLength, C = uClock;
  if (uFx <= 0.0 || fxNear(ro, rd, 0.0, L) > 16.0) return base;
  // the venom storm: tall curtains of poison leaning round it, with acid flames running up inside them
  if (part(3.0) > 0.5){
    for (int i = 0; i < 5; i++){
      float k = float(i), run = C*(0.14 + 0.03*h11(k)) + k*0.27, cyc = floor(run), ph = fract(run), en = sin(3.1416*ph);
      a += 1.1*en*en*syLick(ro, rd, px, L*(0.05 + 0.72*h11(k*2.3 + cyc)), h11(k*5.7 + cyc) + 0.05*ph, 1.8 + 1.5*h11(k*3.1 + cyc), 4.0 + 3.2*h11(k*4.3 + cyc), 0.42, k*3.0 + cyc, H*0.7 + A*0.3, A*0.7, D*5.0);
    }
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
  if (part(4.0) > 0.5) col += vec3(0.0);

  return col;
}

// >>> what this blade has in the air of its own
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 base = sig_vol_own(ro, rd, px), a = vec3(0.0);
  if (uFx <= 0.0) return base;
  float L = uLength;
  a += 1.53*gxKin(ro, rd, 6.0, 7.0, 1.80, 0.050, 0.50, 0.08, 0.96, 1.0, 9017.0, vec3(0.920, 1.000, 0.700), vec3(0.180, 0.780, 0.230));
  a += 1.2*gxHelix(ro, rd, px, 18.0, 3.0, 2.6, 0.04, 61.0, vec3(0.920, 1.000, 0.700), vec3(0.180, 0.780, 0.230));
  return base + a*rxLife(0.8, 3.0, 0.45)*tipFade(ro, rd)*hiltFade(ro, rd)*growMask(ro, rd);
}
// <<<

