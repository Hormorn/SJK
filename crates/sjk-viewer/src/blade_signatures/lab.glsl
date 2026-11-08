
// ---- signature helpers ----
float h11(float p){ p=fract(p*0.1031); p*=p+33.33; p*=p+p; return fract(p); }
float h21(vec2 p){ vec3 q=fract(vec3(p.xyx)*0.1031); q+=dot(q,q.yzx+33.33); return fract((q.x+q.y)*q.z); }
vec2 h22(vec2 p){ vec3 q=fract(vec3(p.xyx)*vec3(0.1031,0.1030,0.0973)); q+=dot(q,q.yzx+33.33); return fract((q.xx+q.yz)*q.zy); }
float vn(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.0-2.0*f);
  return mix(mix(h21(i),h21(i+vec2(1.0,0.0)),f.x),mix(h21(i+vec2(0.0,1.0)),h21(i+vec2(1.0)),f.x),f.y); }
float fbm(vec2 p){ float s=0.0, a=0.5; for(int i=0;i<5;i++){ s+=a*vn(p); p=p*2.03+vec2(1.7,9.2); a*=0.5; } return s; }
float fbm3(vec2 p){ float s=0.0, a=0.5; for(int i=0;i<3;i++){ s+=a*vn(p); p=p*2.03+vec2(1.7,9.2); a*=0.5; } return s/0.875; }
vec3 hsv(float h, float s, float v){ vec3 p=abs(fract(vec3(h)+vec3(0.0,2.0/3.0,1.0/3.0))*6.0-3.0); return v*mix(vec3(1.0),clamp(p-1.0,0.0,1.0),s); }
mat2 rot(float a){ float c=cos(a), s=sin(a); return mat2(c,-s,s,c); }
float sdSeg(vec2 p, vec2 a, vec2 b){ vec2 pa=p-a, ba=b-a; float h=clamp(dot(pa,ba)/dot(ba,ba),0.0,1.0); return length(pa-ba*h); }
float sdBox(vec2 p, vec2 b){ vec2 d=abs(p)-b; return length(max(d,0.0))+min(max(d.x,d.y),0.0); }
float sdEll(vec2 p, vec2 r){ float k=length(p/r); return (k-1.0)*min(r.x,r.y); }
float fillD(float d, float px){ return clamp(0.5-d/max(px,1e-4),0.0,1.0); }
float halo(float d, float w){ float e=max(d,0.0)/w; return exp(-e*e); }
float voro(vec2 p, out vec2 cell){ vec2 i=floor(p), f=fract(p); float d1=8.0, d2=8.0; cell=vec2(0.0);
  for(int y=-1;y<=1;y++) for(int x=-1;x<=1;x++){ vec2 g=vec2(float(x),float(y)); vec2 o=h22(i+g); float d=length(g+o-f);
    if(d<d1){ d2=d1; d1=d; cell=i+g; } else if(d<d2) d2=d; }
  return d2-d1; }
float sigBody(vec2 b, float r){ float y=b.y, L=uLength; float beyond=max(y-L,0.0); return length(vec2(b.x, beyond))/r*step(-1.0,y); }
// ---- SJK effect helpers (shared by the signatures; Creyon's round 7) ----
// Distance from p to a jagged lightning bolt from a to c: 10 kinks, displaced across the
// line by up to jag of its length, the kinks chosen by seed (strike anew by changing it).
float boltD(vec2 p, vec2 a, vec2 c, float seed, float jag){
  vec2 d = c - a; float L = max(length(d), 1e-3); vec2 n = vec2(-d.y, d.x)/L;
  float best = 1e3; vec2 prev = a;
  for (int i = 1; i <= 10; i++){
    float f = float(i)/10.0;
    float off = i == 10 ? 0.0 : (h11(seed*17.13 + float(i)*7.31) - 0.5)*jag*L*sin(f*3.1416);
    vec2 q = a + d*f + n*off;
    best = min(best, sdSeg(p, prev, q));
    prev = q;
  }
  return best;
}
// A bolt's light: a white-hot thread in a coloured glow, never thinner than a pixel.
vec3 boltLight(float dist, vec3 col, float px){
  return vec3(1.0)*halo(dist, max(px, 0.035))*1.6 + col*halo(dist, 0.22)*0.9 + col*halo(dist, 0.9)*0.18;
}
// A Storm-like loop: a bolt that leaves the blade at height y0, arcs out to reach units on
// side s and comes back in at y1, struck anew rate times a second.
vec3 arcLoop(vec2 b, float y0, float y1, float s, float reach, float seed, float rate, vec3 col, float px){
  float k = floor(uTime*rate + seed*13.0);
  vec2 a = vec2(0.0, y0), c = vec2(0.0, y1);
  vec2 m = vec2(s*reach*(0.75 + 0.25*h11(k + seed)), mix(y0, y1, 0.5) + (h11(k*1.7 + seed) - 0.5)*2.0);
  float d = min(boltD(b, a, m, k + seed, 0.45), boltD(b, m, c, k + seed + 0.5, 0.45));
  return boltLight(d, col, px);
}
// A helix strand wound round the blade: across (x) and how near the viewer it is (z, -1..1)
// at height y, turns per 10 units, phase turns, radius r.
vec2 helix(float y, float r, float turns, float phase){
  float a = 6.2832*(y*turns/10.0 + phase);
  return vec2(r*sin(a), cos(a));
}
// ---- 3D helpers (round 8): effects drawn along the true view ray ----
// In sig_vol(ro, rd, px) a signature gets the view ray in the blade's frame: ro the camera,
// rd its unit direction, y along the blade from the hilt, x across it to the camera's side,
// z toward the camera. Everything placed with these sits round the blade in depth and keeps
// its shape from every side, from above and in first person.
// Round 22 (Creyon: nothing may be seen through the blade when it is behind it): how hidden by
// the blade's body a point t along the ray is, 0 (seen) to 1 (hidden): the ray passes through the
// blade's body (its nearest approach to the axis within the blade) before reaching it.
// Round 23 (players' reports after the 2026.1021.1 release: hard-edged boxes and lines round
// the tip when a blade pointed at the camera): it fades in and out softly; it used to switch
// off at once when looking nearly along the blade.
// 31/10/2026 (Sol: the top-tier blades cost too much): what of it depends on the ray alone (where
// the ray passes the axis, how near, the tip's narrowing) is worked out once per pixel for the
// view ray (`labRayPrepare`, which saber.wgsl calls before sig_vol, sig_orbit and their solid
// parts) and read at every piece's point; a ray that never passes behind the body hides nothing,
// so the rest is skipped. A piece asking for another ray (a shifted one) works its own out. The
// same values as before. behindBlade only reads what was kept (writing it there kept the shader
// compiler from dropping calls whose result goes unused: World Shift drew at half speed).
vec3 bbRo = vec3(1e9);
vec3 bbRd = vec3(0.0);
float bbTa = 0.0;
float bbNear = 0.0;
float bbAlong = 0.0;
void labRayPrepare(vec3 ro, vec3 rd){
  bbRo = ro;
  bbRd = rd;
  bbTa = 0.0;
  bbNear = 0.0;
  float a = dot(rd.xz, rd.xz);
  bbAlong = smoothstep(0.01, 0.04, a);
  if (bbAlong <= 0.0) return;
  float ta = -dot(ro.xz, rd.xz)/a;
  float y = ro.y + rd.y*ta;
  vec2 off = ro.xz + rd.xz*ta;
  float d = length(off);
  float into = clamp((y - (uLength - 1.4))/1.4, 0.0, 1.0);
  float w = sqrt(max(1.0 - into*into, 0.0));
  float onBlade = smoothstep(-0.8, -0.2, y)*(1.0 - smoothstep(uLength - 0.15, uLength + 0.05, y));
  bbTa = ta;
  bbNear = (1.0 - smoothstep(0.35*w, 0.8*w + 0.02, d))*onBlade;
}
// The whole of it, from the ray alone: what labRayPrepare keeps, and what a piece asking for
// another ray (or a module without the cache) works out (written out twice rather than shared
// through out parameters, which made World Shift draw at a third of its speed on an RTX 5080).
float behindBladeFull(vec3 ro, vec3 rd, float t){
  float a = dot(rd.xz, rd.xz);
  float along = smoothstep(0.01, 0.04, a);
  if (along <= 0.0) return 0.0;
  float ta = -dot(ro.xz, rd.xz)/a;
  float y = ro.y + rd.y*ta;
  vec2 off = ro.xz + rd.xz*ta;
  float d = length(off);
  // what lies on the blade's own body (its axis, its glow) is never hidden by it
  vec3 p = ro + rd*t;
  float outside = smoothstep(0.5, 0.9, length(p.xz));
  // round 23: the body hiding narrows with the blade's rounded tip (a quarter circle over its
  // last 1.4 units) down to the point, so what passes behind the tip is hidden too (Sol's review:
  // ending the hiding before the tip let Exegol Rift's flare and Quasar's jet show through),
  // without the dark round end past the tip that a full-width hiding drew seen end on
  float into = clamp((y - (uLength - 1.4))/1.4, 0.0, 1.0);
  float w = sqrt(max(1.0 - into*into, 0.0));
  float onBlade = smoothstep(-0.8, -0.2, y)*(1.0 - smoothstep(uLength - 0.15, uLength + 0.05, y));
  return (1.0 - smoothstep(0.35*w, 0.8*w + 0.02, d))*onBlade*outside*smoothstep(ta + 0.15, ta + 0.7, t)*along;
}
// For a signature whose header says `@raycache off`, its module calls behindBladeFull in its place
// (blade_signatures.rs module_source): World Shift drew at a third of its speed reading what was
// kept, or even with the reading switched off by a constant, on an RTX 5080 (Vulkan).
float behindBlade(vec3 ro, vec3 rd, float t){
  if (ro.x == bbRo.x && ro.y == bbRo.y && ro.z == bbRo.z && rd.x == bbRd.x && rd.y == bbRd.y && rd.z == bbRd.z){
    if (bbNear <= 0.0 || bbAlong <= 0.0) return 0.0;
    vec3 p = ro + rd*t;
    float outside = smoothstep(0.5, 0.9, length(p.xz));
    // multiplied in this order: hideD magnifies the last digit a thousandfold where a piece is
    // nearly hidden
    return bbNear*outside*smoothstep(bbTa + 0.15, bbTa + 0.7, t)*bbAlong;
  }
  return behindBladeFull(ro, rd, t);
}
// A distance d as seen when h (behindBlade) of it is hidden by the blade: unchanged at 0, out of
// reach at 1, and in between the piece narrows and dims, so whatever passes behind the blade fades
// out over the blade's soft edge. Round 23: adding 1e3*h hid a piece wherever h was more than a
// thousandth, a hard cut along the blade's body that showed as a box round the tip when the blade
// pointed at the camera.
float hideD(float d, float h){ return d/max(1.0 - h, 1e-3) + h; }
// The distance from the ray to point p, as seen (a point the blade hides is far off); t
// returns how far along the ray that is.
float rayPtRaw(vec3 ro, vec3 rd, vec3 p, out float t){
  t = max(dot(p - ro, rd), 0.0);
  return length(ro + rd*t - p);
}
float rayPt(vec3 ro, vec3 rd, vec3 p, out float t){
  float d = rayPtRaw(ro, rd, p, t);
  return hideD(d, behindBlade(ro, rd, t));
}
// The distance from the ray to the segment a-b; s returns where on it (0 at a, 1 at b); the
// raw one ignores the blade's body (for bounds and the blade's own axis).
float raySegRaw(vec3 ro, vec3 rd, vec3 a, vec3 b, out float s){
  vec3 ba = b - a, w = ro - a;
  float bd = dot(rd, ba), bb = dot(ba, ba), dw = dot(rd, w), bw = dot(ba, w);
  float den = max(bb - bd*bd, 1e-6);
  s = clamp((bw - bd*dw)/den, 0.0, 1.0);
  vec3 q = a + ba*s;
  float t = max(dot(q - ro, rd), 0.0);
  vec3 pr = ro + rd*t;
  s = clamp(dot(pr - a, ba)/max(bb, 1e-6), 0.0, 1.0);
  return length(pr - (a + ba*s));
}
float raySeg(vec3 ro, vec3 rd, vec3 a, vec3 b, out float s){
  float d = raySegRaw(ro, rd, a, b, s);
  vec3 q = a + (b - a)*s;
  return hideD(d, behindBlade(ro, rd, max(dot(q - ro, rd), 0.0)));
}
// Where the ray crosses the plane across the blade at height h: xz there, and t (negative:
// never, or behind the camera).
vec3 rayAt(vec3 ro, vec3 rd, float h){
  float t = abs(rd.y) > 1e-5 ? (h - ro.y)/rd.y : -1.0;
  vec3 p = ro + rd*max(t, 0.0);
  return vec3(p.x, p.z, t);
}
// Where the ray is within R of the blade's axis and between heights y0 and y1: the span of
// t (empty when x >= y).
vec2 raySpan(vec3 ro, vec3 rd, float R, float y0, float y1){
  float a = dot(rd.xz, rd.xz), b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - R*R;
  vec2 s = vec2(0.0, 1e5);
  if (a < 1e-6){
    if (c > 0.0) return vec2(1.0, 0.0);
  } else {
    float h = b*b - a*c;
    if (h < 0.0) return vec2(1.0, 0.0);
    h = sqrt(h);
    s = vec2((-b - h)/a, (-b + h)/a);
  }
  if (abs(rd.y) > 1e-6){
    float ta = (y0 - ro.y)/rd.y, tb = (y1 - ro.y)/rd.y;
    s = vec2(max(s.x, min(ta, tb)), min(s.y, max(ta, tb)));
  } else if (ro.y < y0 || ro.y > y1){
    return vec2(1.0, 0.0);
  }
  s.x = max(s.x, 0.0);
  return s;
}
// round 19: `p` turned a (turns) round the blade's axis, for a piece laid out in the blade's
// picture (x across, y up) to stand at its own place round the blade in three dimensions.
vec3 turnY(vec3 p, float a){ float c = cos(6.2832*a), s = sin(6.2832*a); return vec3(p.x*c + p.z*s, p.y, -p.x*s + p.z*c); }
// A point round the blade: radius r, angle a (turns), height y.
vec3 around(float r, float a, float y){ return vec3(r*sin(6.2832*a), y, r*cos(6.2832*a)); }
// The blade itself in three dimensions: a white-hot core and a coloured glow round the
// axis from just inside the hilt to the tip. It fills the hilt's mouth as a stock blade
// does, seen from any side, from above and in first person. w is the core's half width.
vec3 blade3(vec3 ro, vec3 rd, float px, vec3 col, float w, float top){
  float s;
  float d = raySegRaw(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), s);
  return vec3(1.0)*exp(-pow(d/max(w, px), 2.0))*1.4 + col*halo(d, w*2.2)*0.75 + col*halo(d, w*6.0)*0.18;
}
// A lightning bolt in three dimensions from a to c: 10 kinks, each pushed off the line in
// both directions across it by up to jag of its length.
float bolt3(vec3 ro, vec3 rd, vec3 a, vec3 c, float seed, float jag, float px){
  vec3 d = c - a; float L = max(length(d), 1e-3);
  // 31/10/2026 (Sol: the top-tier blades cost too much): every point of the bolt lies within
  // half its length and 0.71 of its zigzag (jag*L) of its middle, so a ray passing further off
  // than that plus six units and three pixels is further than that from all of it, where the
  // widest glow a caller gives a bolt (1.5) or a pixel-wide one is under a ten-thousandth: such a
  // ray skips the ten segments.
  float tm;
  if (rayPtRaw(ro, rd, (a + c)*0.5, tm) - L*(0.5 + 0.71*jag) > max(6.0, 3.0*px)) return 1e3;
  vec3 n1 = normalize(cross(d, abs(d.y) < 0.9*L ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
  vec3 n2 = cross(d/L, n1);
  float best = 1e3; vec3 prev = a; float s;
  for (int i = 1; i <= 10; i++){
    float f = float(i)/10.0;
    float k = i == 10 ? 0.0 : sin(f*3.1416)*jag*L;
    vec3 q = a + d*f + (n1*(h11(seed*17.13 + float(i)*7.31) - 0.5) + n2*(h11(seed*5.71 + float(i)*3.17) - 0.5))*k;
    best = min(best, raySeg(ro, rd, prev, q, s));
    prev = q;
  }
  return best;
}
// A soft cloud of light: march the ray through a smooth density inside radius R between
// heights y0 and y1 in n steps (the caller sums its own field; this gives the steps).
// Returns the span and the step length in z.
vec3 marchSteps(vec3 ro, vec3 rd, float R, float y0, float y1, float n){
  vec2 s = raySpan(ro, rd, R, y0, y1);
  return vec3(s, max(s.y - s.x, 0.0)/n);
}
// ---- round 9: the big effects (Creyon: "dramatic and beautiful, like beams, strays, rays") ----
// Every one is placed in the blade's own frame (fixed to the hilt), sized in units, and fades
// in and out smoothly; the loops have constant bounds and stop at `count`.
// How near the ray passes the blade's axis between heights y0 and y1 (for early outs).
float fxNear(vec3 ro, vec3 rd, float y0, float y1){
  float s;
  return raySegRaw(ro, rd, vec3(0.0, y0, 0.0), vec3(0.0, y1, 0.0), s);
}
// The ray's distance to a ring of radius R round the blade at height y: the nearer of where
// it crosses the ring's plane and the ring's cylinder (smooth from any side).
float ringD(vec3 ro, vec3 rd, float y, float R){
  float best = 1e3;
  vec3 h = rayAt(ro, rd, y);
  if (h.z > 0.0) best = hideD(abs(length(h.xy) - R), behindBlade(ro, rd, h.z));
  float a = dot(rd.xz, rd.xz), b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - R*R;
  float dsc = b*b - a*c;
  if (dsc > 0.0 && a > 1e-6){
    float q = sqrt(dsc);
    float t0 = (-b - q)/a, t1 = (-b + q)/a;
    if (t0 > 0.0) best = min(best, hideD(abs(ro.y + rd.y*t0 - y), behindBlade(ro, rd, t0)));
    if (t1 > 0.0) best = min(best, hideD(abs(ro.y + rd.y*t1 - y), behindBlade(ro, rd, t1)));
  }
  return best;
}
// A star flare at p facing the camera: a white-hot heart of radius size and n thin rays of
// length len, turned by spin (radians).
vec3 fxFlare(vec3 ro, vec3 rd, vec3 p, float px, float size, float len, float n, float spin, vec3 col){
  float t;
  float d = rayPt(ro, rd, p, t);
  // round 23: skipped only past its widest halo (size*8), which fades to nothing before the cut
  // (at size*4 it was still a tenth as bright and ended at a hard ring round the flare; Sol's
  // review: size*18 cost too much, size*12 with the taper below is as clean)
  if (d > len + size*12.0) return vec3(0.0);
  vec3 q = ro + rd*t - p;
  vec3 u = normalize(cross(rd, abs(rd.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
  vec3 v = cross(rd, u);
  vec2 e = vec2(dot(q, u), dot(q, v));
  float r = length(e);
  float k = 6.2832/max(n, 1.0);
  float aa = mod(atan(e.y, e.x) + spin + k*0.5, k) - k*0.5;
  float across = abs(r*sin(aa)), along = r*cos(aa);
  float w = max(px, 0.03 + 0.012*along);
  float ray = exp(-pow(across/w, 2.0))*exp(-max(along, 0.0)/max(len, 0.01)*2.6)*step(0.0, along);
  float wide = halo(d, size*8.0)*(1.0 - smoothstep(len + size*7.0, len + size*12.0, d));
  return (vec3(1.0)*(halo(d, max(size, px))*1.8 + ray*0.9) + col*(halo(d, size*3.0)*0.7 + ray*0.8 + wide*0.15))*roomAt(p);
}
// Beams of light shooting out of the blade between heights y0 and y1: count of them (up to
// 24), each up to len long, flaring out and fading rate times a second, seed chooses them.
vec3 fxRays(vec3 ro, vec3 rd, float px, float y0, float y1, float count, float len, float rate, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  // round 23: past the beams' widest glow (nine of their widths), which ended at a hard edge
  if (fxNear(ro, rd, y0, y1) > len*1.6 + 5.0) return acc;
  float s;
  for (int i = 0; i < 24; i++){
    float fi = float(i);
    if (fi >= count) break;
    float cyc = uTime*rate*(0.8 + 0.4*h11(fi*3.3 + seed)) + h11(fi*7.13 + seed);
    float k = floor(cyc), q = fract(cyc);
    float y = mix(y0, y1, h11(k*3.1 + fi*1.7 + seed));
    float a = h11(k*5.7 + fi*2.3 + seed);
    float tilt = (h11(k*2.9 + fi + seed) - 0.5)*0.9;
    vec3 dir = normalize(vec3(sin(6.2832*a), tilt, cos(6.2832*a)));
    float lr = min(len*(0.55 + 0.6*h11(k*1.3 + fi + seed)), 0.85*uReach)*smoothstep(0.0, 0.3, q);
    vec3 a0 = vec3(0.0, y, 0.0) + dir*0.35;
    float d = raySeg(ro, rd, a0, a0 + dir*max(lr, 0.01), s);
    float env = sin(3.1416*q);
    float w = mix(0.3, 0.07, s);
    acc += (vec3(1.0)*halo(d, max(w*0.45, px))*1.4 + col*halo(d, w*3.0)*0.95 + col*halo(d, w*9.0)*0.22)*(1.0 - 0.75*s)*env;
  }
  return acc;
}
// A bolt's light (boltLight) faded to nothing 1.7 from it, so fxArcs can skip a bolt past that
// without a seam: its widest glow is a fortieth as bright there.
vec3 arcGlow(float dist, vec3 col, float px){
  return boltLight(dist, col, px)*(1.0 - smoothstep(1.1, 1.7, dist));
}
// Lightning in three dimensions: count (up to 10) bolts that leave the blade between y0 and
// y1, arc out up to reach units on every side and come back in, with a fork, struck anew
// rate times a second (Sol's Storm, all round the blade).
vec3 fxArcs(vec3 ro, vec3 rd, float px, float y0, float y1, float count, float reach, float rate, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  // round 23: past the forks' reach (half again the bolt's, which the blade's room caps at 0.6 of
  // it) and their glow, which ended at a hard edge
  if (fxNear(ro, rd, y0, y1) > min(reach, 0.6*uReach)*1.6 + 3.0) return acc;
  float t;
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    if (fi >= count) break;
    float cyc = uTime*rate*(0.7 + 0.6*h11(fi*1.9 + seed)) + h11(fi*3.3 + seed)*5.0;
    float k = floor(cyc), q = fract(cyc);
    float ya = mix(y0, y1, h11(k*1.9 + fi*2.7 + seed));
    float yb = min(ya + 3.0 + 6.0*h11(k*2.3 + fi + seed), y1 + 2.0);
    float a = h11(k*4.1 + fi*2.0 + seed);
    float R = min(reach, 0.6*uReach)*(0.45 + 0.55*h11(k*7.7 + fi + seed));
    vec3 mid = around(R, a, mix(ya, yb, 0.5) + (h11(k*9.1 + fi) - 0.5)*2.0);
    // round 23 (Sol's review: timed on Creyon's RTX 5070, the arc blades cost a fifth to a third
    // more once their glow faded out instead of being cut): each of the arc's three bolts (its two
    // halves and the fork) is drawn only near its own middle. A bolt's points lie within half its
    // length of its middle (its zigzag reaches less aside than that), and its glow is made to end
    // 1.7 past them (arcGlow), so a pixel further off would draw nothing from it.
    if (rayPtRaw(ro, rd, mid, t) > R*1.25 + (yb - ya)*0.65 + 2.8) continue;
    float env = (1.0 - q)*(1.0 - q)*(0.6 + 0.4*step(0.5, fract(q*9.0 + fi)));
    vec3 ea = around(0.25, a, ya), eb = around(0.25, a, yb);
    vec3 fork = mid + around(R*0.55, a + 0.12*(h11(k + fi) - 0.5), (h11(k*3.3 + fi) - 0.5)*3.0);
    float d = 1e3, d2 = 1e3;
    if (rayPtRaw(ro, rd, (ea + mid)*0.5, t) < 0.55*length(mid - ea) + 1.8 + px)
      d = bolt3(ro, rd, ea, mid, k + fi*1.37 + seed, 0.5, px);
    if (rayPtRaw(ro, rd, (mid + eb)*0.5, t) < 0.55*length(eb - mid) + 1.8 + px)
      d = min(d, bolt3(ro, rd, mid, eb, k + fi*1.37 + seed + 0.5, 0.5, px));
    if (rayPtRaw(ro, rd, (mid + fork)*0.5, t) < 0.55*length(fork - mid) + 1.8 + px)
      d2 = bolt3(ro, rd, mid, fork, k*2.1 + fi + seed, 0.6, px);
    acc += (arcGlow(d, col, px) + arcGlow(d2, col, px)*0.55)*env;
  }
  return acc;
}
// Sparks spraying off the blade between y0 and y1: count (up to 32) streaks flying out up
// to len units, white-hot then cooling to col, rate cycles a second.
vec3 fxSparks(vec3 ro, vec3 rd, float px, float y0, float y1, float count, float len, float rate, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > len + 1.5) return acc;
  float s;
  for (int i = 0; i < 32; i++){
    float fi = float(i);
    if (fi >= count) break;
    float cyc = uTime*rate*(0.7 + 0.6*h11(fi*2.1 + seed)) + h11(fi*5.9 + seed);
    float k = floor(cyc), q = fract(cyc);
    float y = mix(y0, y1, h11(k*2.7 + fi*1.3 + seed));
    float a = h11(k*6.3 + fi*0.7 + seed);
    vec3 dir = normalize(vec3(sin(6.2832*a), 0.2 + 0.9*(h11(k*1.1 + fi + seed) - 0.3), cos(6.2832*a)));
    float v = min(len, 0.85*uReach)*(0.5 + 0.5*h11(k*4.4 + fi + seed));
    vec3 o = around(0.3, a, y);
    float e1 = 1.0 - exp(-q*3.0), e0 = 1.0 - exp(-max(q - 0.07, 0.0)*3.0);
    float d = raySeg(ro, rd, o + dir*v*e0, o + dir*v*e1, s);
    float life = 1.0 - smoothstep(0.55, 1.0, q);
    acc += mix(vec3(1.0, 0.97, 0.9), col, smoothstep(0.0, 0.6, q))*(halo(d, max(0.05, px))*2.0 + halo(d, 0.28)*0.3)*life;
  }
  return acc;
}
// Coils of light wound round the blade from y0 to y1 at radius R, turns per 10 units,
// travelling up at speed; knots of light run along them. count (up to 4) strands.
vec3 fxCoils(vec3 ro, vec3 rd, float px, float y0, float y1, float R, float turns, float speed, float count, vec3 col){
  vec3 acc = vec3(0.0);
  // round 23: past the coils' widest glow (it ended at a hard edge, R + 1.5)
  if (fxNear(ro, rd, y0, y1) > R + 2.5) return acc;
  float s;
  for (int j = 0; j < 4; j++){
    float fj = float(j);
    if (fj >= count) break;
    float best = 1e3, bf = 0.0;
    vec3 prev = vec3(0.0);
    for (int i = 0; i <= 24; i++){
      float f = float(i)/24.0;
      float y = mix(y0, y1, f);
      vec3 p = around(R*(0.85 + 0.15*sin(f*12.0 + uTime*2.0 + fj)), y*turns/10.0 - uTime*speed + fj/max(count, 1.0), y);
      if (i > 0){
        float d = raySeg(ro, rd, prev, p, s);
        if (d < best){ best = d; bf = (float(i) - 1.0 + s)/24.0; }
      }
      prev = p;
    }
    float knot = pow(0.5 + 0.5*sin(bf*40.0 - uTime*9.0 + fj*2.0), 6.0);
    float ends = smoothstep(0.0, 0.08, bf)*(1.0 - smoothstep(0.9, 1.0, bf));
    acc += (vec3(1.0)*halo(best, max(0.05, px))*(0.5 + 1.5*knot) + col*halo(best, 0.3)*(0.5 + knot) + col*halo(best, 1.0)*0.12)*ends;
  }
  return acc;
}
// Motes (dust, embers, sparks of light) drifting up round the blade between radii r0 and r1:
// count (up to 40), each a soft point that twinkles.
vec3 fxMotes(vec3 ro, vec3 rd, float px, float y0, float y1, float r0, float r1, float count, float speed, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > r1 + 1.0) return acc;
  float t;
  for (int i = 0; i < 40; i++){
    float fi = float(i);
    if (fi >= count) break;
    float f = fract(uTime*speed*(0.6 + 0.8*h11(fi*2.9 + seed)) + h11(fi*5.1 + seed));
    float y = mix(y0 - 1.0, y1 + 2.0, f);
    vec3 p = around(mix(r0, r1, h11(fi*3.7 + seed)), h11(fi*1.9 + seed) + uTime*0.04 + 0.08*sin(uTime*0.7 + fi), y);
    float d = rayPt(ro, rd, p, t);
    float tw = 0.55 + 0.45*sin(uTime*5.0 + fi*2.3);
    acc += col*(halo(d, max(0.07, px))*1.8 + halo(d, 0.35)*0.2)*tw*smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.8, 1.0, f))*roomAt(p);
  }
  return acc;
}
// A shock ring racing out from the blade at height y: q its age (0 to 1), out to R.
vec3 fxShock(vec3 ro, vec3 rd, float px, float y, float q, float R, vec3 col){
  if (q <= 0.0 || q >= 1.0) return vec3(0.0);
  float Rr = 0.5 + R*(1.0 - pow(1.0 - q, 2.0));
  float d = ringD(ro, rd, y, Rr);
  float env = (1.0 - q)*smoothstep(0.0, 0.05, q)*roomFade(Rr + 0.8);
  return (vec3(1.0)*halo(d, max(0.07, px))*1.4 + col*halo(d, 0.45)*0.7 + col*halo(d, 1.4)*0.15)*env;
}
// A wide soft aura round the blade from y0 to y1: its light reaching w units out.
vec3 fxAura(vec3 ro, vec3 rd, float y0, float y1, float w, vec3 col){
  float d = fxNear(ro, rd, y0, y1);
  return col*(halo(d, w)*0.35 + halo(d, w*0.4)*0.25);
}
// ---- round 11: small helpers for the lower tiers ----
// The glow's level and the glow at that level in colour c (a tint keeping Sol's shape).
float lvl(vec3 g){ return max(g.r, max(g.g, g.b)); }
vec3 recol(vec3 g, vec3 c){ return lvl(g)*c; }
// Bands of light travelling along the blade: k bands a unit, moving at v units a second.
float bands(float y, float k, float v){ return 0.5 + 0.5*sin(y*k*6.2832 - uTime*v*k*6.2832); }
// Falling streaks (rain, snow, ash) in the blade's picture: count of them (up to 32) between
// heights y0 and y1, reaching r across, falling at v, slanted by lean.
vec3 fxFall(vec3 ro, vec3 rd, float px, float y0, float y1, float r, float count, float v, float len, float lean, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  // round 23: the streaks start up to 2 units past y0 and y1 and reach len further (leaning
  // lean*len aside): skipped only outside all that (Lightspeed Ram's rising streaks were cut
  // above the tip)
  if (fxNear(ro, rd, y0 - 2.0 + min(len, 0.0), y1 + 2.0 + max(len, 0.0)) > r + abs(lean*len) + 1.0) return acc;
  float s;
  for (int i = 0; i < 32; i++){
    float fi = float(i);
    if (fi >= count) break;
    float f = fract(uTime*v/(y1 - y0 + 4.0)*(0.7 + 0.6*h11(fi*2.3 + seed)) + h11(fi*5.9 + seed));
    float y = mix(y1 + 2.0, y0 - 2.0, f);
    vec3 p = around(0.8 + (min(r, 0.9*uReach) - 0.8)*h11(fi*3.1 + seed), h11(fi*7.7 + seed), y);
    vec3 q = p + vec3(lean, 1.0, 0.0)*len;
    float d = raySeg(ro, rd, p, q, s);
    acc += col*(halo(d, max(0.04, px))*1.2 + halo(d, 0.2)*0.2)*(1.0 - 0.6*s);
  }
  return acc;
}
// ---- round 11: every effect can be switched off ----
// 1 while the signature's own part k (its `// @part` line k, from 0) is on, 0 when its wearer
// switched it off in the Collection.
float part(float k){ return 1.0 - mod(floor(uParts/exp2(k) + 0.001), 2.0); }
// ---- round 19: nothing is ever cut at the quad's edge ----
// How much of what lies r units from the blade's axis shows: all of it well inside the reach
// the blade's quad holds without fading (`uReach`), fading smoothly to nothing before it
// (Creyon: Reactor's shock ring was cut off where it left that room).
float roomFade(float r){ return 1.0 - smoothstep(0.72*uReach, 0.97*uReach, r); }
float roomAt(vec3 p){ return roomFade(length(p.xz)); }
// ---- round 23: sharp patterns without stair steps (players' reports after 2026.1021.1) ----
// A step from 0 to 1 at e, smoothed over w (a pixel, in x's units): a pattern's edge stays sharp
// but no longer stair-steps or flickers as the blade moves.
float aaStep(float e, float x, float w){ return smoothstep(e - 0.5*w, e + 0.5*w, x); }
// How far a pixel is into the next cell of u (cells one long), over the cell's last w (a pixel in
// u's units, and at least a twelfth of the cell, as px is the pixel across the blade and a pixel
// along it is longer where the blade is seen end on): blending a cell's value into the next one's
// by it keeps blocks and stripes sharp.
float cellMix(float u, float w){ return smoothstep(1.0 - clamp(w, 0.08, 0.5), 1.0, fract(u)); }
// A square wave of u, 0 for the first half of each one-long cell and 1 for the second, its two
// edges smoothed over w (a pixel in u's units, at least a twelfth of the cell, as cellMix).
float aaSquare(float u, float w){ float f = fract(u); w = clamp(w, 0.08, 0.25); return aaStep(0.5, f, w)*(1.0 - smoothstep(1.0 - w, 1.0, f)); }
// ---- round 12: richer pieces for the lower tiers (each blade sets its own pattern) ----
// Soft curling smoke round the blade between heights y0 and y1, w units to each side at its
// thickest, rising at `rise` units a second (negative sinks): warped smooth noise with no hard
// edge (Creyon's round 12: "the smoke looks so squary"). Round 19 (Creyon: seen from above, the
// smoke was a flat sheet following the camera): a cloud in three dimensions, the ray marched
// through it in six steps, as bright side on as the old sheet.
float fxSmokeAt(vec3 p, float w, float y0, float y1, float rise, float seed){
  float ax = length(p.xz);
  vec2 q = vec2((p.x + 0.62*p.z)*0.32, p.y*0.12 - uTime*rise*0.12) + vec2(seed*7.3, seed*3.1);
  q += vec2(0.0, p.z*0.09 - p.x*0.05);
  vec2 warp = vec2(vn(rot(0.6)*q*0.9 + vec2(0.0, uTime*0.07)), vn(rot(2.1)*q*0.9 + vec2(5.2, -uTime*0.06)));
  vec2 r = q + 1.3*warp;
  float n = 0.6*vn(rot(0.35)*r) + 0.4*vn(rot(1.7)*r*2.1 + vec2(3.3, 1.1) + vec2(p.z*0.2, -p.x*0.2));
  float shape = exp(-pow(ax/w, 2.0))*smoothstep(y0 - 3.0, y0 + 2.0, p.y)*(1.0 - smoothstep(y1 - 7.0, y1 - 1.0, p.y));
  return smoothstep(0.35, 0.8, n)*shape;
}
// Round 23 (players' reports after 2026.1021.1: a hard-edged dome in the smoke seen along the
// blade): a step at most about a third of the cloud's width, up to 12 of them (16 cost the smoke
// blades up to a sixth more seen along them, Sol's review), so a long path through it (looking
// along the blade) is not sampled six units apart; seen from the side the path is short and it
// still takes six.
vec3 fxSmoke(vec3 ro, vec3 rd, float y0, float y1, float w, float rise, float seed, vec3 col){
  vec2 span = raySpan(ro, rd, w*2.4, y0 - 3.0, y1);
  if (span.x >= span.y) return vec3(0.0);
  float n = clamp(ceil((span.y - span.x)/(0.8*w)), 6.0, 12.0);
  float dt = (span.y - span.x)/n;
  float acc = 0.0;
  for (int i = 0; i < 12; i++){
    if (float(i) >= n) break;
    float ts = span.x + (float(i) + 0.5)*dt;
    acc += fxSmokeAt(ro + rd*ts, w, y0, y1, rise, seed)*(1.0 - behindBlade(ro, rd, ts));
  }
  return col*acc*dt/(1.77*w)*0.9;
}
// Ribbons of energy flowing along the blade: count (up to 4) wavy strands alternating sides,
// R units out at most, `wave` their waviness, pulses of light running up them at `speed`.
vec3 fxStreams(vec3 ro, vec3 rd, float px, float y0, float y1, float count, float R, float wave, float speed, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > R + 1.5) return acc;
  float s;
  for (int j = 0; j < 4; j++){
    float fj = float(j);
    if (fj >= count) break;
    float best = 1e3, bf = 0.0;
    vec3 prev = vec3(0.0);
    // round 19: each strand on its own side all round the blade, in three dimensions (it lay
    // in one flat sheet)
    float side = fj/max(count, 1.0) + 0.13*seed;
    for (int i = 0; i <= 14; i++){
      float f = float(i)/14.0;
      float y = mix(y0, y1, f);
      float x = 0.45 + (min(R, 0.9*uReach) - 0.45)*(0.5 + 0.5*sin(y*wave + fj*1.7 + seed + uTime*(0.5 + 0.2*fj)));
      vec3 p = around(x, side + 0.05*sin(y*0.15 + uTime*0.3 + fj), y);
      if (i > 0){
        float d = raySeg(ro, rd, prev, p, s);
        if (d < best){ best = d; bf = (float(i) - 1.0 + s)/14.0; }
      }
      prev = p;
    }
    float pulse = pow(0.5 + 0.5*sin(bf*18.0 - uTime*speed + fj*2.1 + seed), 4.0);
    float ends = smoothstep(0.0, 0.1, bf)*(1.0 - smoothstep(0.85, 1.0, bf));
    acc += (vec3(1.0)*halo(best, max(0.035, px))*(0.25 + 1.1*pulse) + col*halo(best, 0.22)*(0.35 + 0.8*pulse) + col*halo(best, 0.9)*0.07)*ends;
  }
  return acc;
}
// Flakes of light (snow, ash, petals, leaves of light) drifting down round the blade: count
// (up to 24) soft flat ovals `size` long and `aspect` as wide, falling at v, turning at spin,
// swaying `drift` units, spread up to `spread` units to each side.
vec3 fxFlakes(vec3 ro, vec3 rd, float px, float y0, float y1, float spread, float count, float v, float size, float aspect, float spin, float drift, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  // round 23: skipped only past where a flake's soft halo has faded (a flake sways `drift` past
  // `spread`, and its halo reaches about six sizes): cut there, it drew a hard dome round the
  // blade seen along it (Yavin Vines, players' reports after 2026.1021.1)
  if (fxNear(ro, rd, y0 - 3.0, y1) > spread + abs(drift) + size*6.0 + 1.0) return acc;
  float ta;
  for (int i = 0; i < 24; i++){
    float fi = float(i);
    if (fi >= count) break;
    float f = fract(uTime*v/(y1 - y0 + 4.0)*(0.7 + 0.6*h11(fi*2.7 + seed)) + h11(fi*6.1 + seed));
    float y = mix(y1, y0 - 3.0, f);
    // round 19: all round the blade in three dimensions (they drifted in one flat sheet),
    // each flake a small oval facing the camera
    float rr = min(0.6 + (spread - 0.6)*h11(fi*3.9 + seed) + drift*sin(uTime*0.9 + fi*2.0), 0.95*uReach);
    vec3 p = around(max(rr, 0.4), h11(fi*4.3 + seed) + 0.03*sin(uTime*0.5 + fi), y);
    float d = rayPt(ro, rd, p, ta);
    if (d > size*6.0 + px) continue;
    vec3 q = ro + rd*ta - p;
    vec3 fu = normalize(cross(rd, abs(rd.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
    vec2 e = rot(uTime*spin + fi*1.7)*vec2(dot(q, fu), dot(q, cross(rd, fu)));
    float pd = length(e/vec2(size, size*aspect));
    acc += col*(halo(pd - 1.0, 0.3)*0.8 + halo(d, size*2.5)*0.1)*smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.85, 1.0, f));
  }
  return acc;
}
// Waves of light travelling up (or down, v < 0) the blade as rings round it, `count` at a
// time (up to 6), R across, each fading as it goes.
vec3 fxRingWaves(vec3 ro, vec3 rd, float px, float y0, float y1, float count, float R, float v, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > R + 1.5) return acc;
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    if (fi >= count) break;
    float q = fract(uTime*v/(y1 - y0) + fi/max(count, 1.0) + seed);
    if (v < 0.0) q = 1.0 - q;
    float y = mix(y0, y1, q);
    float d = ringD(ro, rd, y, min(R, 0.9*uReach)*(0.7 + 0.3*sin(3.1416*q)));
    float env = sin(3.1416*q);
    acc += (vec3(1.0)*halo(d, max(0.04, px))*0.7 + col*halo(d, 0.3)*0.6 + col*halo(d, 1.0)*0.08)*env;
  }
  return acc;
}
// ---- round 17: what circles the blade goes behind it ----
// How much of what is at `at` the blade's body leaves seen along this ray (in the hilt's frame):
// all of it in front of the blade, none of it behind (Creyon: Balance's orb of shadow showed
// through the blade).
float bladeSeen(vec3 ro, vec3 rd, vec3 at){
  float L = uLength, sAx, tAx, tAt;
  raySegRaw(ro, rd, vec3(0.0), vec3(0.0, L, 0.0), sAx);
  float dAx = rayPtRaw(ro, rd, vec3(0.0, sAx*L, 0.0), tAx);
  rayPt(ro, rd, at, tAt);
  // round 23: fading in behind the axis over a little depth, not at a hard step
  return 1.0 - (1.0 - smoothstep(0.35, 0.9, dAx))*smoothstep(tAx - 0.4, tAx + 0.4, tAt);
}
// ---- Creyon's Star Wars round, first pass (1/11/2026): building blocks its blades compose in their own ways ----
// A smooth, grid-free field from 0 to 1: warped sine waves at three angles (value noise's square
// cells show as straight lines when thresholded; this never does).
float rxFlow(vec2 p){
  float v = sin(p.x + 1.5*sin(p.y*0.9 + 0.4));
  v += 0.8*sin(p.y*1.3 + 1.2*sin(p.x*1.2 + 1.9));
  v += 0.5*sin((p.x - p.y)*2.0 + 1.0*sin(p.y*2.1 + 0.7));
  v += 0.3*sin((p.x + p.y)*2.7 + 0.9*sin(p.x*2.6 - 0.8));
  return 0.5 + v/5.2;
}
// How far the blade has come to life: 0 to 1 over `on` seconds after it lights (after `delay`),
// back to 0 over `off` seconds once it is let go.
float rxLife(float delay, float on, float off){
  if (uAge >= 2000.0) return 1.0 - smoothstep(0.0, off, uAge - 2000.0);
  return smoothstep(delay, delay + on, uAge);
}
// A slow cycle of `period` seconds (offset by the blade's seed): rising from a to b, holding,
// falling from c to d; 0 to 1, smooth both ways.
float rxCycle(float period, float a, float b, float c, float d){
  float t = mod(uClock + uSeed*7.0, period);
  return smoothstep(a, b, t)*(1.0 - smoothstep(c, d, t));
}
// How long rxCycle has been on so far, in seconds (its time integral, 1/11/2026, update 16):
// a speed that rises with the cycle must move by `uTime*base + extra*rxCycleInt(...)`, never by
// `uTime*(base + extra*rxCycle(...))`, which jumps by the whole age of the blade times the change
// as the cycle rises (Creyon's videos: effects suddenly racing; the longer he played, the worse).
float rxCycleInt(float period, float a, float b, float c, float d){
  float T = uClock + uSeed*7.0;
  float n = floor(T/period);
  float x = T - n*period;
  float area = 0.5*(b - a) + (c - b) + 0.5*(d - c);
  float u = clamp((x - a)/max(b - a, 1e-3), 0.0, 1.0);
  float v = clamp((x - c)/max(d - c, 1e-3), 0.0, 1.0);
  float up = (b - a)*(u*u*u - 0.5*u*u*u*u);
  float hold = clamp(x - b, 0.0, c - b);
  float down = (d - c)*(v - (v*v*v - 0.5*v*v*v*v));
  return n*area + up + hold + down;
}
// Where in a one-off moment of the same cycle we are: 0 to 1 from `at` over `len` seconds, below
// 0 outside it.
float rxMoment(float period, float at, float len){
  float t = mod(uClock + uSeed*7.0, period) - at;
  return (t >= 0.0 && t < len) ? t/len : -1.0;
}
// A blade's half width for its own body: w across, drawn to a rounded point over its last
// `tipLen` units (top is where its point is now).
float rxHalf(float y, float top, float w, float tipLen){
  return w*sqrt(clamp((top - y)/max(tipLen, 0.01), 0.0, 1.0));
}
// The distance from b to a blade of half width `half` at height y (y clamped to the blade), soft
// and rounded past its point and below its emitter.
float rxOut(vec2 b, float top, float half){
  float y = clamp(b.y, 0.0, max(top, 0.0));
  return length(vec2(max(abs(b.x) - half, 0.0), b.y - y));
}
// Tongues of flame licking up off a blade's outline (in its picture): d the distance from the
// outline, reach their height, speed how fast they climb; hot where they leave it, cooling to
// `cool` at their tips. Rounded tongues, smooth both ways.
vec3 rxFlames(vec2 b, float d, float top, float reach, float speed, float seed, vec3 hot, vec3 cool){
  if (d <= 0.0 || b.y < 0.0 || b.y > top + reach) return vec3(0.0);
  float side = clamp(b.x*2.0, -1.0, 1.0);
  vec2 q = vec2(b.y*0.55 - d*1.2 - uTime*speed*0.4, side*6.0 + d*0.4 + seed);
  float n = rxFlow(q*1.4)*0.7 + rxFlow(q*2.6 + 3.0)*0.3;
  float h = reach*(0.2 + 1.3*n*n);
  float tongue = 1.0 - smoothstep(h*0.3, h, d);
  float heat = 1.0 - clamp(d/max(h, 0.01), 0.0, 1.0);
  // nothing over the point (Creyon): the tongues end below it, softly
  return mix(cool, hot, smoothstep(0.2, 0.9, heat))*tongue*(1.0 - smoothstep(top - 2.5, top + 0.3, b.y))*smoothstep(0.0, 2.5, b.y);
}
// Tall curtains of light standing round a blade (in its picture), wavering slowly upward like an
// aurora's: d the distance from the blade, reach how far they stand out.
vec3 rxCurtains(vec2 b, float d, float top, float reach, vec3 c1, vec3 c2){
  if (d > reach*1.5) return vec3(0.0);
  float sheet = rxFlow(vec2(b.x*1.1 + 0.6*sin(b.y*0.08 + uTime*0.25), b.y*0.05 - uTime*0.18));
  float folds = pow(0.5 + 0.5*sin(b.x*2.4 + 3.0*sheet + uTime*0.4), 3.0);
  float r = reach*(0.55 + 0.6*sheet);
  float body = (1.0 - smoothstep(0.3*r, r, d))*smoothstep(0.1, 0.8, d + 0.3);
  float span = smoothstep(-0.5, 2.5, b.y)*(1.0 - smoothstep(top - 2.5, top + 0.3, b.y));
  return (mix(c1, c2, folds)*folds*0.7 + c1*0.3)*body*span;
}
// The distance from the ray to a jagged bolt from a to c whose kinks writhe smoothly in time (each
// on its own slow wave, so it crawls rather than jumps); f returns where along it the nearest
// point is (0 at a).
float rxBolt(vec3 ro, vec3 rd, vec3 a, vec3 c, float seed, float jag, float speed, out float f){
  vec3 ab = c - a;
  float len = length(ab);
  vec3 side = normalize(cross(ab, vec3(0.31, 0.12, 0.94)) + vec3(1e-4));
  vec3 up = normalize(cross(side, ab) + vec3(1e-4));
  float best = 1e3, s;
  f = 0.0;
  vec3 prev = a;
  for (int i = 1; i <= 10; i++){
    float k = float(i)/10.0;
    float env = sin(3.1416*k);
    vec3 p = a + ab*k;
    if (i < 10){
      float ph = seed*7.1 + k*31.0;
      float sx = sin(uTime*speed*0.5*(1.0 + 0.5*h11(ph)) + 6.2832*h11(ph + 1.0)) + 0.5*sin(uTime*speed*1.15 + ph);
      float sy = sin(uTime*speed*0.5*(0.8 + 0.5*h11(ph + 2.0)) + 6.2832*h11(ph + 3.0));
      p += (side*sx*0.6 + up*sy*0.5)*jag*len*env;
    }
    float d = raySeg(ro, rd, prev, p, s);
    if (d < best){ best = d; f = (float(i) - 1.0 + s)/10.0; }
    prev = p;
  }
  return best;
}
// The distance from the ray to a smooth spiral round the blade from y0 to y1: radius r0 at its
// foot to r1 at its top, `turns` times round, turned by `phase` (turns); 24 short pieces, so no
// corner shows. f returns where along it the nearest point is.
float rxHelix(vec3 ro, vec3 rd, float r0, float r1, float turns, float phase, float y0, float y1, out float f){
  float best = 1e3, s;
  f = 0.0;
  // 1/11/2026 (Creyon's FPS): four chords first; every point of the helix lies within the
  // chords' sagitta of them, so a ray further than that and three units (past every caller's
  // glow) gets the chords' distance, a lower bound, where nothing it draws shows
  {
    float coarse = 1e3;
    vec3 cp = around(r0, phase, y0);
    for (int j = 1; j <= 4; j++){
      float k = float(j)/4.0;
      vec3 p = around(mix(r0, r1, k), phase + k*turns, mix(y0, y1, k));
      coarse = min(coarse, raySegRaw(ro, rd, cp, p, s));
      cp = p;
    }
    float sag = max(r0, r1)*(1.0 - cos(min(3.1416*abs(turns)*0.25, 3.1416))) + 0.1;
    if (coarse - sag > 3.0){ f = 0.5; return coarse - sag; }
  }
  vec3 prev = around(r0, phase, y0);
  for (int j = 1; j <= 24; j++){
    float k = float(j)/24.0;
    vec3 p = around(mix(r0, r1, k), phase + k*turns, mix(y0, y1, k));
    float d = raySeg(ro, rd, prev, p, s);
    if (d < best){ best = d; f = (float(j) - 1.0 + s)/24.0; }
    prev = p;
  }
  return best;
}
// Particles born inside the blade's edge (a small flare where each leaves), pushed out of it and
// rising: `count` of them (up to 24) between heights y0 and y1, `rate` lives a second each,
// carried out to `out` units and up `rise` units, `size` across; seed chooses them. Each is hot at
// its birth and cools to `col`.
vec3 rxBorn(vec3 ro, vec3 rd, float px, float count, float y0, float y1, float rate, float outR, float rise, float size, float seed, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1 + rise) > outR + 2.0) return acc;
  float t;
  for (int i = 0; i < 24; i++){
    float k = float(i);
    if (k >= count) break;
    float cyc = uTime*rate*(0.7 + 0.6*h11(k*2.3 + seed)) + h11(k*7.1 + seed);
    float n = floor(cyc), q = fract(cyc);
    float a = h11(n*3.7 + k*0.9 + seed);
    float y = mix(y0, y1, h11(n*5.1 + k + seed));
    vec3 p = around(0.5 + outR*smoothstep(0.0, 0.5, q) + 0.4*q, a + 0.15*q*q, y + rise*q*q);
    float d = rayPt(ro, rd, p, t);
    float life = smoothstep(0.0, 0.05, q)*(1.0 - smoothstep(0.45, 1.0, q))*(1.0 - smoothstep(uLength - 1.5, uLength + 0.5, p.y));
    acc += (mix(hot, col, smoothstep(0.0, 0.4, q))*halo(d, max(size, px))*1.5 + col*halo(d, size*4.0)*0.35)*life*roomAt(p);
  }
  return acc;
}
// One great tongue of flame in three dimensions (the round's first Mythicals, 1/11/2026, so their
// auras fill the space round the blade): rooted in the blade at height y0 and angle a (turns), it
// leans out up to `lean` units and rises `len`, curling a little round the blade as it climbs;
// broad below its middle and drawn to a soft point, its outline eaten by a slow grid-free field so
// it reads as fire, not a leaf: no bright spine, hot at its root, `mid` through it and `cool` at
// its tip. k is its own seed; the result is its light (add it).
vec3 rxTongue(vec3 ro, vec3 rd, float y0, float a, float len, float lean, float wMax, float k, vec3 hot, vec3 mid, vec3 cool){
  vec3 tip = around(0.4 + lean, a + 0.08, y0 + len);
  vec3 mid3 = around(0.4 + lean*0.7, a + 0.04, y0 + len*0.45);
  float s0;
  if (rayPtRaw(ro, rd, mid3, s0) > abs(len)*0.6 + lean + wMax + 1.0) return vec3(0.0);
  float best = 1e3, bf = 0.0, s;
  // 1/11/2026 (Creyon's FPS): four chords first; past their error and 3.2 widths of the widest
  // glow nothing shows, so the ten segments are skipped
  {
    float coarse = 1e3;
    vec3 cp = around(0.4, a, y0);
    for (int j = 1; j <= 4; j++){
      float f = float(j)/4.0;
      float sway = 0.04*sin(uTime*0.35 + k*1.9 - f*2.2)*f;
      float rr = 0.4 + lean*sin(1.5708*min(f*1.25, 1.0));
      vec3 p = around(rr, a + 0.08*f + sway, y0 + len*f);
      coarse = min(coarse, raySegRaw(ro, rd, cp, p, s));
      cp = p;
    }
    float err = 0.2*abs(lean) + 0.12*(0.4 + abs(lean)) + 0.1;
    if (coarse - err > 3.2*(wMax*1.6 + 0.2) + wMax) return vec3(0.0);
  }
  vec3 prev = around(0.4, a, y0);
  for (int j = 1; j <= 10; j++){
    float f = float(j)/10.0;
    float sway = 0.04*sin(uTime*0.35 + k*1.9 - f*2.2)*f;
    float rr = 0.4 + lean*sin(1.5708*min(f*1.25, 1.0));
    vec3 p = around(rr, a + 0.08*f + sway, y0 + len*f);
    float d = raySeg(ro, rd, prev, p, s);
    if (d < best){ best = d; bf = (float(j) - 1.0 + s)/10.0; }
    prev = p;
  }
  float w = wMax*pow(max(sin(3.1416*pow(bf, 0.65)), 0.0), 0.9);
  float n = rxFlow(vec2(bf*5.0 - uTime*0.45 + k*3.1, best/max(w, 0.05)*1.5 + k));
  float n2 = rxFlow(vec2(bf*11.0 - uTime*0.7 + k*7.3, k*2.0));
  float edge = w*(0.35 + 0.85*n)*(0.75 + 0.35*n2);
  float body = 1.0 - smoothstep(edge*0.15, edge, best);
  body *= body;
  vec3 c = mix(hot, mid, smoothstep(0.05, 0.45, bf));
  c = mix(c, cool, smoothstep(0.45, 0.95, bf));
  float ends = smoothstep(0.0, 0.07, bf)*(1.0 - smoothstep(0.75, 1.0, bf));
  float glow = halo(best, w*1.6 + 0.2)*0.18;
  return (c*body*0.85 + cool*glow)*ends*roomFade(0.4 + lean*sin(1.5708*min(bf*1.25, 1.0)));
}
// A soft cloud of mist, smoke or haze round the blade in three dimensions (1/11/2026, Creyon: the
// lower tiers filled out): marched in 10 steps through a cylinder of radius R round the blade
// between heights y0 and y1, its density a slow rising swirl that thins to nothing at the
// cylinder's wall and at both ends (never an edge), coloured from `a` in its thick parts to `b` in
// its thin ones; `rise` is how fast it drifts up, k chooses it.
vec3 rxMist(vec3 ro, vec3 rd, float y0, float y1, float R, float rise, float k, vec3 a, vec3 b){
  float qa = dot(rd.xz, rd.xz);
  if (qa < 1e-5) return vec3(0.0);
  float qb = dot(ro.xz, rd.xz);
  float qc = dot(ro.xz, ro.xz) - R*R;
  float disc = qb*qb - qa*qc;
  if (disc <= 0.0) return vec3(0.0);
  float sq = sqrt(disc);
  float t0 = (-qb - sq)/qa, t1 = (-qb + sq)/qa;
  t0 = max(t0, 0.0);
  if (t1 <= t0) return vec3(0.0);
  float dt = (t1 - t0)/10.0;
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 10; i++){
    float t = t0 + dt*(float(i) + 0.5);
    vec3 p = ro + rd*t;
    float r = length(p.xz);
    float span = smoothstep(y0, y0 + 2.5, p.y)*(1.0 - smoothstep(y1 - 3.0, y1, p.y));
    if (span <= 0.0) continue;
    // 1/11/2026 (Creyon's FPS): the outer edge needs only n2, so a sample past it (or at the
    // axis, where it is thin to nothing) skips the other wave
    if (r < 0.05*R) continue;
    float n2 = rxFlow(vec2(p.z*0.3 - p.x*0.24 - k*1.7, p.y*0.22 - uTime*rise*1.3 + k));
    if (r >= (0.45 + 0.55*n2)*R) continue;
    float n1 = rxFlow(vec2(p.x*0.32 + p.z*0.21 + k, p.y*0.16 - uTime*rise));
    float dens = smoothstep(0.35, 0.9, n1*0.6 + n2*0.55)*smoothstep(0.05*R, 0.35*R, r)*(1.0 - smoothstep(0.3*R, (0.45 + 0.55*n2)*R, r))*span;
    acc += mix(b, a, smoothstep(0.2, 0.8, dens))*dens*dt;
  }
  return acc*0.4*roomFade(R*0.6);
}
// A soft band of light (a ribbon of water, wind, energy) swept loosely round the blade in three
// dimensions: from radius r0 at height y0 to r1 at y1, turning `turns` of the way round (keep it
// under about 0.6 so it never reads as a ring), w across at its widest and tapering to nothing at
// both ends, its outline gently eaten, light running along it; `hot` at its middle, `cool` at
// its edge.
vec3 rxRibbon(vec3 ro, vec3 rd, float r0, float r1, float turns, float phase, float y0, float y1, float w, float k, vec3 hot, vec3 cool){
  vec3 mid = around(0.5*(r0 + r1), phase + 0.5*turns, 0.5*(y0 + y1));
  float s0;
  if (rayPtRaw(ro, rd, mid, s0) > abs(y1 - y0)*0.6 + max(r0, r1) + w + 1.0) return vec3(0.0);
  float best = 1e3, bf = 0.0, s;
  // 1/11/2026 (Creyon's FPS): four chords first (with the same sway at their ends); the ribbon
  // lies within their sagitta and the sway's bend of them, so a ray past that and 3.2 widths of
  // its widest glow (under a ten-thousandth) draws nothing and skips the fourteen segments
  {
    float coarse = 1e3;
    vec3 cp = around(r0, phase, y0);
    for (int j = 1; j <= 4; j++){
      float f = float(j)/4.0;
      float sway = 0.03*sin(uTime*0.2 + k*2.3 + f*3.0);
      vec3 p = around(mix(r0, r1, f), phase + turns*f + sway, mix(y0, y1, f));
      coarse = min(coarse, raySegRaw(ro, rd, cp, p, s));
      cp = p;
    }
    float rm = max(r0, r1);
    float sag = rm*(1.0 - cos(min(3.1416*abs(turns)*0.25 + 0.05, 3.1416))) + 0.15*rm + 0.1;
    if (coarse - sag > 3.2*(w*1.5 + 0.15) + w) return vec3(0.0);
  }
  vec3 prev = around(r0, phase, y0);
  for (int j = 1; j <= 14; j++){
    float f = float(j)/14.0;
    float sway = 0.03*sin(uTime*0.2 + k*2.3 + f*3.0);
    vec3 p = around(mix(r0, r1, f), phase + turns*f + sway, mix(y0, y1, f));
    float d = raySeg(ro, rd, prev, p, s);
    if (d < best){ best = d; bf = (float(j) - 1.0 + s)/14.0; }
    prev = p;
  }
  float ww = w*pow(max(sin(3.1416*bf), 0.0), 0.7);
  float n = rxFlow(vec2(bf*6.0 - uTime*0.35 + k*2.9, best/max(ww, 0.05) + k));
  float edge = ww*(0.55 + 0.6*n);
  float body = 1.0 - smoothstep(edge*0.1, edge, best);
  body *= body;
  float run = 0.65 + 0.45*sin(bf*14.0 - uTime*0.6 + k*1.7);
  vec3 c = mix(cool, hot, body);
  return (c*body*run + cool*halo(best, ww*1.5 + 0.15)*0.15)*roomFade(mix(r0, r1, bf) + ww);
}
// Tall curtains of light standing round the blade in three dimensions, like an aurora's (1/11/2026):
// on a cylinder of radius R between heights y0 and y1, `n` curtains (arcs a fraction of the way
// round, never a whole ring) whose light falls in fine vertical folds that waver slowly upward;
// soft at their tops, bottoms and sides. c1 is their bright folds, c2 their glow.
vec3 rxAurora(vec3 ro, vec3 rd, float R, float y0, float y1, float n, float k, vec3 c1, vec3 c2){
  float qa = dot(rd.xz, rd.xz);
  if (qa < 1e-5) return vec3(0.0);
  float qb = dot(ro.xz, rd.xz);
  float qc = dot(ro.xz, ro.xz) - R*R;
  float disc = qb*qb - qa*qc;
  if (disc <= 0.0) return vec3(0.0);
  float sq = sqrt(disc);
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 2; i++){
    float t = (-qb + (i == 0 ? -sq : sq))/qa;
    if (t <= 0.0) continue;
    vec3 p = ro + rd*t;
    float h = (p.y - y0)/max(y1 - y0, 0.01);
    if (h <= 0.0 || h >= 1.0) continue;
    float a = atan(p.x, p.z)/6.2832 + 0.5;
    float sheet = sin(3.1416*fract(a*n + 0.02*uTime + k));
    sheet = pow(max(sheet, 0.0), 2.5);
    float fold = rxFlow(vec2(sin(6.2832*a)*6.0 + k*3.0, h*1.5 - uTime*0.12))*0.5 + rxFlow(vec2(cos(6.2832*a)*6.0 - k, h*1.2 - uTime*0.1))*0.5;
    float folds = 0.35 + 0.9*pow(fold, 2.0);
    float v = smoothstep(0.0, 0.25, h)*(1.0 - smoothstep(0.55, 1.0, h));
    // grazing views see more of the sheet: kept gentle so the silhouette never turns into an edge
    float graze = 1.0/(0.6 + abs(dot(normalize(p.xz), normalize(rd.xz))));
    acc += mix(c2, c1, fold)*sheet*folds*v*min(graze, 1.6);
  }
  return acc*1.3*roomFade(R*0.85);
}
// A cloud of `count` (up to 40) tiny twinkling stars round the blade in three dimensions, out to
// radius R between y0 and y1, turning slowly round it; `size` across.
vec3 rxStarCloud(vec3 ro, vec3 rd, float px, float count, float R, float y0, float y1, float size, float k, vec3 c1, vec3 c2){
  vec3 acc = vec3(0.0);
  float t;
  // a ray passing further from the cloud than its widest glow draws nothing of it
  if (fxNear(ro, rd, y0 - 1.0, y1 + 1.0) > R + 3.0*size*6.0 + 3.0*px) return acc;
  for (int i = 0; i < 40; i++){
    float m = float(i);
    if (m >= count) break;
    float r = R*sqrt(0.08 + 0.92*h11(m*3.7 + k));
    vec3 p = around(r, h11(m*5.3 + k) + uTime*(0.003 + 0.004*h11(m*1.9 + k))*min(R/max(r, 0.5), 2.0), mix(y0, y1, h11(m*7.1 + k)));
    // 1/11/2026 (Creyon's FPS): a star the ray passes further from than 3.2 widths of its glow
    // adds under a ten-thousandth: skipped before its twinkle and hiding are worked out
    if (rayPtRaw(ro, rd, p, t) > 3.2*max(size*6.0, px)) continue;
    float tw = 0.35 + 0.65*pow(0.5 + 0.5*sin(uTime*(0.8 + 1.2*h11(m*2.2 + k)) + m*4.0), 3.0);
    float d = rayPt(ro, rd, p, t);
    acc += (c1*halo(d, max(size, px))*2.2 + c2*halo(d, size*6.0)*0.4)*tw*roomAt(p);
  }
  return acc;
}
// rxMist turning round the blade (`spin` turns a second) and widening with height, from radius
// R0 at y0 to R1 at y1: a slow vortex of mist.
// (update 16) the turned mist's angle comes as a phase in turns, so a caller whose spin changes
// passes its integral; rxMistSpin keeps the old steady spin
vec3 rxMistSpinP(vec3 ro, vec3 rd, float y0, float y1, float R0, float R1, float phase, float k, vec3 a, vec3 b){
  float R = max(R0, R1);
  float qa = dot(rd.xz, rd.xz);
  if (qa < 1e-5) return vec3(0.0);
  float qb = dot(ro.xz, rd.xz);
  float qc = dot(ro.xz, ro.xz) - R*R;
  float disc = qb*qb - qa*qc;
  if (disc <= 0.0) return vec3(0.0);
  float sq = sqrt(disc);
  float t0 = max((-qb - sq)/qa, 0.0), t1 = (-qb + sq)/qa;
  if (t1 <= t0) return vec3(0.0);
  float dt = (t1 - t0)/10.0;
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 10; i++){
    vec3 p = ro + rd*(t0 + dt*(float(i) + 0.5));
    float h = (p.y - y0)/max(y1 - y0, 0.01);
    if (h <= 0.0 || h >= 1.0) continue;
    float r = length(p.xz);
    float Rh = mix(R0, R1, h);
    float ang = atan(p.x, p.z)/6.2832 - phase*(1.2 - 0.4*h);
    if (r < 0.05*Rh) continue;
    float n2 = rxFlow(vec2(cos(6.2832*ang)*1.6 - r*0.3 - k, h*4.0 + k));
    if (r >= (0.45 + 0.55*n2)*Rh) continue;
    float n1 = rxFlow(vec2(sin(6.2832*ang)*1.6 + r*0.4 + k, h*3.0 - uTime*0.1));
    float swirl = 0.5 + 0.5*sin(6.2832*(ang*2.0 + r*0.25));
    float dens = smoothstep(0.3, 0.9, n1*0.55 + n2*0.4 + swirl*0.3)*smoothstep(0.05*Rh, 0.35*Rh, r)*(1.0 - smoothstep(0.3*Rh, (0.45 + 0.55*n2)*Rh, r))
               *smoothstep(0.0, 0.15, h)*(1.0 - smoothstep(0.75, 1.0, h));
    acc += mix(b, a, smoothstep(0.2, 0.8, dens))*dens*dt;
  }
  return acc*0.4*roomFade(R*0.6);
}
vec3 rxMistSpin(vec3 ro, vec3 rd, float y0, float y1, float R0, float R1, float spin, float k, vec3 a, vec3 b){
  return rxMistSpinP(ro, rd, y0, y1, R0, R1, uTime*spin*0.5, k, a, b);
}
// The round's clock (1/11/2026, Creyon's update-12 test: everything felt like slow motion, so it
// keeps real time again; the round's spins were slowed where they are set instead). It was a calm
// clock (1/11/2026, Creyon: "all blades have fast effects in them when
// rotating; it all must go fluent and smooth at a comfy pace"): a little over half real time.
// The round's first blades ran their own motion on it (the shells below use real time); their cycles keep real time.
float tCalm(){ return uTime; }
// A ki aura in three dimensions (1/11/2026): an egg-shaped shell of flame round the blade from y0
// to y1, R at its widest, narrow at its foot and drawn to a flickering crown at its top, its
// light streaming upward in long licks; brightest at its rim (as an aura is drawn, a glowing
// outline with a softer inside), every edge soft. `inner` fills it, `rim` lights its edge.
vec3 rxAuraShell(vec3 ro, vec3 rd, float y0, float y1, float R, float k, vec3 inner, vec3 rim){
  float qa = dot(rd.xz, rd.xz);
  if (qa < 1e-5) return vec3(0.0);
  float qb = dot(ro.xz, rd.xz);
  float qc = dot(ro.xz, ro.xz) - R*R*1.3;
  float disc = qb*qb - qa*qc;
  if (disc <= 0.0) return vec3(0.0);
  float sq = sqrt(disc);
  float t0 = max((-qb - sq)/qa, 0.0), t1 = (-qb + sq)/qa;
  if (t1 <= t0) return vec3(0.0);
  float dt = (t1 - t0)/12.0;
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 12; i++){
    vec3 p = ro + rd*(t0 + dt*(float(i) + 0.5));
    float h = (p.y - y0)/max(y1 - y0, 0.01);
    if (h <= -0.05 || h >= 1.15) continue;
    float r = length(p.xz);
    float ang = atan(p.x, p.z);
    // its outline: narrow at the foot, widest two thirds up, a crown of licks at the top
    float lick = rxFlow(vec2(sin(ang)*3.2 + cos(ang)*1.9 + k, h*8.0 - uTime*0.9));
    float lick2 = rxFlow(vec2(cos(ang)*4.1 - sin(ang)*1.7 - k, h*11.0 - uTime*1.2));
    float Rh = R*pow(max(sin(3.1416*clamp(h*0.85 + 0.05, 0.0, 1.0)), 0.0), 0.55)*(0.55 + 0.75*lick);
    float crown = smoothstep(0.75, 1.1, h)*(0.4 + 0.8*lick2);
    float top = 1.0 - smoothstep(0.9 + 0.2*lick2, 1.12, h);
    float shell = exp(-pow((r - Rh*0.85)/max(Rh*0.13, 0.05), 2.0));
    float fill = (1.0 - smoothstep(0.2*Rh, Rh, r))*0.06;
    float streak = 0.15 + 1.4*pow(lick2, 2.5);
    float dens = (shell*streak + fill + crown*shell*0.5)*top*smoothstep(-0.05, 0.08, h);
    acc += mix(inner, rim, shell)*dens*dt;
  }
  return acc*0.45*roomFade(R*0.75);
}
// A shower of sparks thrown out from p on ballistic arcs (falling back as real sparks do), the
// burst q (0 to 1) after it was struck: `count` (up to 24) sparks, flying out about `speed`
// units, white-hot cooling to `col`, each a short bright streak along its path.
vec3 rxSparkShower(vec3 ro, vec3 rd, float px, vec3 p0, float q, float count, float speed, float k, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  if (q < 0.0 || q > 1.0) return acc;
  float s, t0;
  // the sparks fly at most `speed` units out (and fall): past that and their glow, nothing
  if (rayPtRaw(ro, rd, p0, t0) > speed*1.3 + 9.0*q*q + 1.0) return acc;
  for (int i = 0; i < 24; i++){
    float m = float(i);
    if (m >= count) break;
    float a = h11(m*3.1 + k)*6.2832;
    float up = 0.2 + 0.9*h11(m*5.7 + k);
    vec3 v = vec3(cos(a), up, sin(a))*speed*(0.6 + 0.5*h11(m*7.3 + k));
    float tq = q*(0.7 + 0.3*h11(m*2.9 + k));
    vec3 p = p0 + v*tq - vec3(0.0, 9.0*tq*tq, 0.0);
    float tb = max(tq - 0.06, 0.0);
    vec3 b = p0 + v*tb - vec3(0.0, 9.0*tb*tb, 0.0);
    float d = raySeg(ro, rd, b, p, s);
    float life = (1.0 - smoothstep(0.5, 1.0, tq))*smoothstep(0.0, 0.03, q);
    vec3 c = mix(hot, col, smoothstep(0.0, 0.5, tq));
    acc += (c*halo(d, max(0.035, px))*1.6*s + col*halo(d, 0.25)*0.25)*life*roomAt(p);
  }
  return acc;
}
// A comet sweeping slowly round the blade: its head a bright point, its tail a long soft streak
// behind it along its path (radius r, height from y0 rising `climb` over its pass), passing every
// `period` seconds; nothing at all between passes.
vec3 rxComet(vec3 ro, vec3 rd, float px, float r, float y0, float climb, float period, float k, vec3 head, vec3 tail){
  float c = mod(uTime + k*7.3, period);
  float q = c/min(period*0.5, 9.0);
  if (q >= 1.0) return vec3(0.0);
  float life = sin(3.1416*q);
  float a0 = h11(k*4.1 + floor((uTime + k*7.3)/period));
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(r, a0 + 0.45*(q - 0.12), y0 + climb*(q - 0.12));
  for (int j = 1; j <= 8; j++){
    float f = float(j)/8.0;
    float qq = q - 0.12*(1.0 - f);
    vec3 p = around(r, a0 + 0.45*qq, y0 + climb*qq);
    float d = raySeg(ro, rd, prev, p, s);
    if (d < best){ best = d; bf = (float(j) - 1.0 + s)/8.0; }
    prev = p;
  }
  vec3 hp = around(r, a0 + 0.45*q, y0 + climb*q);
  float t;
  float dh = rayPt(ro, rd, hp, t);
  vec3 col = head*halo(dh, max(0.12, px))*2.0 + head*halo(dh, 0.6)*0.4 + tail*halo(best, 0.05 + 0.25*bf)*pow(bf, 1.5)*0.9;
  return col*life*roomAt(hp);
}
// Real smoke (1/11/2026): rxMist's cloud as a solid layer (for `sig_vol_solid`), so it can be dark
// smoke that hides what is behind it rather than light added over it. Returns its colour and
// how much it covers.
vec4 rxSmoke(vec3 ro, vec3 rd, float y0, float y1, float R, float rise, float k, vec3 col, float thick){
  float qa = dot(rd.xz, rd.xz);
  if (qa < 1e-5) return vec4(0.0);
  float qb = dot(ro.xz, rd.xz);
  float qc = dot(ro.xz, ro.xz) - R*R;
  float disc = qb*qb - qa*qc;
  if (disc <= 0.0) return vec4(0.0);
  float sq = sqrt(disc);
  float t0 = max((-qb - sq)/qa, 0.0), t1 = (-qb + sq)/qa;
  if (t1 <= t0) return vec4(0.0);
  float dt = (t1 - t0)/10.0;
  float sum = 0.0;
  for (int i = 0; i < 10; i++){
    vec3 p = ro + rd*(t0 + dt*(float(i) + 0.5));
    float r = length(p.xz);
    float span = smoothstep(y0, y0 + 2.5, p.y)*(1.0 - smoothstep(y1 - 4.0, y1, p.y));
    float n1 = rxFlow(vec2(p.x*0.3 + p.z*0.2 + k, p.y*0.14 - uTime*rise));
    float n2 = rxFlow(vec2(p.z*0.28 - p.x*0.22 - k*1.7, p.y*0.2 - uTime*rise*1.3 + k));
    float dens = smoothstep(0.4, 0.9, n1*0.6 + n2*0.55)*smoothstep(0.18*R, 0.42*R, r)*(1.0 - smoothstep(0.5*R, (0.65 + 0.35*n2)*R, r))*span;
    sum += dens*dt;
  }
  float a = (1.0 - exp(-sum*thick))*roomFade(R*0.6);
  return vec4(col, a);
}
// Floating rocks (1/11/2026): `count` (up to 16) lumps of stone hanging round the blade in a
// solid layer, out to radius R between y0 and y1, drifting slowly round and bobbing, each lit
// from above with a rim of `rim` light on its edge facing the blade. Colour and cover of the
// nearest.
vec4 rxRocks(vec3 ro, vec3 rd, float px, float count, float R, float y0, float y1, float size, float k, vec3 stone, vec3 rim){
  vec4 best = vec4(0.0);
  float bestT = 1e9, t;
  for (int i = 0; i < 16; i++){
    float m = float(i);
    if (m >= count) break;
    float r = R*(0.45 + 0.55*h11(m*3.7 + k));
    vec3 p = around(r, h11(m*5.3 + k) + uTime*(0.004 + 0.004*h11(m*1.9 + k)), mix(y0, y1, h11(m*7.1 + k)) + 0.3*sin(uTime*0.4 + m));
    float rad = size*(0.6 + 0.7*h11(m*2.2 + k));
    float d = rayPt(ro, rd, p, t);
    if (d > rad || t > bestT) continue;
    // a lumpy outline: the radius wobbles round its edge
    vec3 o = (ro + rd*t) - p;
    float ang = atan(o.y, length(o.xz) + 1e-4) + atan(o.x, o.z);
    float lump = rad*(0.82 + 0.18*sin(ang*3.0 + m*2.0)*sin(ang*2.0 + m));
    float cover = 1.0 - smoothstep(lump - max(px, 0.03), lump, d);
    if (cover <= 0.0) continue;
    vec3 n = normalize(o - rd*sqrt(max(lump*lump - d*d, 0.0)));
    float lit = 0.25 + 0.75*clamp(dot(n, normalize(vec3(0.2, 1.0, 0.3))), 0.0, 1.0);
    float edge = pow(clamp(dot(n, normalize(-vec3(p.x, 0.0, p.z) + vec3(1e-4))), 0.0, 1.0), 3.0);
    vec3 c = stone*lit + rim*edge*0.9;
    best = vec4(c, cover*roomAt(p));
    bestT = t;
  }
  return best;
}
// rxTongue's flame as a solid layer (1/11/2026): its body's cover and colour, for flames that are
// dark (black fire, shadow) and must hide what is behind them; `edge` lights its outline.
vec4 rxTongueSolid(vec3 ro, vec3 rd, float y0, float a, float len, float lean, float wMax, float k, vec3 body, vec3 edge){
  vec3 mid3 = around(0.4 + lean*0.7, a + 0.04, y0 + len*0.45);
  float s0;
  if (rayPtRaw(ro, rd, mid3, s0) > abs(len)*0.6 + lean + wMax + 1.0) return vec4(0.0);
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(0.4, a, y0);
  for (int j = 1; j <= 10; j++){
    float f = float(j)/10.0;
    float sway = 0.04*sin(uTime*0.3 + k*1.9 - f*2.2)*f;
    float rr = 0.4 + lean*sin(1.5708*min(f*1.25, 1.0));
    vec3 p = around(rr, a + 0.08*f + sway, y0 + len*f);
    float d = raySeg(ro, rd, prev, p, s);
    if (d < best){ best = d; bf = (float(j) - 1.0 + s)/10.0; }
    prev = p;
  }
  float w = wMax*pow(max(sin(3.1416*pow(bf, 0.65)), 0.0), 0.9);
  float n = rxFlow(vec2(bf*5.0 - uTime*0.4 + k*3.1, best/max(w, 0.05)*1.5 + k));
  float outline = w*(0.35 + 0.85*n);
  float cover = 1.0 - smoothstep(outline*0.7, outline, best);
  float rim = halo(abs(best - outline*0.85), max(outline*0.12, 0.04));
  float ends = smoothstep(0.0, 0.07, bf)*(1.0 - smoothstep(0.8, 1.0, bf));
  float r = 0.4 + lean*sin(1.5708*min(bf*1.25, 1.0));
  vec3 c = mix(body, edge, clamp(rim*1.2 - cover*0.3, 0.0, 1.0));
  return vec4(c, clamp(max(cover*0.92, rim*0.8), 0.0, 1.0)*ends*roomFade(r));
}
// Light rays fanning out of the blade (1/11/2026): `n` (up to 12) long soft shafts of light leaving
// it at a slant all round, each swelling and fading slowly, as starlight breaks out of glass.
vec3 rxRays(vec3 ro, vec3 rd, float px, float n, float y0, float y1, float reach, float k, vec3 c1, vec3 c2){
  vec3 acc = vec3(0.0);
  float s;
  for (int i = 0; i < 12; i++){
    float m = float(i);
    if (m >= n) break;
    float y = mix(y0, y1, fract(m*0.618 + k));
    float a = m/n + 0.1*h11(m*3.1 + k);
    float tm;
    if (rayPtRaw(ro, rd, around(reach*0.4, a, y + reach*0.17), tm) > reach*0.65 + 2.0) continue;
    float life = 0.5 + 0.5*sin(uTime*0.35 + m*2.3 + k);
    vec3 pa = around(0.4, a, y);
    vec3 pc = around(reach*(0.7 + 0.3*h11(m*5.3 + k)), a, y + reach*0.35);
    float d = raySeg(ro, rd, pa, pc, s);
    float w = 0.08 + 0.5*s;
    acc += (c1*halo(d, max(w*0.35, px))*0.7 + c2*halo(d, w)*0.25)*(1.0 - s)*life*roomAt(pc);
  }
  return acc;
}
// ---- Creyon's Star Wars round, second pass (1/11/2026): shells ----
// Creyon's review: the round's effects looked cheap, ran at different speeds and cost frames
// (Zeffo Thunderchorus fell from 125 to 30 frames a second up close, its loops of 3D bolts). Its
// effects are now laid on shells: the view ray meets a cylinder of radius R round the blade at most
// twice (before the blade and behind it), and a pattern is read at those two points in the shell's
// own coordinates (a turns round it, y up it). The same thing from every side and from above, a few
// look-ups a pixel instead of a loop over pieces, and faded out where the ray grazes the shell, so
// no outline, edge or bound ever shows. Every speed is in units (or turns) a second of real time,
// within one calm range for the whole round (`SH_RISE`, `SH_SPIN`).
const float SH_RISE = 3.2;
const float SH_SPIN = 0.045;
// The ray's two crossings of the shell of radius R (t of each; negative: none).
vec2 shT(vec3 ro, vec3 rd, float R){
  float a = dot(rd.xz, rd.xz);
  if (a < 1e-5) return vec2(-1.0);
  float b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - R*R;
  float h = b*b - a*c;
  if (h <= 0.0) return vec2(-1.0);
  h = sqrt(h);
  return vec2((-b - h)/a, (-b + h)/a);
}
// How much of the crossing at t is seen: none behind the camera or hidden by the blade's body,
// fading where the ray grazes the shell and over `e` units at both ends (heights y0 to y1). q
// returns its place on the shell: turns round (0 to 1) and height.
float shAt(vec3 ro, vec3 rd, float R, float t, float y0, float y1, float e, out vec2 q){
  q = vec2(0.0);
  if (t <= 0.0) return 0.0;
  vec3 p = ro + rd*t;
  if (p.y <= y0 || p.y >= y1) return 0.0;
  float graze = abs(dot(p.xz, rd.xz))/(R*max(length(rd), 1e-4));
  float w = smoothstep(0.08, 0.65, graze)*smoothstep(y0, y0 + e, p.y)*(1.0 - smoothstep(y1 - e, y1, p.y));
  if (w <= 0.0) return 0.0;
  w *= 1.0 - behindBlade(ro, rd, t);
  q = vec2(fract(atan(p.x, p.z)/6.2832 + 1.0), p.y);
  return w;
}
// A smooth noise round the shell with no seam: n is read on the circle (so it closes on itself)
// and up the height.
float shNoise(vec2 q, float k, float s, float v){
  float A = q.x*6.2832;
  float Y = q.y*s - uTime*v;
  return rxFlow(vec2(cos(A)*k + 0.37*Y, Y + sin(A)*k));
}
// Motes: soft points of light drifting on the shell, `n` cells round it and cells `ch` tall,
// rising at `rise` units a second and turning `spin` turns a second; a share `fill` of the cells
// hold one, each fading in and out on its own (`rate` lives a second), `size` units across, hot
// at its heart. Each stays inside its cell (its glow is a quarter of the cell at most).
vec3 shMotes(vec3 ro, vec3 rd, float R, float y0, float y1, float n, float ch, float rise, float spin, float size, float fill, float rate, float seed, vec3 hot, vec3 col){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  float circ = 6.2832*R;
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 1.5, q);
    if (w <= 0.001) continue;
    float a = fract(q.x - spin*uTime)*n;
    float y = (q.y - rise*uTime)/ch;
    vec2 cell = floor(vec2(a, y));
    cell.x = mod(cell.x, n);
    vec2 h = h22(cell + vec2(seed*13.1, seed*7.7));
    if (h.x > fill) continue;
    vec2 c = vec2(0.3 + 0.4*h.y, 0.3 + 0.4*fract(h.x*17.3));
    vec2 f = (fract(vec2(a, y)) - c)*vec2(circ/n, ch);
    float d = length(f);
    float life = fract(uTime*rate + h.y*7.0);
    float b = pow(sin(3.1416*life), 2.0);
    acc += (hot*exp(-pow(d/size, 2.0)) + col*exp(-pow(d/(size*2.2), 2.0))*0.5)*b*w;
  }
  return acc;
}
// Zigzag 1D: straight runs between kinks every 1/k (a real bolt's look, not a smooth wave).
float shZig(float s, float k, float seed){
  float x = s*k, i = floor(x);
  return mix(h11(i + seed) - 0.5, h11(i + 1.0 + seed) - 0.5, fract(x));
}
// Lightning crawling on the shell: up to three bolts at a time, each `len` units long, struck at a
// place of its own and living 1/rate seconds (a quick flicker at its birth, then fading); jagged
// by `jag` (units), with a fork from near its middle. A white core a hair wide and a coloured glow.
vec3 shBolts(vec3 ro, vec3 rd, float px, float R, float y0, float y1, float count, float len, float rate, float jag, float seed, vec3 hot, vec3 col){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  float circ = 6.2832*R;
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 1.0, q);
    if (w <= 0.001) continue;
    for (int k = 0; k < 3; k++){
      float m = float(k);
      if (m >= count) break;
      float clock = uTime*rate + m*0.37 + seed;
      float slot = floor(clock), age = fract(clock);
      float hs = h11(slot*3.7 + m*11.0 + seed);
      float ys = mix(y0, max(y1 - len, y0), hs);
      float s = (q.y - ys)/len;
      if (s < 0.0 || s > 1.0) continue;
      float a0 = h11(slot*5.3 + m*7.0 + seed*3.0);
      float lean = (h11(slot*9.1 + m) - 0.5)*0.8*len;
      float du = (fract(q.x - a0 + 0.5) - 0.5)*circ;
      float x = lean*s + jag*(shZig(s, 7.0, slot*13.0 + m) + 0.45*shZig(s, 17.0, slot*29.0 + m))*sin(3.1416*s);
      float d = abs(du - x);
      // the fork: from about the middle, leaning the other way
      float sf = (s - 0.45)/0.4;
      if (sf > 0.0 && sf < 1.0){
        float xf = x + (lean < 0.0 ? 1.0 : -1.0)*0.35*len*sf*sf + jag*0.6*shZig(sf, 6.0, slot*41.0 + m)*sin(3.1416*sf);
        d = min(d, abs(du - xf) + 0.04);
      }
      float ends = sin(3.1416*s);
      float fl = (0.7 + 0.3*step(0.5, fract(clock*9.0)))*(1.0 - age)*(1.0 - age)*smoothstep(0.0, 0.04, age);
      float core = max(0.06, px*0.7);
      acc += (hot*exp(-pow(d/core, 2.0))*1.5 + col*exp(-pow(d/(core*5.0), 2.0))*0.9 + col*exp(-d/0.8)*0.22)*fl*ends*w;
    }
  }
  return acc;
}
// A sheath of flowing light on the shell: smooth wisps (warped waves read round it with no seam)
// rising at `rise` units a second; `k` sets how many round it, `s` how tall they are (smaller:
// taller), `thr` how much is lit; c1 at their brightest, c2 where they thin.
vec3 shSheath(vec3 ro, vec3 rd, float R, float y0, float y1, float k, float s, float rise, float thr, float seed, vec3 c1, vec3 c2){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 3.0, q);
    if (w <= 0.001) continue;
    q.x += seed;
    float n = shNoise(q, k, s, rise*s) * 0.65 + shNoise(q + vec2(0.31, 0.0), k*2.1, s*2.3, rise*s*2.3)*0.35;
    float v = smoothstep(thr, thr + 0.3, n);
    // (update 21) thin before the shell's limb, no tall hard walls
    acc += mix(c2, c1, smoothstep(0.3, 1.0, v))*v*w*w;
  }
  return acc;
}
// Strands of light wound loosely round the blade: `n` of them, turning `twist` turns a unit of
// height (keep it small: a gentle lean, never a coil) and `spin` turns a second, `w` units across,
// light running up them at `rise`.
vec3 shStrands(vec3 ro, vec3 rd, float R, float y0, float y1, float n, float twist, float spin, float w, float rise, float seed, vec3 hot, vec3 col){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  float circ = 6.2832*R;
  for (int i = 0; i < 2; i++){
    vec2 q;
    float wt = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 3.0, q);
    if (wt <= 0.001) continue;
    float a = q.x*n - (twist*q.y + spin*uTime + seed)*n;
    float d = (fract(a) - 0.5)/n*circ;
    float lane = floor(a);
    float run = rxFlow(vec2(q.y*0.18 - uTime*rise*0.18 + lane*2.7, lane*1.3 + seed));
    float b = smoothstep(0.25, 0.85, run);
    acc += (hot*exp(-pow(d/(w*0.3), 2.0)) + col*exp(-pow(d/w, 2.0))*0.6)*b*wt;
  }
  return acc;
}
// Fire on the shell (for the fire blades only): rising warped noise, cut into flames by height
// inside each `band` of the blade and coloured by its heat, from deep red through orange to a
// white-gold heart; soft everywhere, no tongues or outlines.
vec3 shFire(vec3 ro, vec3 rd, float R, float y0, float y1, float k, float rise, float seed, vec3 deep, vec3 mid, vec3 hot){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 2.0, q);
    if (w <= 0.001) continue;
    q.x += seed;
    float warp = shNoise(q, k*0.7, 0.09, rise*0.09);
    float n = shNoise(q + vec2(warp*0.15, warp*1.5), k, 0.16, rise*0.16);
    float n2 = shNoise(q + vec2(0.5, 3.0), k*2.3, 0.33, rise*0.33);
    float heat = n*0.7 + n2*0.4 - 0.25;
    heat = clamp(heat, 0.0, 1.0);
    vec3 c = mix(deep, mid, smoothstep(0.1, 0.45, heat));
    c = mix(c, hot, smoothstep(0.5, 0.8, heat));
    acc += c*smoothstep(0.08, 0.6, heat)*w;
  }
  return acc;
}
// Ridged veins crackling on the shell: thin bright filaments where two waves cross zero, the
// pattern jumping to a new one `rate` times a second with a quick cross-fade (crackle), lit only
// where a slow mask lets them.
vec3 shVeins(vec3 ro, vec3 rd, float px, float R, float y0, float y1, float k, float rate, float cover, float seed, vec3 hot, vec3 col){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 2.0, q);
    if (w <= 0.001) continue;
    float clock = uTime*rate + seed;
    float slot = floor(clock);
    vec2 qq = q + vec2(h11(slot)*0.5, h11(slot + 3.0)*20.0);
    float r = abs(shNoise(qq, k, 0.35, 0.0) - 0.5);
    float line = exp(-pow(r/0.022, 2.0));
    float mask = smoothstep(1.0 - cover, 1.0 - cover + 0.25, shNoise(q + vec2(0.3, 7.0), 1.3, 0.06, 0.25));
    float fl = 1.0 - fract(clock);
    acc += (hot*line*line*1.2 + col*line*0.5)*mask*(0.35 + 0.65*fl)*w;
  }
  return acc;
}
// Streaks falling (rise < 0) or climbing (rise > 0) on the shell: rain, sparks thrown upward,
// starlines; `n` lanes round it, each streak `len` units long.
vec3 shStreaks(vec3 ro, vec3 rd, float R, float y0, float y1, float n, float len, float rise, float fill, float seed, vec3 hot, vec3 col){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  float circ = 6.2832*R;
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 2.0, q);
    if (w <= 0.001) continue;
    float a = q.x*n;
    float lane = mod(floor(a), n);
    float hl = h11(lane*7.13 + seed);
    float sp = rise*(0.7 + 0.6*hl);
    float y = q.y - sp*uTime - hl*40.0;
    float gap = len*3.5;
    float cell = floor(y/gap);
    float hc = h11(cell*3.1 + lane*1.7 + seed);
    if (hc > fill) continue;
    float f = fract(y/gap)*gap;
    float head = rise > 0.0 ? len : 0.0;
    float along = rise > 0.0 ? f/len : 1.0 - f/len;
    float inside = step(0.0, f)*step(f, len);
    float du = (fract(a) - 0.5)/n*circ;
    float wd = 0.035 + 0.03*along;
    float body = exp(-pow(du/wd, 2.0))*inside*along*along;
    acc += (hot*body + col*exp(-pow(du/(wd*5.0), 2.0))*inside*along*0.35)*w;
  }
  return acc;
}
// Curtains on the shell: soft folds standing up it like an aurora's, swaying slowly.
vec3 shCurtain(vec3 ro, vec3 rd, float R, float y0, float y1, float n, float sway, float seed, vec3 c1, vec3 c2){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 4.0, q);
    if (w <= 0.001) continue;
    float m = shNoise(q + vec2(seed, 0.0), 1.2, 0.05, sway);
    float f = 0.5 + 0.5*sin(6.2832*(q.x*n + seed) + 3.0*m + uTime*sway*2.0);
    float fold = pow(f, 3.0);
    float up = smoothstep(0.2, 0.9, shNoise(q + vec2(0.2, 5.0), 2.0, 0.12, 0.4));
    acc += mix(c2, c1, fold)*(fold*0.8 + 0.2)*(0.4 + 0.6*up)*w;
  }
  return acc;
}
// Caustics: the light that water throws, a shifting net of bright lines on the shell.
vec3 shCaustic(vec3 ro, vec3 rd, float R, float y0, float y1, float k, float v, float seed, vec3 hot, vec3 col){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 3.0, q);
    if (w <= 0.001) continue;
    float A = (q.x + seed)*6.2832;
    vec2 p = vec2(cos(A), sin(A))*k + vec2(q.y*0.25, -q.y*0.21);
    float c = 0.0;
    c += sin(p.x*2.0 + uTime*v + sin(p.y*1.7 - uTime*v*0.7));
    c += sin(p.y*2.3 - uTime*v*0.8 + sin(p.x*1.9 + uTime*v*0.6));
    c += sin((p.x + p.y)*1.6 + uTime*v*0.5);
    float l = exp(-pow(abs(c)/0.35, 2.0));
    acc += (hot*l*l + col*l*0.5)*w;
  }
  return acc;
}
// Facets: a shell of crystal cells (periodic round the blade, so no seam), each glinting in its
// own slow turn, their edges a faint light.
vec3 shFacets(vec3 ro, vec3 rd, float R, float y0, float y1, float n, float ch, float seed, vec3 hot, vec3 col){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 3.0, q);
    if (w <= 0.001) continue;
    vec2 g = vec2(q.x*n, q.y/ch);
    vec2 ci = floor(g), f = fract(g);
    float d1 = 8.0, d2 = 8.0; vec2 best = vec2(0.0);
    for (int y = -1; y <= 1; y++){
      for (int x = -1; x <= 1; x++){
        vec2 o = vec2(float(x), float(y));
        vec2 c = ci + o;
        vec2 cw = vec2(mod(c.x, n), c.y);
        vec2 p = o + 0.15 + 0.7*h22(cw + seed) - f;
        float d = dot(p, p);
        if (d < d1){ d2 = d1; d1 = d; best = cw; } else if (d < d2){ d2 = d; }
      }
    }
    float edge = exp(-pow((sqrt(d2) - sqrt(d1))/0.06, 2.0));
    float glint = pow(0.5 + 0.5*sin(uTime*0.6 + h21(best + seed)*6.2832), 8.0);
    acc += (hot*glint*0.8 + col*(edge*0.7 + 0.08))*w;
  }
  return acc;
}
// A soft glow round the axis from y0 to y1 (the air lit round the blade), w its width, its light
// breathing up it in slow swells (no bands: long soft swells only).
vec3 shAir(vec3 ro, vec3 rd, float y0, float y1, float w, float v, vec3 col){
  float s;
  float d = raySegRaw(ro, rd, vec3(0.0, y0, 0.0), vec3(0.0, y1, 0.0), s);
  float y = mix(y0, y1, s);
  float swell = 0.65 + 0.35*sin(y*0.18 - uTime*v);
  float ends = smoothstep(0.0, 0.12, s)*(1.0 - smoothstep(0.85, 1.0, s));
  return col*exp(-pow(d/w, 2.0))*swell*ends;
}
// How far up the blade its effects have come as it lights (`on` seconds) and how they leave it
// as it goes out (`off` seconds): 0 to the blade's length; nothing past the point.
float shTop(float on, float off){
  if (uAge >= 2000.0) return uLength*(1.0 - smoothstep(0.0, off, uAge - 2000.0));
  return uLength*smoothstep(0.0, on, uAge);
}
// The bright front where the effects are coming to life (or going out), at height `top`.
float shFront(float y, float top, float w){
  float live = uAge >= 2000.0 ? 1.0 - smoothstep(0.0, 1.2, uAge - 2000.0) : 1.0 - smoothstep(0.8, 2.5, uAge);
  return exp(-pow((y - top)/w, 2.0))*live;
}
// Flakes: small soft shapes tumbling on the shell (ash, snow, leaves, petals, sand): `n` cells
// round it, cells `ch` tall, moving `rise` units a second (negative: falling) and `spin` turns a
// second round it; each an ellipse `size` long and `size*thin` wide, turning slowly about itself
// and glinting as it turns, so a flake is never a dot.
vec3 shFlakes(vec3 ro, vec3 rd, float R, float y0, float y1, float n, float ch, float rise, float spin, float size, float thin, float fill, float seed, vec3 lit, vec3 col){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  float circ = 6.2832*R;
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 2.0, q);
    if (w <= 0.001) continue;
    vec2 h0 = h22(vec2(floor(q.y/ch), seed));
    float a = fract(q.x - spin*uTime + 0.13*sin(q.y*0.21 + uTime*0.6))*n;
    float y = (q.y - rise*uTime)/ch;
    vec2 cell = floor(vec2(a, y));
    cell.x = mod(cell.x, n);
    vec2 h = h22(cell + vec2(seed*5.3, seed*2.9));
    if (h.x > fill) continue;
    vec2 c = vec2(0.3 + 0.4*h.y, 0.3 + 0.4*fract(h.x*13.7));
    vec2 f = (fract(vec2(a, y)) - c)*vec2(circ/n, ch);
    float ang = h.y*6.2832 + uTime*(0.5 + h.x)*(h.y > 0.5 ? 1.0 : -1.0);
    float cs = cos(ang), sn = sin(ang);
    vec2 r = vec2(cs*f.x + sn*f.y, -sn*f.x + cs*f.y);
    float d = length(r/vec2(size, size*thin));
    float body = exp(-d*d*1.6);
    float glint = 0.55 + 0.45*pow(abs(cs), 6.0);
    acc += (mix(col, lit, glint)*body + col*exp(-pow(length(f)/(size*1.8), 2.0))*0.12)*w;
  }
  return acc;
}
// Glints: sharp sparkles (metal, crystal, starlight) that flash and go on the shell, each a short
// bright cross-light of two soft streaks, at most a few lit at once.
vec3 shGlints(vec3 ro, vec3 rd, float R, float y0, float y1, float n, float ch, float size, float rate, float fill, float seed, vec3 hot, vec3 col){
  vec2 t = shT(ro, rd, R);
  vec3 acc = vec3(0.0);
  float circ = 6.2832*R;
  for (int i = 0; i < 2; i++){
    vec2 q;
    float w = shAt(ro, rd, R, i == 0 ? t.x : t.y, y0, y1, 2.0, q);
    if (w <= 0.001) continue;
    float a = fract(q.x - 0.01*uTime)*n;
    float y = q.y/ch;
    vec2 cell = floor(vec2(a, y));
    cell.x = mod(cell.x, n);
    vec2 h = h22(cell + vec2(seed*3.1, seed*8.3));
    if (h.x > fill) continue;
    vec2 c = vec2(0.3 + 0.4*h.y, 0.3 + 0.4*fract(h.x*11.3));
    vec2 f = (fract(vec2(a, y)) - c)*vec2(circ/n, ch);
    float life = fract(uTime*rate + h.y*5.0);
    float b = pow(max(sin(3.1416*life), 0.0), 6.0);
    float xl = exp(-pow(f.x/(size*0.08), 2.0) - pow(f.y/size, 2.0)) + exp(-pow(f.y/(size*0.08), 2.0) - pow(f.x/size, 2.0));
    float heart = exp(-dot(f, f)/(size*size*0.04));
    acc += (hot*(xl*0.8 + heart*1.5) + col*exp(-dot(f, f)/(size*size*0.5))*0.35)*b*w;
  }
  return acc;
}
// Smoke for a solid layer (sig_vol_solid): soft dark wisps rising on two shells round the blade,
// thin where they thin; alpha up to `thick`. Never on the blade's body itself.
vec4 shSmokeA(vec3 ro, vec3 rd, float R, float y0, float y1, float k, float rise, float seed, vec3 col, float thick){
  vec4 acc = vec4(0.0);
  for (int s = 0; s < 2; s++){
    float Rs = R*(s == 0 ? 0.7 : 1.0);
    vec2 t = shT(ro, rd, Rs);
    for (int i = 0; i < 2; i++){
      vec2 q;
      float w = shAt(ro, rd, Rs, i == 0 ? t.x : t.y, y0, y1, 3.0, q);
      if (w <= 0.001) continue;
      q.x += seed + float(s)*0.37;
      float n = shNoise(q, k, 0.1, rise*0.1)*0.6 + shNoise(q + vec2(0.2, 9.0), k*2.2, 0.22, rise*0.22)*0.4;
      // (update 21, Creyon's pictures: a dark block) the wall thins well before its limb
      float a = smoothstep(0.45, 0.85, n)*w*w*thick;
      acc.rgb += col*a*(1.0 - acc.a);
      acc.a += a*(1.0 - acc.a);
    }
  }
  return acc;
}
// Pulses: soft swells of light running up the blade's glow (in its picture), `n` on it at a time,
// `v` units a second; long soft swells, never bands.
float shPulse(float y, float n, float v, float w){
  float L = max(uLength, 1.0);
  float p = fract((y - uTime*v)/L*n);
  return exp(-pow((p - 0.5)*L/n/w, 2.0));
}
// How much of an effect round the blade shows near the hilt (1/11/2026, Creyon's update-12 test:
// a hard line showed where effects started at the emitter): fading in over the first units of the
// blade, read where the view ray passes the axis; looking along the blade, all of it.
// A thin box round c along ax, up and az (half sizes in `size`): its cover, soft at its outline, the
// point hit on it in its own axes and how far along the ray.
float xSlab(vec3 ro, vec3 rd, vec3 c, vec3 ax, vec3 up, vec3 az, vec3 size, float px, out vec3 hit, out float tn){
  vec3 rel = ro - c;
  vec3 lo = vec3(dot(rel, ax), dot(rel, up), dot(rel, az));
  vec3 ld = vec3(dot(rd, ax), dot(rd, up), dot(rd, az));
  vec3 m = 1.0/(ld + vec3(1e-5));
  vec3 n = m*lo;
  vec3 kk = abs(m)*size;
  vec3 t1 = -n - kk, t2 = -n + kk;
  tn = max(max(t1.x, t1.y), t1.z);
  float tf = min(min(t2.x, t2.y), t2.z);
  hit = lo + ld*tn;
  float soft = max(px*1.6, 0.05);
  return 1.0 - smoothstep(-soft, soft, tn - tf);
}
// A crooked line's offset at y, -0.5 to 0.5, straight between kinks, the kinks moving smoothly with t: a
// bolt that writhes without the steps a new random path every few frames shows (update 22).
float xZigT(float y, float t, float seed){
  float i = floor(y), f = fract(y);
  float n = floor(t), u = fract(t);
  u = u*u*(3.0 - 2.0*u);
  float a = mix(h11(i*1.37 + seed + n*7.1), h11(i*1.37 + seed + (n + 1.0)*7.1), u);
  float b = mix(h11((i + 1.0)*1.37 + seed + n*7.1), h11((i + 1.0)*1.37 + seed + (n + 1.0)*7.1), u);
  return mix(a, b, f) - 0.5;
}
// Update 22 (Creyon: nearly every blade had the same curved ribbons): the same long path round a blade
// as rxRibbon's, drawn in other ways: beads of light travelling it, fine dust, short comets, soft puffs.
vec3 xPathAt(float r0, float r1, float turns, float phase, float y0, float y1, float f){
  return around(mix(r0, r1, f), phase + turns*f, mix(y0, y1, f));
}
vec3 xBeads(vec3 ro, vec3 rd, float r0, float r1, float turns, float phase, float y0, float y1, float w, float k, vec3 hot, vec3 cool){
  vec3 acc = vec3(0.0);
  float t;
  for (int i = 0; i < 6; i++){
    float m = float(i);
    float f = fract(m/6.0 + uClock*0.1 + h11(k));
    vec3 p = xPathAt(r0, r1, turns, phase, y0, y1, f);
    float d = rayPt(ro, rd, p, t);
    float sz = w*(0.22 + 0.12*sin(uClock*2.0 + m*2.0));
    acc += (hot*halo(d, sz)*1.3 + cool*halo(d, sz*3.0)*0.35)*sin(3.1416*f)*roomAt(p);
  }
  return acc;
}
vec3 xDust(vec3 ro, vec3 rd, float r0, float r1, float turns, float phase, float y0, float y1, float w, float k, vec3 hot, vec3 cool){
  vec3 acc = vec3(0.0);
  float t;
  for (int i = 0; i < 12; i++){
    float m = float(i);
    float f = fract(m/12.0 + uClock*0.05 + h11(k*1.3));
    vec3 p = xPathAt(r0 + w*(h11(m + k) - 0.5)*2.5, r1 + w*(h11(m*2.0 + k) - 0.5)*2.5, turns, phase, y0, y1, f) + vec3(0.0, w*(h11(m*3.0 + k) - 0.5)*2.5, 0.0);
    float d = rayPt(ro, rd, p, t);
    float tw = 0.5 + 0.5*sin(uClock*(1.5 + 2.0*h11(m*5.0)) + m*2.3);
    acc += (hot*halo(d, 0.05)*1.3*tw + cool*halo(d, 0.2)*0.2)*sin(3.1416*f)*roomAt(p);
  }
  return acc;
}
vec3 xComets(vec3 ro, vec3 rd, float r0, float r1, float turns, float phase, float y0, float y1, float w, float k, vec3 hot, vec3 cool){
  vec3 acc = vec3(0.0);
  float s;
  for (int i = 0; i < 3; i++){
    float m = float(i);
    float f = fract(m/3.0 + uClock*0.14 + h11(k*2.1));
    vec3 a = xPathAt(r0, r1, turns, phase, y0, y1, max(f - 0.09, 0.0));
    vec3 b = xPathAt(r0, r1, turns, phase, y0, y1, f);
    float d = raySeg(ro, rd, a, b, s);
    acc += (hot*halo(d, 0.03 + 0.05*s)*1.4*s + cool*halo(d, 0.18)*0.35*s)*sin(3.1416*f)*roomAt(b);
  }
  return acc;
}
vec3 xPuffs(vec3 ro, vec3 rd, float r0, float r1, float turns, float phase, float y0, float y1, float w, float k, vec3 hot, vec3 cool){
  vec3 acc = vec3(0.0);
  float t;
  for (int i = 0; i < 5; i++){
    float m = float(i);
    float f = fract(m/5.0 + uClock*0.07 + h11(k*3.1));
    vec3 p = xPathAt(r0, r1, turns, phase, y0, y1, f);
    float d = rayPt(ro, rd, p, t);
    float sz = w*(0.5 + 1.8*f);
    acc += (cool*halo(d, sz)*0.42 + hot*halo(d, sz*0.35)*0.2)*sin(3.1416*f)*roomAt(p);
  }
  return acc;
}
float hiltFade(vec3 ro, vec3 rd){
  float a = dot(rd.xz, rd.xz);
  float ta = -dot(ro.xz, rd.xz)/max(a, 1e-5);
  float y = ro.y + rd.y*ta;
  return mix(1.0, smoothstep(0.2, 3.2, y), smoothstep(0.4, 0.85, a));
}

// ---- update 14 (1/11/2026): solid pieces for the Mythicals' own parts ----
// Creyon: every Mythical needs things of its own round it, like Obsidian Edge's shards and Abyssal
// Eye's eye. These are the shapes a few of them are built from.
// A ball of radius R at c, squashed to `squash` of its height: how much of the pixel it covers
// (soft over a pixel, hidden behind the blade's body) and, in n, its normal where the ray meets it.
float xBall(vec3 ro, vec3 rd, vec3 c, float R, float squash, float px, out vec3 n){
  vec3 sc = vec3(1.0, 1.0/max(squash, 0.05), 1.0);
  vec3 o2 = c + (ro - c)*sc;
  vec3 d2 = normalize(rd*sc);
  float t;
  float d = rayPtRaw(o2, d2, c, t);
  n = vec3(0.0, 1.0, 0.0);
  if (d > R + px) return 0.0;
  float back = sqrt(max(R*R - d*d, 0.0));
  n = normalize((o2 + d2*(t - back) - c)*sc);
  float tr = dot(c - ro, rd);
  return clamp((R - d)/max(px, 1e-3) + 0.5, 0.0, 1.0)*(1.0 - behindBlade(ro, rd, tr));
}
// A six-sided crystal at c, pointed at both ends, sz long, turned by spin: its cover, and in sh
// its shading (x which facet faces the eye, 0 to 1; y its glowing edges and ridges).
float xGem(vec3 ro, vec3 rd, vec3 c, float sz, float spin, float k, float px, out vec2 sh){
  float th;
  float d = rayPt(ro, rd, c, th);
  sh = vec2(0.0);
  if (d > sz*0.75 + px) return 0.0;
  vec3 o = ro + rd*th - c;
  vec2 a = rot(spin + k*6.0)*o.xz;
  o = vec3(a.x, o.y, a.y);
  vec2 b = rot(0.5*sin(spin*0.6 + k))*o.xy;
  o = vec3(b, o.z);
  float ang = atan(o.z, o.x);
  float sec = floor(ang/1.0472 + 0.5);
  float ca = ang - sec*1.0472;
  float rr = length(o.xz)*cos(ca);
  float e = rr + abs(o.y)*0.42 - sz*0.2;
  float ridge = halo(abs(ca)*length(o.xz), 0.015 + px*0.5);
  sh = vec2(0.5 + 0.5*h11(sec + k*7.0 + sign(o.y)*3.0), halo(abs(e), 0.025 + px*0.8) + 0.5*ridge);
  return clamp(0.5 - e/max(px, 1e-3), 0.0, 1.0);
}

// ---- update 15 (1/11/2026): swarms round the whole blade ----
// Creyon: "the effects must be over the whole blade and move round it". A swarm: `n` (up to 32)
// small things on their own spiral paths round the blade from the emitter to near its point,
// `R0` to `R1` units out, climbing `rise` units a second (falling when negative) and turning `spin`
// turns a second, `size` across, each fading in and out over its climb. `kind` is what they are:
// 0 motes, 1 sparks (streaks along their path), 2 petals or leaves (soft ovals turning over),
// 3 stars (four-pointed twinkles), 4 flakes (six-pointed), 5 drops (a bright head, a short tail),
// 6 bubbles (a soft hollow with a glint), 7 wisps (long soft blobs along their path). `c1` lights
// their hearts, `c2` their glow. Each is read in the plane facing the camera, cheap per pixel.
vec3 xSwarm(vec3 ro, vec3 rd, float px, float kind, float n, float R0, float R1, float rise, float spin, float size, float k, vec3 c1, vec3 c2){
  vec3 acc = vec3(0.0);
  float L = uLength;
  if (L < 2.0 || fxNear(ro, rd, 0.0, L) > R1 + 1.5) return acc;
  vec3 u = normalize(cross(rd, abs(rd.y) < 0.95 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
  vec3 v = cross(rd, u);
  float span = L*0.9;
  float reach = size*(kind > 0.5 && kind < 1.5 ? 9.0 : 6.0) + 3.0*px;
  float t;
  for (int i = 0; i < 32; i++){
    float m = float(i);
    if (m >= n) break;
    float h1 = h11(m*3.7 + k), h2 = h11(m*5.3 + k*1.3), h3 = h11(m*7.1 + k*0.7);
    float sp = abs(rise)*(0.75 + 0.5*h3);
    float life = fract(uTime*sp/span + h1);
    float y = L*0.04 + span*(rise >= 0.0 ? life : 1.0 - life);
    float a = h2 + uTime*spin*(0.7 + 0.6*h3) + 0.03*sin(uTime*0.5 + m);
    float r = mix(R0, R1, h3) + 0.25*sin(uTime*0.7 + m*2.0);
    vec3 p = around(r, a, y);
    if (rayPtRaw(ro, rd, p, t) > reach) continue;
    float fade = smoothstep(0.0, 0.1, life)*(1.0 - smoothstep(0.82, 1.0, life))*roomAt(p)*(1.0 - behindBlade(ro, rd, t));
    vec3 q = ro + rd*t - p;
    vec2 e = vec2(dot(q, u), dot(q, v));
    vec3 vel = around(6.2832*r*spin, a + 0.25, (rise >= 0.0 ? 1.0 : -1.0)*sp);
    vec2 vd = vec2(dot(vel, u), dot(vel, v));
    vd = vd/max(length(vd), 1e-4);
    float along = dot(e, vd), across = dot(e, vec2(-vd.y, vd.x));
    float tw = 0.6 + 0.4*sin(uTime*(2.0 + 3.0*h2) + m*4.0);
    vec3 c = vec3(0.0);
    if (kind < 0.5){
      float d = length(e);
      c = c1*halo(d, max(size, px))*1.8 + c2*halo(d, size*3.0)*0.45;
      c *= tw;
    } else if (kind < 1.5){
      float len = size*8.0;
      float d = length(vec2(across, along - clamp(along, -len, 0.0)));
      float tail = 1.0 + clamp(along, -len, 0.0)/len;
      c = (c1*halo(d, max(size*0.5, px))*1.8 + c2*halo(d, size*2.0)*0.4)*(0.3 + 0.7*tail);
    } else if (kind < 2.5){
      vec2 f2 = rot(uTime*(1.0 + h1) + m)*e;
      float flip = 0.35 + 0.65*abs(sin(uTime*(1.2 + h2) + m));
      float ell = length(f2/vec2(size*2.0, size*0.8*flip));
      float body = 1.0 - smoothstep(0.55, 1.0, ell);
      c = mix(c2, c1, body*body)*body*1.3 + c2*halo(length(e), size*2.5)*0.25;
    } else if (kind < 3.5){
      float d = length(e);
      float rays = exp(-abs(e.x)/max(size*0.25, px))*exp(-abs(e.y)/(size*3.5)) + exp(-abs(e.y)/max(size*0.25, px))*exp(-abs(e.x)/(size*3.5));
      c = (c1*(halo(d, max(size*0.6, px))*1.8 + rays*1.1) + c2*halo(d, size*3.0)*0.35)*tw*tw;
    } else if (kind < 4.5){
      float d = length(e);
      float aa = mod(atan(e.y, e.x) + m, 1.0472) - 0.5236;
      float spoke = exp(-pow(d*sin(aa)/max(size*0.18, px*0.7), 2.0))*(1.0 - smoothstep(size*1.2, size*2.2, d));
      c = c1*(spoke*1.2 + halo(d, max(size*0.4, px))*1.2) + c2*halo(d, size*2.5)*0.25;
    } else if (kind < 5.5){
      float len = size*4.0;
      float d = length(vec2(across, along - clamp(along, -len, 0.0)));
      float w = mix(size*0.15, size*0.8, 1.0 + clamp(along, -len, 0.0)/len);
      c = c1*halo(d, max(w, px))*1.6 + c2*halo(d, size*2.0)*0.35;
    } else if (kind < 6.5){
      float d = length(e);
      float shell = halo(abs(d - size*1.6), max(size*0.22, px));
      float glint = halo(length(e - vec2(-0.6, 0.6)*size), max(size*0.25, px));
      c = c1*(shell*0.9 + glint*1.2) + c2*(1.0 - smoothstep(size*1.2, size*1.7, d))*0.12;
    } else {
      float ell = length(vec2(along/(size*5.0), across/(size*1.2)));
      c = (c1*exp(-ell*ell*3.0)*0.9 + c2*exp(-ell*ell)*0.45)*(0.75 + 0.25*tw);
    }
    acc += c*fade;
  }
  return acc;
}
// Update 30 (Creyon: the lightning lagged, like low fps): lightning alive every frame. A bolt whose
// shape writhes continuously and which fades in and out smoothly, instead of a shape held still and
// swapped for another. liveArcs takes the same arguments as fxArcs.
float liveBolt(vec3 ro, vec3 rd, vec3 a, vec3 m, vec3 b, float amp, float seed, float px){
  vec3 ax = b - a;
  float sq;
  if (raySegRaw(ro, rd, a, b, sq) > length(m - 0.5*(a + b)) + amp + 2.6) return 1e3;
  float n0 = 18.0;
  vec3 u = normalize(cross(ax, vec3(0.31, 0.17, 0.93)));
  vec3 v = normalize(cross(ax, u));
  float best = 1e3, s, T = floor(uClock*22.0)*1.37;
  vec3 prev = a;
  for (int j = 1; j <= 18; j++){
    float g = float(j)/n0;
    if (g > 1.001) break;
    float n1 = sin(g*23.0 + seed + T) + 0.6*sin(g*51.0 - seed*1.7 + T*1.9) + 0.35*sin(g*97.0 + seed*2.3 - T*3.1);
    float n2 = sin(g*29.0 + seed*3.1 - T*1.3) + 0.6*sin(g*61.0 + seed*0.7 + T*2.2);
    vec3 p = mix(mix(a, m, g), mix(m, b, g), g) + (u*n1 + v*n2)*amp*sin(3.1416*g)*0.5;
    best = min(best, raySegRaw(ro, rd, prev, p, s));
    prev = p;
  }
  return best;
}
vec3 liveArcs(vec3 ro, vec3 rd, float px, float y0, float y1, float count, float reach, float rate, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > min(reach, 0.6*uReach)*1.6 + 3.0) return acc;
  float t;
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    if (fi >= count) break;
    // Storm's timing for every bolt, whatever else differs: `rate` is kept for the callers' sake
    float cyc = uClock*6.5 + fi*0.618 + h11(fi*3.3 + seed)*5.0;
    float k = floor(cyc), q = fract(cyc);
    if (h11(k*1.3 + fi*5.1 + seed) < 0.3) continue;
    float env = pow(1.0 - q, 1.5);
    float crawl = 2.77*q;
    float ya = mix(y0, y1, h11(k*1.9 + fi*2.7 + seed)) + crawl;
    float yb = min(ya + 3.0 + 6.0*h11(k*2.3 + fi + seed), y1);
    if (yb <= ya + 0.5) continue;
    float a = h11(k*4.1 + fi*2.0 + seed) + 0.02*sin(uClock*0.9 + fi);
    float R = min(reach, 0.6*uReach)*(0.45 + 0.55*h11(k*7.7 + fi + seed));
    vec3 mid = around(R*1.6, a, 0.5*(ya + yb));
    if (rayPtRaw(ro, rd, mid, t) > R*1.4 + (yb - ya)*0.65 + 2.8) continue;
    float d = liveBolt(ro, rd, around(0.25, a, ya), mid, around(0.25, a, yb), 0.42, k + fi*1.37 + seed, px);
    float fade = 1.0 - smoothstep(1.2, 2.4, d);
    acc += col*(halo(d, max(0.035, px))*1.5 + halo(d, 0.16)*0.65 + halo(d, 0.7)*0.12)*env*fade;
    acc += mix(col, vec3(1.0), 0.75)*halo(d, max(0.018, px*0.7))*0.9*env;
  }
  return acc;
}
// Update 30 (Creyon: effects fade toward the tip or above it, never in a straight line): how much of an effect shows
// by where the view ray passes: a soft dome round the tip, lower further from the blade, its edge never straight.
float tipFade(vec3 ro, vec3 rd){
  float H = uLength + 4.0, s;
  float dr = raySeg(ro, rd, vec3(0.0), vec3(0.0, H, 0.0), s);
  float top = uLength + 1.1 - 0.28*dr*dr + 0.35*sin(dr*2.1 + uClock*0.7);
  // Only from the side does the ray's nearest point on the blade say where along it the ray is. Looking along the blade, from behind
  // a player's shoulder, that point runs far past the tip for everything high up: every effect above the hand was faded out.
  return mix(1.0, 1.0 - smoothstep(top - 3.2, top, s*H), smoothstep(0.4, 0.85, dot(rd.xz, rd.xz)));
}
// The same in the blade's picture, for what a blade draws in its glow.
float tipFadeB(vec2 b){
  return 1.0 - smoothstep(uLength - 1.0, uLength + 1.9, b.y + 0.22*b.x*b.x);
}

// ---- Update 34: signature pieces for the round's blades: big, readable things round a blade, its own for each ----
// Tall flame up both edges of a blade `half` wide, in its picture.
vec3 sxFlames(vec2 b, float half, float tall, float speed, vec3 hot, vec3 deep){
  float L = uLength, side = b.x > 0.0 ? 2.0 : 8.0, d = max(abs(b.x) - half, 0.0);
  if (d > tall*1.6) return vec3(0.0);
  float f1 = rxFlow(vec2(b.y*0.3 - uClock*speed, side));
  float f2 = rxFlow(vec2(b.y*0.72 - uClock*speed*1.45, side + 3.0));
  float along = smoothstep(0.5, 4.0, b.y)*(1.0 - smoothstep(L - 3.5, L - 0.5, b.y));
  if (along <= 0.0) return vec3(0.0);
  float reach = 0.18 + tall*(f1*f1 + 0.35*f2*f2);
  float q = clamp(d/max(reach, 0.05), 0.0, 1.0);
  float fl = (1.0 - q)*(1.0 - q);
  return mix(deep, hot, fl)*fl*along;
}
// Great soft lights circling the blade, each with a tail along its path.
vec3 sxOrbs(vec3 ro, vec3 rd, float px, float n, float r, float size, float speed, float y0, float y1, float seed, vec3 core, vec3 aura){
  vec3 acc = vec3(0.0);
  float t;
  for (int i = 0; i < 8; i++){
    float k = float(i);
    if (k >= n) break;
    float a = k/n + uClock*speed*(0.8 + 0.4*h11(k + seed));
    float y = mix(y0, y1, 0.5 + 0.5*sin(uClock*speed*5.0 + k*2.4 + seed));
    float rr = r*(0.85 + 0.3*h11(k*3.1 + seed));
    float d0 = rayPt(ro, rd, around(rr, a, y), t);
    if (d0 > size*6.0 + 3.0*abs(speed)*20.0 + 2.0) continue;
    acc += core*halo(d0, max(size*0.42, px))*1.5 + aura*(halo(d0, size)*0.6 + halo(d0, size*2.6)*0.13);
    for (int j = 1; j <= 6; j++){
      float g = float(j);
      float d1 = rayPt(ro, rd, around(rr, a - g*0.016*sign(speed), y), t);
      acc += aura*halo(d1, size*(0.62 - 0.07*g))*0.3*(1.0 - g/7.0);
    }
  }
  return acc;
}
// Broken slabs circling the blade, solid, lit along their edges.
vec4 sxShards(vec3 ro, vec3 rd, float px, float n, float r, float size, float speed, float seed, vec3 dark, vec3 edge){
  vec4 o = vec4(0.0);
  float best = 1e9, t;
  for (int i = 0; i < 8; i++){
    float k = float(i);
    if (k >= n) break;
    vec3 c = around(r*(0.8 + 0.5*h11(k*5.1 + seed)), h11(k*1.7 + seed) + uClock*speed*(0.7 + 0.6*h11(k*2.9 + seed)), uLength*(0.12 + 0.78*fract(k*0.618 + seed*0.13)) + 0.6*sin(uClock*0.22 + k*2.0));
    rayPt(ro, rd, c, t);
    if (t > best) continue;
    vec2 sh;
    float cov = xGem(ro, rd, c, size*(0.8 + 0.5*h11(k*9.1 + seed)), uClock*(0.1 + 0.15*h11(k)), k + seed, px, sh);
    if (cov <= 0.0) continue;
    best = t;
    o = vec4(mix(dark, dark*3.0 + edge*0.1, sh.x) + edge*sh.y*1.2, cov*(1.0 - behindBlade(ro, rd, t)));
  }
  return o;
}
// Small worlds circling the blade, round and lit from one side, burning in their cracks.
vec4 sxGlobes(vec3 ro, vec3 rd, float px, float n, float r, float size, float speed, float seed, vec3 lit, vec3 dark, vec3 burn){
  vec4 o = vec4(0.0);
  float best = 1e9, t;
  for (int i = 0; i < 6; i++){
    float k = float(i);
    if (k >= n) break;
    vec3 c = around(r*(0.8 + 0.5*h11(k*5.1 + seed)), h11(k*1.7 + seed) + uClock*speed*(0.7 + 0.6*h11(k*2.9 + seed)), uLength*(0.14 + 0.72*fract(k*0.618 + seed*0.13)) + 0.5*sin(uClock*0.2 + k*2.0));
    float R = size*(0.7 + 0.6*h11(k*7.7 + seed));
    if (rayPtRaw(ro, rd, c, t) > R + 0.3 || t > best) continue;
    vec3 nr;
    float cov = xBall(ro, rd, c, R, 1.0, px, nr);
    if (cov <= 0.0) continue;
    best = t;
    float day = max(dot(nr, normalize(vec3(-0.5, 0.6, 0.6))), 0.0);
    float cr = rxFlow(nr.xy*2.4 + nr.z*1.7 + k*3.0 + vec2(0.0, uClock*0.05));
    vec3 col = mix(dark, lit, day) + burn*(1.0 - smoothstep(0.0, 0.07, abs(cr - 0.5)))*(0.5 + 0.5*sin(uClock*0.9 + k));
    o = vec4(col, cov*(1.0 - behindBlade(ro, rd, t)));
  }
  return o;
}
// Holes of nothing: soft-edged black discs in the air.
vec4 sxVoids(vec3 ro, vec3 rd, float px, float n, float r, float size, float speed, float seed){
  vec4 o = vec4(0.0);
  float t;
  for (int i = 0; i < 5; i++){
    float k = float(i);
    if (k >= n) break;
    vec3 c = around(r*(0.85 + 0.3*h11(k*5.1 + seed)), h11(k*1.7 + seed) + uClock*speed, uLength*(0.18 + 0.66*fract(k*0.618 + seed*0.13)) + 0.5*sin(uClock*0.17 + k*2.0));
    float R = size*(0.75 + 0.5*h11(k*7.7 + seed))*(0.85 + 0.15*sin(uClock*0.7 + k*2.0));
    float d = rayPt(ro, rd, c, t);
    float a = (1.0 - smoothstep(R - 0.08 - px, R + 0.04 + px, d))*(1.0 - behindBlade(ro, rd, t));
    if (a > o.a) o = vec4(vec3(0.0), a);
  }
  return o;
}
// The light round those holes: same places as sxVoids.
vec3 sxVoidGlow(vec3 ro, vec3 rd, float n, float r, float size, float speed, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  float t;
  for (int i = 0; i < 5; i++){
    float k = float(i);
    if (k >= n) break;
    vec3 c = around(r*(0.85 + 0.3*h11(k*5.1 + seed)), h11(k*1.7 + seed) + uClock*speed, uLength*(0.18 + 0.66*fract(k*0.618 + seed*0.13)) + 0.5*sin(uClock*0.17 + k*2.0));
    float R = size*(0.75 + 0.5*h11(k*7.7 + seed))*(0.85 + 0.15*sin(uClock*0.7 + k*2.0));
    float d = max(rayPt(ro, rd, c, t) - R, 0.0);
    acc += col*(halo(d, 0.16)*1.1 + halo(d, 0.7)*0.45 + halo(d, 2.0)*0.1);
  }
  return acc;
}
// A great bolt from out in the air to the blade, thick and bright, every so often; a flash where it lands.
vec3 sxStrike(vec3 ro, vec3 rd, float px, float n, float rate, float reach, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  float t;
  for (int i = 0; i < 4; i++){
    float k = float(i);
    if (k >= n) break;
    float f = rate*(0.8 + 0.4*h11(k + seed));
    float cy = uClock*f + h11(k*3.3 + seed)*7.0;
    float m = floor(cy), q = fract(cy);
    // rare, but each strike lasts two of Storm's beats
    float env = pow(max(1.0 - q/f*3.25, 0.0), 1.5);
    if (env < 0.01) continue;
    float a = h11(m*4.1 + k + seed), y = uLength*(0.15 + 0.75*h11(m*1.9 + k + seed));
    vec3 far_ = around(reach, a, min(y + 3.0 + 3.0*h11(m*2.3 + k), uLength - 0.5));
    vec3 hit = around(0.7, a, y);
    float d = liveBolt(ro, rd, far_, mix(far_, hit, 0.5) + vec3(0.0, 0.6, 0.0), hit, 0.55, m + k*3.1 + seed, px);
    acc += (mix(col, vec3(1.0), 0.7)*halo(d, max(0.05, px))*1.9 + col*(halo(d, 0.26)*0.9 + halo(d, 1.0)*0.18))*env*(1.0 - smoothstep(2.0, 3.6, d));
    float dh = rayPt(ro, rd, hit, t);
    acc += col*(halo(dh, 0.9)*0.9 + halo(dh, 2.4)*0.2)*env;
  }
  return acc;
}
// Heavy drops falling past the blade, each with a long fine tail.
vec3 sxDrops(vec3 ro, vec3 rd, float px, float n, float r, float speed, float seed, vec3 core, vec3 aura){
  vec3 acc = vec3(0.0);
  float t, s;
  for (int i = 0; i < 12; i++){
    float k = float(i);
    if (k >= n) break;
    float q = fract(uClock*speed*(0.7 + 0.6*h11(k*2.3 + seed)) + h11(k*6.7 + seed));
    vec3 p = around(r*(0.7 + 0.5*h11(k*3.7 + seed)), h11(k*5.1 + seed), uLength*(0.95 - 0.95*q*q));
    float fade = smoothstep(0.0, 0.1, q)*(1.0 - smoothstep(0.8, 1.0, q));
    float d = rayPt(ro, rd, p, t);
    float dt = raySeg(ro, rd, p, p + vec3(0.0, 1.4 + 2.4*q, 0.0), s);
    acc += (core*halo(d, max(0.09, px))*1.5 + aura*halo(d, 0.32)*0.45 + aura*halo(dt, max(0.05, px))*0.6*(1.0 - s))*fade;
  }
  return acc;
}
// ---- end of update 34's pieces ----

// ---- Update 35: pieces that are not orbs. Each is meant for only a few blades. ----
// Light drawn in on a spiral: a short streak starts far out, winds in and is gone before the blade.
vec3 syInflow(vec3 ro, vec3 rd, float px, float n, float R, float y0, float y1, float rate, float seed, vec3 hot, vec3 deep){
  vec3 acc = vec3(0.0);
  float s;
  for (int i = 0; i < 16; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uClock*rate + k/n + h11(k*1.7 + seed);
    float ph = fract(run), cyc = floor(run);
    float y = mix(y0, y1, h11(k*3.3 + cyc + seed));
    float a = h11(k*5.1 + cyc*1.3 + seed);
    float p0 = max(ph - 0.09, 0.0);
    vec3 head = around(mix(R, 1.05, ph*ph), a + 0.4*ph, y), tail = around(mix(R, 1.05, p0*p0), a + 0.4*p0, y);
    float d = raySeg(ro, rd, tail, head, s);
    float vis = smoothstep(0.0, 0.3, ph)*(1.0 - smoothstep(0.8, 0.98, ph))*s;
    acc += (hot*halo(d, max(0.05, px))*1.1 + deep*halo(d, 0.3)*0.35)*vis;
  }
  return acc;
}
// Loops of light leaving the blade's side, standing out from it and landing further up, one at a time.
vec3 syArcs(vec3 ro, vec3 rd, float px, float n, float tall, float span, float rate, float seed, vec3 hot, vec3 deep){
  vec3 acc = vec3(0.0);
  float L = uLength, s, tm;
  for (int i = 0; i < 6; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uClock*rate + k/n;
    float ph = fract(run), cyc = floor(run);
    float y = mix(L*0.08, max(L*0.92 - span, L*0.1), h11(k*2.3 + cyc*0.7 + seed));
    float a = h11(k*4.1 + cyc*1.9 + seed);
    float hgt = tall*(0.75 + 0.25*h11(k*6.7 + cyc + seed));
    if (rayPtRaw(ro, rd, around(1.0 + hgt*0.5, a, y + span*0.5), tm) > hgt + span*0.6 + 1.5) continue;
    vec3 prev = around(0.8, a, y);
    float best = 1e3;
    for (int j = 1; j <= 14; j++){
      float f = float(j)/14.0;
      vec3 q = around(0.8 + hgt*sin(3.1416*f), a + 0.05*sin(6.2832*f), y + span*f);
      best = min(best, raySeg(ro, rd, prev, q, s));
      prev = q;
    }
    float vis = sin(3.1416*ph);
    acc += (hot*halo(best, max(0.045, px))*0.9 + deep*halo(best, 0.34)*0.4)*vis*vis;
  }
  return acc;
}
// Thin threads of smoke curling up beside the blade; they thin out toward both ends.
vec3 sySmoke(vec2 b, float R, float rise, float k, vec3 col){
  float L = uLength, d = abs(b.x);
  if (d > R) return vec3(0.0);
  float along = smoothstep(0.5, 4.0, b.y)*(1.0 - smoothstep(L - 4.0, L - 0.5, b.y));
  if (along <= 0.0) return vec3(0.0);
  float bend = rxFlow(vec2(b.y*0.11 - uClock*rise*0.5, k));
  float v = rxFlow(vec2(b.x*0.5 + 1.6*bend, b.y*0.06 - uClock*rise + k));
  float thread = pow(1.0 - abs(2.0*v - 1.0), 7.0);
  float out_ = 1.0 - d/R;
  return col*thread*out_*out_*along*smoothstep(0.55, 1.1, d);
}
// Effects rise with the blade as it lights and sink with it as it goes out, never ahead of it.
float growMask(vec3 ro, vec3 rd){
  float sa;
  raySegRaw(ro, rd, vec3(0.0), vec3(0.0, uLength + 4.0, 0.0), sa);
  float grown = (uLength + 6.0)*rxLife(0.0, 0.55, 0.32);
  return mix(1.0 - smoothstep(0.0, 1.0, 1.0 - rxLife(0.0, 0.55, 0.32)), 1.0 - smoothstep(grown - 4.0, grown, sa*(uLength + 4.0)), smoothstep(0.4, 0.85, dot(rd.xz, rd.xz)));
}
// ---- the round's mist at half the steps (3/11/2026, Sol's rule: looks the same for less): it is smooth enough that five samples read as ten ----
vec3 syMistSpinP(vec3 ro, vec3 rd, float y0, float y1, float R0, float R1, float phase, float k, vec3 a, vec3 b){
  float R = max(R0, R1);
  float qa = dot(rd.xz, rd.xz);
  if (qa < 1e-5) return vec3(0.0);
  float qb = dot(ro.xz, rd.xz);
  float qc = dot(ro.xz, ro.xz) - R*R;
  float disc = qb*qb - qa*qc;
  if (disc <= 0.0) return vec3(0.0);
  float sq = sqrt(disc);
  float t0 = max((-qb - sq)/qa, 0.0), t1 = (-qb + sq)/qa;
  // Only the stretch inside its height: a ray running along the blade crosses the whole shell, and
  // eleven steps over that much air drew its end as a hard, stepped line (Creyon's God Ki, Scipio).
  if (abs(rd.y) > 1e-4){
    float ya = (y0 - ro.y)/rd.y, yb = (y1 - ro.y)/rd.y;
    t0 = max(t0, min(ya, yb)); t1 = min(t1, max(ya, yb));
  }
  if (t1 <= t0) return vec3(0.0);
  float n = 11.0, jit = 0.5;
  float dt = (t1 - t0)/n;
  vec3 acc = vec3(0.0);
  for (int i = 0; i < int(n); i++){
    vec3 p = ro + rd*(t0 + dt*(float(i) + jit));
    float h = (p.y - y0)/max(y1 - y0, 0.01);
    if (h <= 0.0 || h >= 1.0) continue;
    float r = length(p.xz);
    float Rh = mix(R0, R1, h);
    float ang = atan(p.x, p.z)/6.2832 - phase*(1.2 - 0.4*h);
    if (r < 0.05*Rh) continue;
    float n2 = rxFlow(vec2(cos(6.2832*ang)*1.6 - r*0.3 - k, h*4.0 + k));
    if (r >= (0.45 + 0.55*n2)*Rh) continue;
    float n1 = rxFlow(vec2(sin(6.2832*ang)*1.6 + r*0.4 + k, h*3.0 - uTime*0.1));
    float swirl = 0.5 + 0.5*sin(6.2832*(ang*2.0 + r*0.25));
    float dens = smoothstep(0.3, 0.9, n1*0.55 + n2*0.4 + swirl*0.3)*smoothstep(0.05*Rh, 0.35*Rh, r)*(1.0 - smoothstep(0.3*Rh, (0.45 + 0.55*n2)*Rh, r))
               *smoothstep(0.0, 0.15, h)*(1.0 - smoothstep(0.75, 1.0, h));
    acc += mix(b, a, smoothstep(0.2, 0.8, dens))*dens*dt;
  }
  return acc*0.4*roomFade(R*0.6);
}
vec3 syMistSpin(vec3 ro, vec3 rd, float y0, float y1, float R0, float R1, float spin, float k, vec3 a, vec3 b){
  return syMistSpinP(ro, rd, y0, y1, R0, R1, uTime*spin*0.5, k, a, b);
}
vec3 syMist(vec3 ro, vec3 rd, float y0, float y1, float R, float rise, float k, vec3 a, vec3 b){
  float qa = dot(rd.xz, rd.xz);
  if (qa < 1e-5) return vec3(0.0);
  float qb = dot(ro.xz, rd.xz);
  float qc = dot(ro.xz, ro.xz) - R*R;
  float disc = qb*qb - qa*qc;
  if (disc <= 0.0) return vec3(0.0);
  float sq = sqrt(disc);
  float t0 = (-qb - sq)/qa, t1 = (-qb + sq)/qa;
  t0 = max(t0, 0.0);
  if (abs(rd.y) > 1e-4){
    float ya = (y0 - ro.y)/rd.y, yb = (y1 - ro.y)/rd.y;
    t0 = max(t0, min(ya, yb)); t1 = min(t1, max(ya, yb));
  }
  if (t1 <= t0) return vec3(0.0);
  float n = 11.0;
  float dt = (t1 - t0)/n;
  vec3 acc = vec3(0.0);
  for (int i = 0; i < int(n); i++){
    float t = t0 + dt*(float(i) + 0.5);
    vec3 p = ro + rd*t;
    float r = length(p.xz);
    float span = smoothstep(y0, y0 + 2.5, p.y)*(1.0 - smoothstep(y1 - 3.0, y1, p.y));
    if (span <= 0.0) continue;
    // 1/11/2026 (Creyon's FPS): the outer edge needs only n2, so a sample past it (or at the
    // axis, where it is thin to nothing) skips the other wave
    if (r < 0.05*R) continue;
    float n2 = rxFlow(vec2(p.z*0.3 - p.x*0.24 - k*1.7, p.y*0.22 - uTime*rise*1.3 + k));
    if (r >= (0.45 + 0.55*n2)*R) continue;
    float n1 = rxFlow(vec2(p.x*0.32 + p.z*0.21 + k, p.y*0.16 - uTime*rise));
    float dens = smoothstep(0.35, 0.9, n1*0.6 + n2*0.55)*smoothstep(0.05*R, 0.35*R, r)*(1.0 - smoothstep(0.3*R, (0.45 + 0.55*n2)*R, r))*span;
    acc += mix(b, a, smoothstep(0.2, 0.8, dens))*dens*dt;
  }
  return acc*0.4*roomFade(R*0.6);
}
// Nothing a blade's glow carries (its file's arcs, motes and wisps) hangs in the air above a blade still lighting or going out.
float growGlow(vec2 b){
  float grown = (uLength + 1.0)*rxLife(0.0, 0.7, 0.42);
  return 1.0 - smoothstep(grown - 1.5, grown + 5.5, b.y);
}
// The round's rays (`@raycache fast`, 4/11/2026, Creyon's RTX fell from 125 to 25 frames on blades that cost less than Abyssal
// Eye on llvmpipe, World Shift's pattern): no kept ray, and the body's hiding only where a piece can still show: hiding only
// widens a distance, and past three units the widest of these glows is next to nothing anyway.
float rayPtFast(vec3 ro, vec3 rd, vec3 p, out float t){
  t = max(dot(p - ro, rd), 0.0);
  float d = length(ro + rd*t - p);
  if (d > 3.0) return d;
  return hideD(d, behindBladeFull(ro, rd, t));
}
float raySegFast(vec3 ro, vec3 rd, vec3 a, vec3 b, out float s){
  vec3 ba = b - a, w = ro - a;
  float bd = dot(rd, ba), bb = dot(ba, ba), dw = dot(rd, w), bw = dot(ba, w);
  float den = max(bb - bd*bd, 1e-6);
  float s0 = clamp((bw - bd*dw)/den, 0.0, 1.0);
  float t = max(dot(a + ba*s0 - ro, rd), 0.0);
  vec3 pr = ro + rd*t;
  float s1 = clamp(dot(pr - a, ba)/max(bb, 1e-6), 0.0, 1.0);
  s = s1;
  vec3 q = a + ba*s1;
  float d = length(pr - q);
  if (d > 3.0) return d;
  return hideD(d, behindBladeFull(ro, rd, max(dot(q - ro, rd), 0.0)));
}
// The same three strings of points as xDust, xBeads and xPuffs (`@raycache fast`): a point the ray passes well outside its widest glow pays nothing more.
vec3 xDustFast(vec3 ro, vec3 rd, float r0, float r1, float turns, float phase, float y0, float y1, float w, float k, vec3 hot, vec3 cool){
  vec3 acc = vec3(0.0);
  bool near_ = false;
  float t;
  for (int i = 0; i < 12; i++){
    float m = float(i);
    float f = fract(m/12.0 + uClock*0.05 + h11(k*1.3));
    vec3 p = xPathAt(r0 + w*(h11(m + k) - 0.5)*2.5, r1 + w*(h11(m*2.0 + k) - 0.5)*2.5, turns, phase, y0, y1, f) + vec3(0.0, w*(h11(m*3.0 + k) - 0.5)*2.5, 0.0);
    if (near_ && mod(m, 2.0) > 0.5) continue;
    if (rayPtRaw(ro, rd, p, t) > 0.62) continue;
    float d = rayPt(ro, rd, p, t);
    float tw = 0.5 + 0.5*sin(uClock*(1.5 + 2.0*h11(m*5.0)) + m*2.3);
    acc += (hot*halo(d, 0.05)*1.3*tw + cool*halo(d, 0.2)*0.2)*sin(3.1416*f)*roomAt(p);
  }
  return acc;
}
vec3 xBeadsFast(vec3 ro, vec3 rd, float r0, float r1, float turns, float phase, float y0, float y1, float w, float k, vec3 hot, vec3 cool){
  vec3 acc = vec3(0.0);
  float t;
  for (int i = 0; i < 6; i++){
    float m = float(i);
    float f = fract(m/6.0 + uClock*0.1 + h11(k));
    vec3 p = xPathAt(r0, r1, turns, phase, y0, y1, f);
    float sz = w*(0.22 + 0.12*sin(uClock*2.0 + m*2.0));
    if (rayPtRaw(ro, rd, p, t) > sz*9.0) continue;
    float d = rayPt(ro, rd, p, t);
    acc += (hot*halo(d, sz)*1.3 + cool*halo(d, sz*3.0)*0.35)*sin(3.1416*f)*roomAt(p);
  }
  return acc;
}
vec3 xPuffsFast(vec3 ro, vec3 rd, float r0, float r1, float turns, float phase, float y0, float y1, float w, float k, vec3 hot, vec3 cool){
  vec3 acc = vec3(0.0);
  float t;
  for (int i = 0; i < 5; i++){
    float m = float(i);
    float f = fract(m/5.0 + uClock*0.07 + h11(k*3.1));
    vec3 p = xPathAt(r0, r1, turns, phase, y0, y1, f);
    float sz = w*(0.5 + 1.8*f);
    if (rayPtRaw(ro, rd, p, t) > sz*3.0) continue;
    float d = rayPt(ro, rd, p, t);
    acc += (cool*halo(d, sz)*0.42 + hot*halo(d, sz*0.35)*0.2)*sin(3.1416*f)*roomAt(p);
  }
  return acc;
}
// Comets and sparks of the round's blades never rush (Creyon, 4/11/2026: an effect that moves very fast at a random time should never happen).
vec3 rxCometFast(vec3 ro, vec3 rd, float px, float r, float y0, float climb0, float period0, float k, vec3 head, vec3 tail){
  float climb = climb0*0.75, period = period0*2.6;
  float c = mod(uTime + k*7.3, period);
  float q = c/min(period*0.5, 9.0);
  if (q >= 1.0) return vec3(0.0);
  float life = sin(3.1416*q);
  float a0 = h11(k*4.1 + floor((uTime + k*7.3)/period));
  float best = 1e3, bf = 0.0, s;
  vec3 prev = around(r, a0 + 0.45*(q - 0.12), y0 + climb*(q - 0.12));
  for (int j = 1; j <= 8; j++){
    float f = float(j)/8.0;
    float qq = q - 0.12*(1.0 - f);
    vec3 p = around(r, a0 + 0.45*qq, y0 + climb*qq);
    float d = raySeg(ro, rd, prev, p, s);
    if (d < best){ best = d; bf = (float(j) - 1.0 + s)/8.0; }
    prev = p;
  }
  vec3 hp = around(r, a0 + 0.45*q, y0 + climb*q);
  float t;
  float dh = rayPt(ro, rd, hp, t);
  vec3 col = head*halo(dh, max(0.12, px))*2.0 + head*halo(dh, 0.6)*0.4 + tail*halo(best, 0.05 + 0.25*bf)*pow(bf, 1.5)*0.9;
  return col*life*roomAt(hp);
}
// Spirits: a few lights wandering slowly round the blade on their own ways, each drawing a long soft tail behind it (4/11/2026).
// How much a piece of a curve counts for where along the curve a ray is. The nearest piece alone jumps
// when a curve folds in the view, and everything measured along it jumped with it, as a hard line
// across the air (Creyon's "barrier" edges).
float bW(float d){ float q = d*d; return 1.0/(q*q + 0.0001); }
vec3 syWisps(vec3 ro, vec3 rd, float px, float n, float r0, float r1, float speed, float seed, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float L = uLength;
  for (int i = 0; i < 7; i++){
    float k = float(i);
    if (k >= n) break;
    float sp = speed*(0.7 + 0.6*h11(k*2.3 + seed)), dir = mod(k, 2.0) < 0.5 ? 1.0 : -1.0;
    float a0 = h11(k*3.7 + seed), y0 = L*(0.12 + 0.76*h11(k*5.1 + seed)), rr = mix(r0, r1, h11(k*7.3 + seed));
    float T0 = uClock*sp;
    vec3 head = around(rr + 0.3*sin(T0*0.7 + k), a0 + dir*T0*0.36, y0 + 1.1*sin(T0*0.55 + k*2.0));
    float tm;
    if (rayPtRaw(ro, rd, head, tm) > 3.6) continue;
    float bWs = 0.0;
    float best = 1e3, bf = 0.0, s, bt = 0.0;
    vec3 prev = head;
    for (int j = 1; j <= 12; j++){
      float T1 = T0 - float(j)*0.0833;
      vec3 q = around(rr + 0.3*sin(T1*0.7 + k), a0 + dir*T1*0.36, y0 + 1.1*sin(T1*0.55 + k*2.0));
      float d = raySegRaw(ro, rd, prev, q, s);
      { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(j) - 1.0 + s)/12.0 - bf)*wN/bWs; bt += (dot(prev + (q - prev)*s - ro, rd) - bt)*wN/bWs; }
      prev = q;
    }
    best = hideD(best, behindBlade(ro, rd, max(bt, 0.0)));
    float tail = (1.0 - bf)*(1.0 - bf);
    acc += (hot*halo(best, max(0.02 + 0.05*tail, px))*(0.25 + 0.75*tail) + col*(halo(best, 0.08 + 0.2*tail)*0.55*tail + halo(best, 0.75)*0.1*tail))*(0.75 + 0.25*sin(uClock*1.1 + k*2.0));
    acc += hot*halo(rayPt(ro, rd, head, tm), 0.09)*1.1;
  }
  return acc;
}
// Curtains of light hanging to each side of a blade, folded, slowly moving: drawn in the blade's picture, so they cost next to nothing.
vec3 syAurora(vec2 b, float reach, float seed, vec3 c1, vec3 c2){
  float x = abs(b.x), side = b.x > 0.0 ? seed : seed + 4.0;
  float f = rxFlow(vec2(b.y*0.1 - uClock*0.03 + side, x*0.13));
  float fold = pow(0.5 + 0.5*sin(b.y*0.62 + 6.0*f + uClock*0.12), 2.6);
  float env = smoothstep(0.25, 1.4, x)*(1.0 - smoothstep(reach*0.5, reach, x));
  return mix(c2, c1, fold)*fold*env;
}
// ---- the round's own pieces (4/11/2026, Creyon: no two blades alike, not every blade with long curved lines) ----
// Bubbles going up: a bright skin and a clear middle.
vec3 syBubbles(vec3 ro, vec3 rd, float n, float R, float rise, float seed, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float t, L = uLength;
  for (int i = 0; i < 12; i++){
    float k = float(i);
    if (k >= n) break;
    float q = fract(uClock*rise*(0.7 + 0.6*h11(k + seed)) + h11(k*3.1 + seed));
    vec3 p = around(R*(0.45 + 0.55*h11(k*5.3 + seed)) + 0.2*sin(q*12.0 + k), h11(k*7.7 + seed) + 0.03*sin(q*9.0 + k), L*q);
    float s = 0.09 + 0.16*h11(k*2.1 + seed);
    float d = rayPtRaw(ro, rd, p, t);
    if (d > s*2.6) continue;
    float e = sin(3.1416*q);
    acc += (hot*exp(-pow((d - s)/(s*0.28), 2.0))*0.9 + col*halo(d, s)*0.22 + hot*halo(length(vec2(d - s*0.55, 0.0)), s*0.12)*0.3)*e;
  }
  return acc;
}
// Glints: small stars of four points, each flashing in its own time.
vec3 syGlints(vec3 ro, vec3 rd, float px, float n, float R, float seed, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float t, L = uLength;
  vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
  for (int i = 0; i < 14; i++){
    float k = float(i);
    if (k >= n) break;
    vec3 p = around(R*(0.5 + 0.5*h11(k*5.3 + seed)), h11(k*7.7 + seed) + uClock*0.004, L*(0.05 + 0.9*h11(k*2.3 + seed)) + 0.3*sin(uClock*0.2 + k));
    float d = rayPtRaw(ro, rd, p, t);
    if (d > 1.1) continue;
    vec3 w0 = ro + rd*t - p;
    vec2 u = vec2(dot(w0, e1), dot(w0, e2));
    float tw = pow(0.5 + 0.5*sin(uClock*(0.8 + 0.9*h11(k + seed)) + k*2.7), 3.0);
    float st = exp(-abs(u.x)*18.0)*exp(-abs(u.y)*3.4) + exp(-abs(u.y)*18.0)*exp(-abs(u.x)*3.4);
    acc += (hot*(st*0.9 + halo(d, max(0.035, px))*1.2) + col*halo(d, 0.3)*0.28)*tw;
  }
  return acc;
}
// Fireflies: soft points that wander and go dark and bright in turn.
vec3 syFlies(vec3 ro, vec3 rd, float n, float R, float seed, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float t, L = uLength;
  for (int i = 0; i < 12; i++){
    float k = float(i);
    if (k >= n) break;
    vec3 p = around(R*(0.5 + 0.5*h11(k*5.3 + seed)) + 0.5*sin(uClock*0.3 + k*2.0), h11(k*7.7 + seed) + 0.06*sin(uClock*0.17 + k), L*(0.06 + 0.88*h11(k*2.3 + seed)) + 1.1*sin(uClock*0.23 + k*1.3));
    float d = rayPtRaw(ro, rd, p, t);
    if (d > 1.5) continue;
    float b0 = pow(0.5 + 0.5*sin(uClock*(0.9 + 0.6*h11(k + seed)) + k*3.1), 2.0);
    acc += (hot*halo(d, 0.07)*1.4 + col*(halo(d, 0.24)*0.8 + halo(d, 0.6)*0.18))*b0;
  }
  return acc;
}
// Petals or leaves: small tumbling ovals, thin when seen edge on, sinking and swaying.
vec3 syPetalsAir(vec3 ro, vec3 rd, float n, float R, float fall, float seed, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float t, L = uLength;
  vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
  for (int i = 0; i < 12; i++){
    float k = float(i);
    if (k >= n) break;
    float q = fract(uClock*fall*(0.7 + 0.6*h11(k + seed)) + h11(k*3.1 + seed));
    vec3 p = around(R*(0.5 + 0.5*h11(k*5.3 + seed)) + 0.4*sin(q*7.0 + k), h11(k*7.7 + seed) + 0.1*q, L*(1.0 - q));
    float d = rayPtRaw(ro, rd, p, t);
    if (d > 0.5) continue;
    vec3 w0 = ro + rd*t - p;
    vec2 u = rot(uClock*0.5*(0.6 + h11(k)) + k*2.0)*vec2(dot(w0, e1), dot(w0, e2));
    float thin = 0.35 + 0.65*abs(sin(uClock*0.8 + k*1.7));
    float o0 = length(vec2(u.x/0.2, u.y/(0.09*thin)));
    acc += mix(col, hot, 0.5 + 0.5*sin(u.x*14.0))*(1.0 - smoothstep(0.7, 1.05, o0))*0.75*sin(3.1416*q);
  }
  return acc;
}
// Faceted crystal splinters hanging in the air and turning, lit along one face.
vec3 syCrystals(vec3 ro, vec3 rd, float px, float n, float R, float seed, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float t, L = uLength;
  vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
  for (int i = 0; i < 10; i++){
    float k = float(i);
    if (k >= n) break;
    vec3 p = around(R*(0.55 + 0.45*h11(k*5.3 + seed)), h11(k*7.7 + seed) + uClock*0.008*(mod(k, 2.0) < 0.5 ? 1.0 : -1.0), L*(0.08 + 0.84*fract(k*0.618 + seed*0.13)) + 0.4*sin(uClock*0.19 + k));
    float d = rayPtRaw(ro, rd, p, t);
    float s = 0.22 + 0.26*h11(k*2.3 + seed);
    if (d > s*2.8) continue;
    vec3 w0 = ro + rd*t - p;
    vec2 u = rot(uClock*0.12 + k*1.7)*vec2(dot(w0, e1), dot(w0, e2))/s;
    float kite = abs(u.x)*1.75 + abs(u.y + 0.2*sign(u.y)*0.0)*0.62;
    float face = u.x > 0.0 ? 1.0 : 0.42;
    acc += (mix(col, hot, 0.3 + 0.7*face*(1.0 - smoothstep(0.0, 1.0, kite)))*(1.0 - smoothstep(0.92 - px/s, 1.0 + px/s, kite))*face*0.9 + col*halo(d, s*1.3)*0.16)*(0.75 + 0.25*sin(uClock*0.7 + k*2.0));
  }
  return acc;
}
// Shafts of light standing straight out from the blade, slowly passing along it (in the blade's picture).
vec3 syShafts(vec2 b, float reach, float seed, vec3 c1, vec3 c2){
  float x = abs(b.x), side = b.x > 0.0 ? seed : seed + 3.0;
  float v = rxFlow(vec2(b.y*0.62 - uClock*0.08 + side, 1.0));
  float sh = pow(v, 3.0);
  return mix(c2, c1, sh)*sh*smoothstep(0.2, 1.0, x)*(1.0 - smoothstep(reach*0.35, reach, x));
}
// Rings of quiet going out through the air from the blade, wandering a little (in the blade's picture).
vec3 syRipples(vec2 b, float reach, float seed, vec3 c1){
  float x = abs(b.x);
  float wob = 0.7*rxFlow(vec2(b.y*0.18 + seed, uClock*0.1));
  float wave = pow(0.5 + 0.5*sin((x + wob)*2.3 - uClock*1.0 + seed), 3.0);
  return c1*wave*smoothstep(0.3, 1.1, x)*(1.0 - smoothstep(reach*0.4, reach, x));
}
// A soft bank of tinted cloud round it (in the blade's picture).
vec3 syBank(vec2 b, float reach, float seed, vec3 c1, vec3 c2){
  float x = abs(b.x);
  vec2 q = vec2(x*0.5 + seed - uClock*0.03, b.y*0.16 + 0.02*uClock);
  float n = rxFlow(q + 0.8*(rxFlow(q*1.4 + 3.0) - 0.5));
  float mass = smoothstep(0.34, 0.8, n)*(1.0 - smoothstep(reach*0.3, reach, x));
  return mix(c2, c1, smoothstep(0.45, 0.85, n))*mass;
}
// A great lick of fire leaving the blade in the air: thick where it leaves, whipping as it reaches and thinning
// to nothing; soft, with a bright heart, it curls upward as it goes.
vec3 syLick(vec3 ro, vec3 rd, float px, float y, float a, float out_, float up, float w, float k, vec3 hot, vec3 mid, vec3 deep){
  float tm;
  // nothing of it reaches further than this from the straight line through it, so the rest of the screen pays nothing
  if (raySegRaw(ro, rd, around(0.35, a, y), around(0.35 + out_, a, y + up), tm) > 0.3*out_ + 0.25*up + 0.5 + w*3.7) return vec3(0.0);
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s, bt = 0.0;
  vec3 prev = around(0.35, a, y);
  for (int j = 1; j <= 7; j++){
    float f = float(j)/7.0;
    float whip = 0.05*sin(f*7.0 - uClock*2.4 + k*2.1)*f;
    vec3 q = around(0.35 + out_*sqrt(f), a + whip, y + up*f*f + 0.35*sin(f*5.0 - uClock*1.7 + k)*f);
    float d = raySegRaw(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(j) - 1.0 + s)/7.0 - bf)*wN/bWs; bt += (dot(prev + (q - prev)*s - ro, rd) - bt)*wN/bWs; }
    prev = q;
  }
  best = hideD(best, behindBlade(ro, rd, max(bt, 0.0)));
  float ww = w*pow(1.0 - bf, 0.8)*smoothstep(0.0, 0.1, bf) + 0.012;
  float n = 0.75 + 0.5*rxFlow(vec2(bf*6.0 - uClock*1.6 + k*3.0, k));
  float e = best/max(ww*n, 0.015);
  vec3 c = mix(hot, mid, smoothstep(0.05, 0.5, bf));
  return c*exp(-e*e*2.6)*1.25 + mix(mid, deep, bf)*(exp(-e*e*0.55)*0.5 + halo(best, ww*3.6 + 0.2)*0.16);
}
// A long soft band of light winding up round the blade: a smooth curve, thin at both ends, its light travelling.
vec3 syRibbon(vec3 ro, vec3 rd, float px, float r0, float r1, float turns, float phase, float y0, float y1, float w, float k, vec3 hot, vec3 col){
  if (fxNear(ro, rd, min(y0, y1), max(y0, y1)) > max(r0, r1) + w*3.0 + 0.5) return vec3(0.0);
  float bWs = 0.0;
  float best = 1e3, bf = 0.0, s, bt = 0.0;
  float n = clamp(floor(abs(turns)*7.0) + 7.0, 7.0, 20.0);
  vec3 prev = around(r0, phase, y0);
  for (int j = 1; j <= 20; j++){
    float f = float(j)/n;
    if (f > 1.001) break;
    vec3 q = around(mix(r0, r1, f) + 0.22*sin(f*9.0 + uClock*1.1 + k), phase + turns*f, mix(y0, y1, f));
    float d = raySegRaw(ro, rd, prev, q, s);
    { float wN = bW(d); bWs += wN; best = min(best, d); bf += ((float(j) - 1.0 + s)/n - bf)*wN/bWs; bt += (dot(prev + (q - prev)*s - ro, rd) - bt)*wN/bWs; }
    prev = q;
  }
  best = hideD(best, behindBlade(ro, rd, max(bt, 0.0)));
  float taper = sin(3.1416*clamp(bf, 0.0, 1.0));
  float run = 0.45 + 0.75*pow(0.5 + 0.5*sin(bf*16.0 - uClock*3.2 + k), 2.0);
  float ww = w*taper + 0.012;
  return (hot*halo(best, max(ww*0.38, px))*0.85 + col*(halo(best, ww)*0.8 + halo(best, ww*3.0)*0.14))*run*taper;
}
// One torn lump of rock in the air: rough outline, lit from above, its edge in `rim`. No seam and no figure.
vec4 syRock(vec3 ro, vec3 rd, float px, vec3 c, float R, float k, float spin, vec3 dark, vec3 lit, vec3 rim){
  float t;
  float d = rayPt(ro, rd, c, t);
  if (d > R*1.35 + 0.06) return vec4(0.0);
  vec3 w0 = ro + rd*t - c;
  vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
  vec2 dir = vec2(dot(w0, e1), dot(w0, e2))/max(d, 0.001);
  vec2 td = rot(spin)*dir;
  float rag = R*(0.55 + 0.7*vn(td*1.5 + vec2(k*3.1, k)));
  float cov = (1.0 - smoothstep(rag - 0.03 - px, rag + 0.03 + px, d))*(1.0 - behindBlade(ro, rd, t));
  if (cov <= 0.0) return vec4(0.0);
  float near_ = smoothstep(0.0, 0.5*R, d);
  float l = 0.5 + 0.5*dot(dir, normalize(vec2(0.35, 1.0)))*near_;
  float grain = vn(td*d*5.0 + vec2(k, 2.0));
  return vec4(mix(dark, lit, l*(0.6 + 0.6*grain)) + rim*smoothstep(rag*0.7, rag, d), cov);
}

// ---------------------------------------------------------------- the round's extra effects
// All of them stand in three dimensions round the blade (Sol's rule, as on his Sun and Void), and
// none of them appears or goes out at once: each piece swells and fades over its whole life.
float gxEnv(float q){ return smoothstep(0.0, 0.2, q)*(1.0 - smoothstep(0.7, 1.0, q)); }
float gx3(vec3 p){
  vec3 i0 = floor(p), f0 = fract(p);
  f0 = f0*f0*(3.0 - 2.0*f0);
  vec2 q = i0.xy + 37.0*i0.z;
  float a0 = mix(mix(h21(q), h21(q + vec2(1.0, 0.0)), f0.x), mix(h21(q + vec2(0.0, 1.0)), h21(q + vec2(1.0, 1.0)), f0.x), f0.y);
  q += 37.0;
  float b0 = mix(mix(h21(q), h21(q + vec2(1.0, 0.0)), f0.x), mix(h21(q + vec2(0.0, 1.0)), h21(q + vec2(1.0, 1.0)), f0.x), f0.y);
  return mix(a0, b0, f0.z);
}
// Embers thrown off the blade: each leaves it, arcs out and falls, a short bright tail behind it.
vec3 gxFountain(vec3 ro, vec3 rd, float px, float n, float R, float rate, float up, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 16; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uTime*rate*(0.75 + 0.5*h11(k + s0)) + h11(k*3.1 + s0), cyc = floor(run), q = fract(run);
    float a = h11(k*5.3 + cyc + s0), y0 = L*(0.08 + 0.78*h11(k*7.7 + cyc + s0)), Rk = R*(0.6 + 0.5*h11(k*2.1 + cyc));
    float q0 = max(q - 0.07, 0.0);
    vec3 p1 = around(0.45 + Rk*q, a + 0.1*q, y0 + up*(q - q*q*1.5));
    vec3 p0 = around(0.45 + Rk*q0, a + 0.1*q0, y0 + up*(q0 - q0*q0*1.5));
    float d = raySegRaw(ro, rd, p0, p1, tm);
    if (d > 0.7) continue;
    acc += (hot*halo(d, max(0.05, px))*1.3 + col*halo(d, 0.26)*0.42)*(0.25 + 0.75*tm)*gxEnv(q);
  }
  return acc;
}
// One flat piece turning in real space at c0, catching the light as it turns.
vec3 gxPiece(vec3 ro, vec3 rd, float px, vec3 c0, float sz, float k, float shape, vec3 hot, vec3 col){
  float tm, d = rayPtRaw(ro, rd, c0, tm), h0 = h11(k*3.1), h2 = h11(k*9.3);
  if (d > sz*3.5) return vec3(0.0);
  float a1 = uTime*(0.25 + 0.3*h2) + k*2.0, a2 = uTime*(0.17 + 0.2*h0) + k*3.0;
  vec3 nn = normalize(vec3(sin(a1)*cos(a2), sin(a2), cos(a1)*cos(a2)));
  vec3 t1 = normalize(cross(nn, vec3(0.3, 1.0, 0.2))), t2 = cross(nn, t1);
  float den = dot(rd, nn), cov = 0.0, tp = 0.0;
  if (abs(den) > 0.04){
    float th = dot(c0 - ro, nn)/den;
    vec3 w0 = ro + rd*th - c0;
    vec2 u = vec2(dot(w0, t1), dot(w0, t2))/sz;
    float m0 = shape > 0.5 ? length(vec2(u.x*2.2*(1.0 + 0.3*u.y), u.y)) : abs(u.x)*2.6 + abs(u.y)*0.8;
    float e0 = max(px/sz, 0.03);
    cov = 1.0 - smoothstep(0.92 - e0, 1.0 + e0, m0);
    tp = smoothstep(0.55, 1.0, m0);
  }
  float flash = 0.35 + 0.65*pow(abs(dot(nn, normalize(vec3(0.5, 0.75, 0.42)))), 3.0);
  return mix(col*0.55, hot, flash)*cov*(0.5 + 0.5*tp) + col*halo(d, sz*1.4)*0.16;
}
// Tumbling flat pieces, each turning in real space and catching the light as it turns.
// mode 0 carried round the blade, 1 sinking, 2 rising, 3 bursting slowly outward. shape 0 a splinter, 1 a soft leaf.
vec3 gxTumble(vec3 ro, vec3 rd, float px, float n, float R, float mode, float shape, float size, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 14; i++){
    float k = float(i);
    if (k >= n) break;
    float h0 = h11(k*3.1 + s0), h1 = h11(k*5.7 + s0), h2 = h11(k*9.3 + s0);
    float q = fract(uTime*(0.035 + 0.03*h0) + h1), cyc = floor(uTime*(0.035 + 0.03*h0) + h1), life = gxEnv(q);
    float rr = R*(0.55 + 0.45*h2), a = h1 + uTime*0.05*(0.6 + h2), y = L*(0.1 + 0.82*h11(k*2.3 + (mode > 0.5 ? cyc : 0.0) + s0)) + (mode > 0.5 ? 0.0 : 1.4*sin(uTime*0.14*(0.6 + h0) + k*2.0));
    if (mode > 2.5){ rr = 0.5 + R*(1.0 - (1.0 - q)*(1.0 - q)); a = h11(k + cyc + s0); }
    else if (mode > 1.5) y = L*(0.04 + 0.92*q);
    else if (mode > 0.5) y = L*(0.96 - 0.92*q);
    else life = 1.0;
    if (life <= 0.0) continue;
    vec3 c0 = around(rr, a, y);
    float d = rayPtRaw(ro, rd, c0, tm), sz = size*(0.7 + 0.6*h0);
    if (d > sz*3.5) continue;
    // the face's turning frame
    float a1 = uTime*(0.25 + 0.3*h2) + k*2.0, a2 = uTime*(0.17 + 0.2*h0) + k*3.0;
    vec3 nn = normalize(vec3(sin(a1)*cos(a2), sin(a2), cos(a1)*cos(a2)));
    vec3 t1 = normalize(cross(nn, vec3(0.3, 1.0, 0.2))), t2 = cross(nn, t1);
    float den = dot(rd, nn), glow = halo(d, sz*1.4)*0.16;
    float cov = 0.0, tp = 0.0;
    if (abs(den) > 0.04){
      float th = dot(c0 - ro, nn)/den;
      vec3 w0 = ro + rd*th - c0;
      vec2 u = vec2(dot(w0, t1), dot(w0, t2))/sz;
      float m0 = shape > 0.5 ? length(vec2(u.x*2.2, u.y)*vec2(1.0 + 0.3*u.y, 1.0)) : abs(u.x)*2.6 + abs(u.y)*0.8;
      float e0 = max(px/sz, 0.03);
      cov = 1.0 - smoothstep(0.92 - e0, 1.0 + e0, m0);
      tp = smoothstep(0.55, 1.0, m0);
    }
    float flash = 0.35 + 0.65*pow(abs(dot(nn, normalize(vec3(0.5, 0.75, 0.42)))), 3.0);
    acc += (mix(col*0.55, hot, flash)*cov*(0.5 + 0.5*tp) + col*glow)*life;
  }
  return acc;
}
// Round bodies with real depth: a lit side, a shadow side and a bright rim, drifting without ever going out.
vec3 gxOrbs(vec3 ro, vec3 rd, float px, float n, float R, float size, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 8; i++){
    float k = float(i);
    if (k >= n) break;
    float h0 = h11(k*3.7 + s0), h1 = h11(k*6.1 + s0);
    vec3 c0 = around(R*(0.7 + 0.3*sin(uTime*0.09*(0.6 + h1) + k*2.0)), h0 + uTime*0.022*(h1 - 0.35), L*(0.5 + 0.42*sin(uTime*0.05*(0.5 + h0) + k*2.4 + s0)));
    float d = rayPtRaw(ro, rd, c0, tm), sz = size*(0.6 + 0.7*h1);
    if (d > sz*3.2) continue;
    acc += col*halo(max(d - sz, 0.0), sz*0.9)*0.2;
    if (d < sz){
      vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1), w0 = ro + rd*tm - c0;
      float z0 = sqrt(max(1.0 - d*d/(sz*sz), 0.0));
      vec3 nn = normalize(w0/sz - rd*z0);
      float lit = clamp(dot(nn, normalize(vec3(0.45, 0.8, 0.4))), 0.0, 1.0);
      float cov = 1.0 - smoothstep(0.86 - px/sz, 1.0, d/sz);
      float grain = 0.8 + 0.4*gx3(nn*3.0 + vec3(0.0, uTime*0.05, s0));
      acc += (col*(0.3 + 0.8*lit)*grain + hot*pow(lit, 5.0)*0.9 + col*pow(1.0 - z0, 3.0)*0.2)*cov;
    }
  }
  return acc;
}
// Comets climbing a wide spiral round the blade, their tails following the same path.
vec3 gxComets(vec3 ro, vec3 rd, float px, float n, float R, float rate, float turns, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 5; i++){
    float k = float(i);
    if (k >= n) break;
    float bWs = 0.0;
    float q = fract(uTime*rate + k/n + h11(k + s0)*0.2), a0 = h11(k*5.3 + s0), best = 1e3, bs = 0.0;
    vec3 prev = around(R*(0.7 + 0.3*sin(q*6.3)), a0 + turns*q, L*q);
    for (int j = 1; j <= 5; j++){
      float u = max(q - float(j)*0.018, 0.0);
      vec3 p = around(R*(0.7 + 0.3*sin(u*6.3)), a0 + turns*u, L*u);
      float d = raySegRaw(ro, rd, prev, p, tm);
      { float wN = bW(d); bWs += wN; best = min(best, d); bs += ((float(j) - 1.0 + tm)/5.0 - bs)*wN/bWs; }
      prev = p;
    }
    if (best > 1.2) continue;
    acc += (hot*halo(best, max(0.06 + 0.1*(1.0 - bs), px))*1.4*(1.0 - bs)*(1.0 - bs) + col*halo(best, 0.42)*0.3*(1.0 - bs))*sin(3.1416*q);
  }
  return acc;
}
// Arches standing off the blade with beads of light sliding down them, as coronal rain does.
vec3 gxBeads(vec3 ro, vec3 rd, float px, float n, float H, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 3; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uTime*0.06*(0.8 + 0.4*h11(k + s0)) + h11(k*5.1 + s0), cyc = floor(run), q = fract(run), life = gxEnv(q);
    float side = h11(cyc*4.9 + k*2.2 + s0), span = 5.0 + 4.0*h11(cyc*2.3 + k), y0 = L*(0.1 + 0.62*h11(cyc*1.7 + k*4.1 + s0)), Hk = H*(0.7 + 0.4*h11(cyc*3.1 + k));
    if (raySegRaw(ro, rd, around(0.35 + 0.5*Hk, side, y0), around(0.35 + 0.5*Hk, side, y0 + span), tm) > 0.5*Hk + 1.2) continue;
    vec3 prev = around(0.35, side, y0);
    float best = 1e3;
    for (int j = 1; j <= 8; j++){
      float f = float(j)/8.0;
      vec3 p = around(0.35 + Hk*sin(3.1416*f), side, y0 + span*f);
      best = min(best, raySegRaw(ro, rd, prev, p, tm));
      prev = p;
    }
    acc += col*(halo(best, max(0.05, px))*0.3 + halo(best, 0.4)*0.12)*life;
    for (int j = 0; j < 5; j++){
      float fj = float(j), g = fract(uTime*(0.14 + 0.05*h11(fj*1.9 + k)) + h11(fj*6.7 + k*3.3)), f = 0.5 + (mod(fj, 2.0) < 0.5 ? -0.5 : 0.5)*g*g;
      float d = rayPtRaw(ro, rd, around(0.35 + Hk*sin(3.1416*f), side, y0 + span*f), tm);
      acc += (hot*halo(d, max(0.09, px))*1.6 + col*halo(d, 0.38)*0.35)*gxEnv(g)*life;
    }
  }
  return acc;
}
// Great slow stars opening in the air: long soft rays and a wide bloom.
vec3 gxFlares(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
  for (int i = 0; i < 8; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uTime*(0.07 + 0.05*h11(k + s0)) + h11(k*3.3 + s0), cyc = floor(run), q = fract(run);
    vec3 c0 = around(R*(0.45 + 0.55*h11(k*7.1 + cyc + s0)), h11(k*2.7 + cyc + s0), L*(0.08 + 0.85*h11(k*5.3 + cyc + s0)));
    float d = rayPtRaw(ro, rd, c0, tm);
    if (d > 2.6) continue;
    vec3 w0 = ro + rd*tm - c0;
    vec2 u = rot(k + 0.05*uTime)*vec2(dot(w0, e1), dot(w0, e2));
    float o0 = pow(sin(3.1416*q), 2.0), st = exp(-abs(u.x)*11.0)*exp(-abs(u.y)*1.5) + exp(-abs(u.y)*11.0)*exp(-abs(u.x)*1.5);
    acc += (hot*(st*0.8 + halo(d, max(0.07, px))*1.6) + col*halo(d, 0.85)*0.32)*o0;
  }
  return acc;
}
// A shoal: points that keep together and wheel as one round the blade.
vec3 gxSwarm(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int g = 0; g < 2; g++){
    float g0 = float(g);
    vec3 c0 = around(R*(0.75 + 0.25*sin(uTime*0.13 + g0*2.0 + s0)), uTime*0.035*(1.0 - 2.0*g0) + g0*0.5 + s0*0.13, L*(0.5 + 0.38*sin(uTime*0.09 + g0*3.0 + s0)));
    if (rayPtRaw(ro, rd, c0, tm) > 2.9) continue;
    for (int i = 0; i < 12; i++){
      float k = float(i);
      if (k >= n) break;
      float h0 = h11(k*3.1 + g0 + s0), h1 = h11(k*5.9 + g0 + s0), w0 = uTime*(0.3 + 0.4*h0) + k*2.4;
      vec3 p = c0 + 1.9*vec3(sin(w0)*h1, cos(w0*0.8 + k)*0.8, sin(w0*0.6 + k*1.7)*h0);
      float d = rayPtRaw(ro, rd, p, tm);
      acc += (hot*halo(d, max(0.05, px))*1.3 + col*halo(d, 0.2)*0.34)*(0.55 + 0.45*sin(w0*1.3));
    }
  }
  return acc;
}
// Beads of light climbing in two strands wound round the blade: points only, never a drawn line.
vec3 gxHelix(vec3 ro, vec3 rd, float px, float n, float R, float turns, float rate, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 18; i++){
    float k = float(i);
    if (k >= n) break;
    float q = fract(k/n + uTime*rate), two = mod(k, 2.0)*0.5;
    float d = rayPtRaw(ro, rd, around(R, s0*0.17 + two + turns*q, L*q), tm);
    if (d > 0.9) continue;
    acc += (hot*halo(d, max(0.07, px))*1.3 + col*halo(d, 0.3)*0.32)*sin(3.1416*q);
  }
  return acc;
}
// Falling stars crossing the air beside the blade.
vec3 gxMeteors(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 6; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uTime*(0.08 + 0.05*h11(k + s0)) + h11(k*3.7 + s0), cyc = floor(run), q = fract(run);
    float a = h11(k*2.3 + cyc + s0), y0 = L*(0.45 + 0.5*h11(k*5.9 + cyc + s0)), f0 = max(q - 0.14, 0.0);
    vec3 p1 = around(R*(0.5 + 0.8*q), a + 0.12*q, y0 - L*0.32*q), p0 = around(R*(0.5 + 0.8*f0), a + 0.12*f0, y0 - L*0.32*f0);
    float d = raySegRaw(ro, rd, p0, p1, tm);
    if (d > 0.8) continue;
    acc += (hot*halo(d, max(0.05, px))*1.3*tm*tm + col*halo(d, 0.3)*0.3*tm)*gxEnv(q);
  }
  return acc;
}
// Motes drawn in from far off and spiralling down into the blade, where they go out.
vec3 gxVortex(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 16; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uTime*(0.07 + 0.05*h11(k + s0)) + h11(k*7.1 + s0), cyc = floor(run), q = fract(run), q0 = max(q - 0.04, 0.0);
    float a = h11(k*3.7 + cyc + s0), y = L*(0.1 + 0.8*h11(k*5.1 + cyc + s0));
    vec3 p1 = around(0.5 + R*pow(1.0 - q, 1.5), a + 0.9*q, y), p0 = around(0.5 + R*pow(1.0 - q0, 1.5), a + 0.9*q0, y);
    float d = raySegRaw(ro, rd, p0, p1, tm);
    if (d > 0.7) continue;
    acc += (hot*halo(d, max(0.05, px))*1.2 + col*halo(d, 0.22)*0.3)*gxEnv(q);
  }
  return acc;
}
// Thin drops falling past it, each swelling in and fading long before the ground.
vec3 gxRain(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 16; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uTime*(0.14 + 0.1*h11(k + s0)) + h11(k*3.7 + s0), cyc = floor(run), q = fract(run);
    vec3 c0 = around(R*(0.45 + 0.55*h11(k*2.9 + cyc + s0)), h11(k*6.3 + cyc + s0), L*(0.95 - 0.9*q));
    float d = raySegRaw(ro, rd, c0, c0 + vec3(0.0, 0.9, 0.0), tm);
    if (d > 0.5) continue;
    acc += (hot*halo(d, max(0.04, px))*0.95*(1.0 - tm) + col*halo(d, 0.16)*0.28)*gxEnv(q);
  }
  return acc;
}
// Lanterns of soft light rising slowly and rocking as they go.
vec3 gxLanterns(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 8; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uTime*(0.035 + 0.025*h11(k + s0)) + h11(k*5.3 + s0), cyc = floor(run), q = fract(run);
    float a = h11(k*2.9 + cyc + s0) + 0.03*sin(uTime*0.5 + k);
    vec3 c0 = around(R*(0.55 + 0.45*h11(k*7.7 + cyc + s0)) + 0.5*sin(q*9.0 + k), a, L*(0.03 + 0.93*q));
    float d = rayPtRaw(ro, rd, c0, tm);
    if (d > 2.4) continue;
    float br = 0.8 + 0.2*sin(uTime*1.1 + k*2.0);
    acc += (hot*halo(d, max(0.19, px))*1.25 + col*(halo(d, 0.55)*0.55 + halo(d, 1.5)*0.14))*br*gxEnv(q);
  }
  return acc;
}
// Small worlds with moons going round them, the worlds themselves slowly round the blade.
vec3 gxPlanets(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = gxOrbs(ro, rd, px, n, R, 0.62, s0, hot, col);
  float tm, L = uLength;
  for (int i = 0; i < 4; i++){
    float k = float(i), g = mod(k, max(n, 1.0));
    float h0 = h11(g*3.7 + s0), h1 = h11(g*6.1 + s0);
    vec3 c0 = around(R*(0.7 + 0.3*sin(uTime*0.09*(0.6 + h1) + g*2.0)), h0 + uTime*0.022*(h1 - 0.35), L*(0.5 + 0.42*sin(uTime*0.05*(0.5 + h0) + g*2.4 + s0)));
    float w0 = uTime*(0.5 + 0.2*k) + k*1.6, rr = 1.05 + 0.32*k;
    float d = rayPtRaw(ro, rd, c0 + vec3(rr*cos(w0), 0.35*rr*sin(w0*0.7), rr*sin(w0)), tm);
    acc += (hot*halo(d, max(0.08, px))*1.2 + col*halo(d, 0.24)*0.28);
  }
  return acc;
}
// Bands of wind: short streaks wheeling round the blade at many heights.
vec3 gxGale(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 16; i++){
    float k = float(i);
    if (k >= n) break;
    float q = fract(uTime*(0.04 + 0.03*h11(k + s0)) + h11(k*3.1 + s0)), cyc = floor(uTime*(0.04 + 0.03*h11(k + s0)) + h11(k*3.1 + s0));
    float rr = R*(0.6 + 0.4*h11(k*2.3 + cyc + s0)), y = L*(0.06 + 0.88*h11(k*7.3 + cyc + s0)), a = h11(k*5.7 + s0) + uTime*(0.1 + 0.08*h11(k*1.3 + s0));
    float d = raySegRaw(ro, rd, around(rr, a, y), around(rr, a - 0.07, y - 0.15), tm);
    if (d > 0.6) continue;
    acc += (hot*halo(d, max(0.04, px))*0.95*tm + col*halo(d, 0.18)*0.26)*gxEnv(q);
  }
  return acc;
}
// A train of lights tracing a slow figure of eight round the blade, without end.
vec3 gxInfinity(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 24; i++){
    float k = float(i);
    if (k >= n) break;
    float u = 6.2832*(k/n) + uTime*0.16 + s0;
    vec3 p = around(R*(0.8 + 0.2*cos(2.0*u)), 0.5*sin(u)/3.1416 + s0*0.1, L*(0.5 + 0.4*sin(u)*cos(u)*1.6));
    float d = rayPtRaw(ro, rd, p, tm), w0 = 0.6 + 0.4*sin(k*6.2832/n*3.0 - uTime*0.7);
    acc += (hot*halo(d, max(0.08, px))*1.2 + col*halo(d, 0.34)*0.32)*w0;
  }
  return acc;
}
// Two bodies, one of light and one of deep colour, going slowly round each other as they climb and sink along the blade.
vec3 gxBinary(vec3 ro, vec3 rd, float px, float R, float s0, vec3 a0, vec3 b0, vec3 glow0){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  vec3 mid = around(R, uTime*0.03 + s0, L*(0.5 + 0.34*sin(uTime*0.08 + s0)));
  for (int i = 0; i < 2; i++){
    float sg = i == 0 ? 1.0 : -1.0, w0 = uTime*0.42 + s0;
    vec3 c0 = mid + sg*1.25*vec3(cos(w0), 0.45*sin(w0*0.7), sin(w0));
    float d = rayPtRaw(ro, rd, c0, tm);
    acc += glow0*halo(d, 1.3)*0.22;
    if (d < 0.6){
      vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1), w1 = ro + rd*tm - c0;
      float z0 = sqrt(max(1.0 - d*d/0.36, 0.0));
      vec3 nn = normalize(w1/0.6 - rd*z0);
      float lit = clamp(dot(nn, normalize(vec3(0.4, 0.8, 0.4))), 0.0, 1.0);
      acc += (i == 0 ? a0 : b0)*(0.25 + 0.95*lit)*(1.0 - smoothstep(0.84 - px/0.6, 1.0, d/0.6));
    }
  }
  return acc;
}
// Thunderheads: soft clouds hanging by the blade, lit slowly from inside as a far storm lights them.
vec3 gxThunderheads(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 6; i++){
    float k = float(i);
    if (k >= n) break;
    vec3 c0 = around(R*(0.75 + 0.25*sin(uTime*0.07 + k)), k/n + uTime*0.018 + s0, L*(0.14 + 0.72*(k + 0.5)/n) + 1.2*sin(uTime*0.1 + k*2.0));
    float d = rayPtRaw(ro, rd, c0, tm);
    if (d > 3.6) continue;
    vec3 w0 = ro + rd*tm - c0;
    float grain = 0.55 + 0.75*vn(w0.xy*1.1 + vec2(uTime*0.05, k*3.0)), flash = pow(0.5 + 0.5*sin(uTime*(0.45 + 0.1*k) + k*2.4), 8.0);
    acc += (col*(0.2 + 0.25*grain) + hot*flash*0.9*grain)*halo(d, 1.55);
  }
  return acc;
}
// ---- What stands round a new blade, in place of the same round lights on every one (Creyon, 4/11/2026: "waarom plak je
// alles hetzelfde"). Each blade picks what its world would really throw off; everything is in the air in three dimensions
// and soft at its edge. kind: 0 grains on the wind, 1 flecks fluttering, 2 drops with a tail, 3 long thin threads,
// 4 flat leaf that flashes as it turns, 5 slow soft smoke, 6 bristles standing off the blade, 7 big faint panes turning,
// 8 slanted streaks climbing round it. y0, y1 where along the blade, dir which way it goes.
vec3 gxKin(vec3 ro, vec3 rd, float kind, float n, float R, float speed, float size, float y0, float y1, float dir, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float L = uLength, s, tm;
  vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
  for (int i = 0; i < 14; i++){
    float k = float(i);
    if (k >= n) break;
    float h0 = h11(k*3.1 + s0), h1 = h11(k*5.7 + s0);
    float run = uTime*speed*(0.7 + 0.6*h0) + h1, cyc = floor(run), q = fract(run), env = gxEnv(q);
    float h4 = h11(k*2.3 + cyc + s0), h5 = h11(k*7.9 + cyc + s0);
    float Y = L*mix(y0, y1, h4), rr = R*(0.45 + 0.55*h5), a0 = h5*3.7 + k*0.37;
    if (kind > 3.5 && kind < 4.5 || kind > 6.5 && kind < 7.5){
      // a flat piece turning in the air
      vec3 c0 = around(rr, a0 + 0.04*sin(q*6.0 + k), Y + 0.6*sin(q*5.0 + k));
      float d = rayPtRaw(ro, rd, c0, tm);
      if (d > size*2.6) continue;
      float a1 = uTime*(0.35 + 0.35*h0) + k*2.0, a2 = uTime*(0.22 + 0.25*h1) + k*3.0;
      vec3 nn = normalize(vec3(sin(a1)*cos(a2), sin(a2), cos(a1)*cos(a2)));
      vec3 t1 = normalize(cross(nn, vec3(0.3, 1.0, 0.2))), t2 = cross(nn, t1);
      float den = dot(rd, nn);
      if (abs(den) < 0.05) continue;
      vec3 w0 = ro + rd*(dot(c0 - ro, nn)/den) - c0;
      vec2 u = vec2(dot(w0, t1), dot(w0, t2))/size;
      float m0 = kind < 4.5 ? max(abs(u.x), abs(u.y)*0.7) : abs(u.x)*0.8 + abs(u.y)*1.5;
      float cov = 1.0 - smoothstep(0.45, 1.0, m0);
      float flash = pow(abs(dot(nn, normalize(vec3(0.5, 0.75, 0.42)))), kind < 4.5 ? 10.0 : 3.0);
      acc += (kind < 4.5 ? mix(col*0.5, hot, flash)*(0.25 + 1.3*flash) : mix(col, hot, flash)*(0.14 + 0.32*flash))*cov*env;
    } else if (kind > 4.5 && kind < 5.5){
      // smoke: it rises, spreads and thins
      vec3 c0 = around(rr*(0.5 + 0.6*q), a0 + 0.05*q, Y + L*0.14*q);
      float r0 = size*(0.6 + 1.1*q), d = rayPtRaw(ro, rd, c0, tm);
      if (d > r0*1.5) continue;
      vec3 w0 = ro + rd*tm - c0;
      vec2 u = vec2(dot(w0, e1), dot(w0, e2))/r0;
      float eat = vn(u*1.8 + vec2(k*3.0, uTime*0.1)) + 0.5*vn(u*4.1 + vec2(k, -uTime*0.14));
      acc += mix(col, hot, eat*0.4)*halo(d/r0 + 0.5*(eat/1.5 - 0.5), 0.72)*0.3*env;
    } else {
      // something long and thin: its two ends, its width, and how it tapers
      vec3 A, B;
      float W = size*0.2, shapeAt = 0.5;
      if (kind < 0.5){
        float ta = a0 + dir*q*0.4, da = size*1.8/(6.2832*max(rr, 0.5)), y = Y + 0.4*sin(q*7.0 + k);
        A = around(rr, ta - da, y); B = around(rr, ta + da, y); W = size*0.16;
      } else if (kind < 1.5){
        float y = Y + dir*q*L*0.24;
        A = around(rr + 0.3*sin(q*8.0 + k), a0 + 0.06*sin(q*5.0 + k), y);
        B = A + vec3(sin(q*17.0 + k), 0.8, cos(q*13.0 + k))*size; W = size*0.3;
      } else if (kind < 2.5){
        float y = L*mix(y0, y1, dir > 0.0 ? q : 1.0 - q);
        A = around(rr, a0, y); B = A + vec3(0.0, size*(3.0 + 3.0*q), 0.0); W = size*0.3; shapeAt = 0.0;
      } else if (kind < 3.5){
        float y = Y + dir*q*L*0.2, long_ = size*L*(0.6 + 0.8*h0);
        A = around(rr, a0 + 0.015*sin(q*4.0), y - long_*0.5); B = around(rr, a0 + 0.015*sin(q*4.0 + 1.0), y + long_*0.5); W = max(0.02 + size*0.12, 0.03);
      } else if (kind < 6.5){
        float sway = 0.03*sin(uTime*0.6 + k*2.0), out_ = 0.5 + 0.5*sin(3.1416*q);
        A = around(0.7, a0, Y); B = around(0.7 + rr*out_, a0 + sway, Y + size*2.0*dir); W = size*0.2; shapeAt = 0.0;
      } else {
        float y = Y + dir*q*L*0.3, ta = a0 + dir*q*0.5;
        A = around(rr, ta, y); B = around(rr, ta + dir*0.1, y + dir*size*3.0); W = size*0.16;
      }
      float d = raySegRaw(ro, rd, A, B, s);
      if (d > W*6.0 + 0.2) continue;
      // tapered to nothing at both ends, or from its thick head to nothing
      float t = shapeAt > 0.25 ? sin(3.1416*clamp(s, 0.0, 1.0)) : pow(1.0 - clamp(s, 0.0, 1.0), 1.5);
      float wn = max(W*(0.3 + 0.7*t), 0.02);
      acc += (hot*halo(d, wn) + col*halo(d, wn*3.0)*0.35)*t*env;
    }
  }
  return acc;
}
// Small clouds that come away from a bank and drift off, thinning as they go.
vec3 gxCloudlets(vec3 ro, vec3 rd, float px, float n, float R, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 6; i++){
    float k = float(i);
    if (k >= n) break;
    float run = uTime*(0.035 + 0.02*h11(k + s0)) + h11(k*5.3 + s0), cyc = floor(run), q = fract(run);
    vec3 c0 = around(R*(0.5 + 0.7*q), h11(k*2.9 + cyc + s0) + 0.05*q, L*(0.1 + 0.8*h11(k*7.1 + cyc + s0)) - 2.5*q);
    float d = rayPtRaw(ro, rd, c0, tm);
    if (d > 2.8) continue;
    vec3 w0 = ro + rd*tm - c0;
    // (measured across the view, or it was drawn as stripes from some sides)
    vec3 e1 = normalize(cross(rd, vec3(0.0, 1.0, 0.0))), e2 = cross(rd, e1);
    float grain = 0.55 + 0.75*vn(vec2(dot(w0, e1), dot(w0, e2))*1.6 + vec2(k*3.0 + uTime*0.04, k));
    acc += mix(col, hot, grain*0.8)*smoothstep(0.08, 0.95, grain*halo(d, 1.45))*gxEnv(q)*0.55;
  }
  return acc;
}
// What a clash knocks loose: pieces thrown from the blade, turning, slowing and fading over a second and a half.
// shape 0 a splinter, 1 a soft leaf, 2 a point of light.
vec3 gxBurst(vec3 ro, vec3 rd, float px, float n, float R, float shape, float size, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float q = clamp(uClashT/1.6, 0.0, 1.0), L = uLength, tm;
  if (q >= 1.0) return acc;
  float out_ = 1.0 - pow(1.0 - q, 2.6), life = smoothstep(0.0, 0.05, q)*pow(1.0 - q, 1.3);
  for (int i = 0; i < 14; i++){
    float k = float(i);
    if (k >= n) break;
    float h0 = h11(k*3.3 + s0), h1 = h11(k*6.1 + s0), h2 = h11(k*8.9 + s0);
    vec3 p = around(0.5 + R*(0.5 + 0.5*h1)*out_, h2 + 0.08*out_, L*(0.42 + 0.34*h0) + (h1 - 0.5)*9.0*out_ - 1.6*q*q);
    if (shape > 1.5){
      float d = rayPtRaw(ro, rd, p, tm);
      acc += (hot*halo(d, max(0.07, px))*1.6 + col*halo(d, 0.3)*0.4)*life;
    } else acc += gxPiece(ro, rd, px, p, size*(0.7 + 0.6*h0), k + s0, shape, hot, col)*life*1.4;
  }
  return acc;
}
// A field of pieces moved as one by a great moment (e0 0 to 1): from all along the blade at radius R0 to radius R1, turning `turn`
// of the way round as they go; with yIn at or above 0 they gather near that share of its length. Each piece goes in its own time.
vec3 gxField(vec3 ro, vec3 rd, float px, float n, float e0, float R0, float R1, float turn, float yIn, float shape, float size, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  if (e0 <= 0.002) return acc;
  float L = uLength;
  for (int i = 0; i < 16; i++){
    float k = float(i);
    if (k >= n) break;
    float h0 = h11(k*3.1 + s0), h1 = h11(k*5.7 + s0), h2 = h11(k*8.3 + s0);
    float e1 = smoothstep(h0*0.3, 0.7 + h0*0.3, e0);
    float own = L*(0.06 + 0.88*h1);
    float y = yIn < 0.0 ? own : mix(own, L*yIn + (h0 - 0.5)*7.0, e1);
    vec3 p = around(mix(R0*(0.65 + 0.35*h1), R1*(0.8 + 0.4*h2), e1), h2 + uTime*0.035 + turn*e1, y);
    acc += gxPiece(ro, rd, px, p, size*(0.7 + 0.6*h0), k + s0, shape, hot, col)*smoothstep(0.0, 0.22, e0);
  }
  return acc;
}
// Thunderheads lighting one after another from the hand to the point, a chorus; c0 how far through it is.
vec3 gxChorus(vec3 ro, vec3 rd, float px, float n, float R, float c0, float s0, vec3 hot, vec3 col){
  vec3 acc = vec3(0.0);
  float tm, L = uLength;
  for (int i = 0; i < 7; i++){
    float k = float(i);
    if (k >= n) break;
    float f0 = (k + 0.5)/n, beat = exp(-pow((c0*1.3 - 0.15 - f0)/0.1, 2.0));
    if (beat < 0.01) continue;
    float d = rayPtRaw(ro, rd, around(R*(0.8 + 0.2*sin(k*2.0)), k*0.37 + s0 + uTime*0.018, L*f0), tm);
    acc += (hot*halo(d, 0.9)*0.9 + col*halo(d, 2.4)*0.42)*beat;
  }
  return acc;
}

