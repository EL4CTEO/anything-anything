// "anything-anything: the -anything trend, explained" — renders trend.gif with a canvas + gifenc.
import { createCanvas, GlobalFonts } from "@napi-rs/canvas";
import gifenc from "gifenc";
import { writeFileSync } from "node:fs";
const { GIFEncoder, quantize, applyPalette } = gifenc;

GlobalFonts.registerFromPath("C:/Windows/Fonts/segoeuib.ttf", "Sans Bold");
GlobalFonts.registerFromPath("C:/Windows/Fonts/segoeui.ttf", "Sans");
GlobalFonts.registerFromPath("C:/Windows/Fonts/segoeuii.ttf", "Sans Italic");
GlobalFonts.registerFromPath("C:/Windows/Fonts/consola.ttf", "Mono");
GlobalFonts.registerFromPath("C:/Windows/Fonts/consolab.ttf", "Mono Bold");
GlobalFonts.registerFromPath("C:/Windows/Fonts/seguisym.ttf", "Sym");
GlobalFonts.registerFromPath("C:/Windows/Fonts/seguiemj.ttf", "Emoji");

const W = 800, H = 450, FPS = 20;
const BG = "#0d0f14", PANEL = "#161921", TEXT = "#eceef4", MUTED = "#969caa", VIOLET = "#a770ff", PINK = "#f472b6", GREEN = "#3fb950", RED = "#f85149", INK = "#110c22";
const c = createCanvas(W, H);
const g = c.getContext("2d");

const ease = (t) => (t < 0 ? 0 : t > 1 ? 1 : 1 - Math.pow(1 - t, 3));
const clamp = (t) => Math.max(0, Math.min(1, t));
function rr(x, y, w, h, r) { g.beginPath(); g.roundRect(x, y, w, h, r); }
function text(s, x, y, font, color, align = "left") { g.font = font; g.fillStyle = color; g.textAlign = align; g.textBaseline = "middle"; g.fillText(s, x, y); }

// ---- scene 1: trending is drowning in -anything ----
const REPOS = [
  ["Egonex-AI/Understand-Anything", "93.8k"], ["Mintplex-Labs/anything-llm", "68.6k"], ["HKUDS/CLI-Anything", "63.6k"],
  ["facebookresearch/segment-anything", "55.2k"], ["storytold/craft-anything", "42.1k"], ["HKUDS/RAG-Anything", "24.8k"],
  ["DepthAnything/Depth-Anything-V2", "9.2k"], ["erkamyaman/rust-anything", "8.7k"], ["geekyutao/Inpaint-Anything", "7.7k"],
  ["gaomingqi/Track-Anything", "7.0k"], ["EL4CTEO/fast-anything", "3.1k"], ["sail-sg/EditAnything", "3.4k"], ["OpenGVLab/Ask-Anything", "3.4k"],
  ["ttengwang/Caption-Anything", "1.8k"], ["zhayujie/bot-on-anything", "4.2k"], ["goodnight000/api-anything", "3.8k"], ["EL4CTEO/shrink-anything", "new"], ["you/your-idea-anything", "0"],
];
function scene1(t) { // 0..3.6s
  g.fillStyle = BG; g.fillRect(0, 0, W, H);
  text("GitHub trending, 2026", 40, 36, "22px 'Sans Bold'", TEXT);
  text("★ every repo this week", 40, 62, "15px Sans", MUTED);
  // cards arrive faster and faster
  let shown = 0, at = 0.25, gap = 0.42;
  const times = [];
  for (let i = 0; i < REPOS.length; i++) { times.push(at); at += gap; gap *= 0.8; }
  const scroll = Math.max(0, (REPOS.filter((_, i) => times[i] < t).length - 7)) * 44;
  for (let i = 0; i < REPOS.length; i++) {
    const k = ease((t - times[i]) / 0.25);
    if (k <= 0) continue;
    shown++;
    const y = 100 + i * 44 - scroll * ease(clamp((t - 1.2) / 0.3));
    const x = 40 + (1 - k) * 300;
    g.globalAlpha = k;
    const mine = i === REPOS.length - 1;
    rr(x, y, 560, 36, 8); g.fillStyle = mine ? "#2a1645" : PANEL; g.fill(); g.strokeStyle = mine ? VIOLET : "#30363d"; g.lineWidth = mine ? 2 : 1; g.stroke();
    const [name, stars] = REPOS[i];
    const cut = name.toLowerCase().lastIndexOf("anything");
    g.font = "16px Mono"; g.textBaseline = "middle"; g.textAlign = "left";
    g.fillStyle = TEXT; g.fillText(name.slice(0, cut), x + 14, y + 18);
    const w0 = g.measureText(name.slice(0, cut)).width;
    g.fillStyle = PINK; g.fillText(name.slice(cut, cut + 8), x + 14 + w0, y + 18);
    const w1 = g.measureText(name.slice(cut, cut + 8)).width;
    g.fillStyle = TEXT; g.fillText(name.slice(cut + 8), x + 14 + w0 + w1, y + 18);
    text("★ " + stars, x + 546, y + 18, "14px Sans", "#d29922", "right");
    if (mine && t > times[i] + 0.35) text("(soon)", x + 470, y + 18, "italic 14px Sans", VIOLET, "right");
    g.globalAlpha = 1;
  }
  // header stays on top of the scrolling cards
  g.fillStyle = BG; g.fillRect(0, 0, W, 88);
  text("GitHub trending, 2026", 40, 36, "22px 'Sans Bold'", TEXT);
  text("★ every repo this week", 40, 62, "15px Sans", MUTED);
  // the counter
  const n = Math.min(999, Math.floor(Math.pow(Math.max(0, t - 0.2) * 3.2, 3)));
  rr(630, 100, 140, 110, 12); g.fillStyle = PANEL; g.fill();
  text("-anything", 700, 126, "16px 'Sans Bold'", PINK, "center");
  text("repos", 700, 146, "13px Sans", MUTED, "center");
  text(t > 3.0 ? "∞" : String(n), 700, 180, (t > 3.0 ? "44px Sym" : "34px 'Sans Bold'"), TEXT, "center");
}

// ---- scene 2: the thought ----
function scene2(t) { // 0..2.4
  g.fillStyle = BG; g.fillRect(0, 0, W, H);
  // a dev in a hoodie, lit by the laptop
  const cx = 200, cy = 300;
  const glow = g.createRadialGradient(cx, cy - 60, 10, cx, cy - 60, 170);
  glow.addColorStop(0, "rgba(167,112,255,0.35)"); glow.addColorStop(1, "rgba(167,112,255,0)");
  g.fillStyle = glow; g.fillRect(cx - 200, cy - 240, 400, 360);
  g.fillStyle = "#3b2a5c"; rr(cx - 62, cy - 78, 124, 120, 40); g.fill(); // hoodie
  g.fillStyle = "#2a1d43"; g.beginPath(); g.arc(cx, cy - 108, 46, Math.PI, 0); g.fill(); // hood
  g.fillStyle = "#f2c9a0"; g.beginPath(); g.arc(cx, cy - 108, 34, 0, Math.PI * 2); g.fill(); // face
  g.fillStyle = "#2a2f3a"; rr(cx - 80, cy + 20, 160, 12, 4); g.fill(); // laptop
  g.fillStyle = "#3a4150"; rr(cx - 62, cy - 46, 124, 70, 6); g.fill();
  g.fillStyle = "#1b1f2a"; rr(cx - 54, cy - 38, 108, 54, 4); g.fill();
  g.fillStyle = PINK; g.font = "11px Mono"; g.textAlign = "left"; g.fillText("-anything", cx - 46, cy - 22);
  g.fillStyle = MUTED; g.fillText("-anything", cx - 46, cy - 8); g.fillText("-anyth", cx - 46, cy + 6);
  const blink = (t % 1.3) > 1.2;
  const worried = t > 1.25;
  g.fillStyle = INK;
  if (blink) { g.fillRect(cx - 15, cy - 114, 9, 2); g.fillRect(cx + 6, cy - 114, 9, 2); }
  else { g.beginPath(); g.arc(cx - 11, cy - 114, worried ? 2.5 : 3.5, 0, 7); g.arc(cx + 11, cy - 114, worried ? 2.5 : 3.5, 0, 7); g.fill(); }
  g.strokeStyle = INK; g.lineWidth = 2.5; g.beginPath();
  if (!worried) g.arc(cx, cy - 102, 10, 0.25, Math.PI - 0.25);
  else { g.moveTo(cx - 8, cy - 94); g.quadraticCurveTo(cx, cy - 99, cx + 8, cy - 94); }
  g.stroke();
  if (worried) { // a sweat drop
    const dy = ((t - 1.25) * 30) % 20;
    g.fillStyle = "#7cc4ff"; g.beginPath(); g.moveTo(cx + 30, cy - 130 + dy); g.quadraticCurveTo(cx + 37, cy - 118 + dy, cx + 30, cy - 114 + dy); g.quadraticCurveTo(cx + 23, cy - 118 + dy, cx + 30, cy - 130 + dy); g.fill();
  }
  // bubbles
  const bub = (x, y, w, h, s, color, k) => {
    const pop = k <= 0 ? 0 : k >= 1 ? 1 : 1 + 0.12 * Math.sin(Math.PI * k) ;
    g.save(); g.translate(x + w / 2, y + h / 2); g.scale(pop * ease(k) || 0.001, pop * ease(k) || 0.001); g.translate(-(x + w / 2), -(y + h / 2));
    g.globalAlpha = ease(k); rr(x, y, w, h, 18); g.fillStyle = "#eceef4"; g.fill();
    g.beginPath(); g.arc(x + 20, y + h + 12, 8, 0, 7); g.arc(x + 4, y + h + 28, 5, 0, 7); g.fill();
    text(s, x + w / 2, y + h / 2, "24px 'Sans Bold'", color, "center"); g.globalAlpha = 1; g.restore();
  };
  bub(290, 70, 420, 70, "I should make an -anything.", INK, (t - 0.15) / 0.3);
  if (t > 1.2) bub(330, 170, 340, 70, "...anything what?", RED, (t - 1.2) / 0.25);
  if (t > 1.7) text("?", 250, 150 - 6 * ease((t - 1.7) / 0.3), "46px 'Sans Bold'", PINK, "center");
}

// ---- terminal helpers ----
function term(lines, title = "anything-anything") {
  g.fillStyle = BG; g.fillRect(0, 0, W, H);
  rr(30, 30, W - 60, H - 60, 14); g.fillStyle = PANEL; g.fill(); g.strokeStyle = "#30363d"; g.stroke();
  ["#ff5f56", "#ffbd2e", "#27c93f"].forEach((col, i) => { g.fillStyle = col; g.beginPath(); g.arc(56 + i * 20, 54, 6, 0, 7); g.fill(); });
  text(title, W / 2, 54, "13px Sans", MUTED, "center");
  let y = 96;
  for (const l of lines) {
    let x = 54;
    for (const [s, col, bold] of l) { g.font = (bold ? "17px 'Mono Bold'" : "17px Mono"); g.fillStyle = col; g.textAlign = "left"; g.textBaseline = "middle"; g.fillText(s, x, y); x += g.measureText(s).width; }
    y += 28;
  }
}
const typed = (s, t, cps = 28) => s.slice(0, Math.max(0, Math.floor(t * cps)));
const cursor = (t) => (Math.floor(t * 3) % 2 ? "" : "▌");

// ---- scene 3: new anything ----
function scene3(t) { // 0..2.6
  const cmd = "anything-anything new anything";
  const ty = typed(cmd, t - 0.1);
  const lines = [[["$ ", GREEN], [ty, TEXT], [ty.length < cmd.length ? cursor(t) : "", TEXT]]];
  if (t > 1.4) lines.push([["✦ anything-anything v0.1.0", VIOLET]]);
  if (t > 1.6) lines.push([["✗ ", RED, true], ["anything-anything already exists: you're running it.", TEXT]]);
  if (t > 1.9) lines.push([["  Pick what yours does to anything, like ", MUTED], ["translate", PINK]]);
  term(lines);
}

// ---- scene 4: slot machine montage ----
const WORDS = ["excuse", "monday", "vibes", "toaster", "meeting", "breakup", "pizza", "deadline", "cat", "standup", "taxes", "nap"];
const PICKS = [["secrets", "built by Claude", "12/12"], ["excuse", "3 commands", "9/9"], ["monday", "a web app", "5/5"], ["toaster", "any stack", "4/4"]];
const RAPID = ["cat", "taxes", "nap", "standup", "vibes", "deadline", "pizza"];
function scene4(t) { // 0..4.0, one pick per second
  const lines = [];
  const step = Math.min(3, Math.floor(t / 1.0));
  for (let i = 0; i <= step; i++) {
    const lt = t - i * 1.0;
    const [word, what, ex] = PICKS[i];
    const spinning = lt < 0.45;
    const shownWord = spinning ? WORDS[Math.floor(lt * 40 + i * 5) % WORDS.length] : word;
    lines.push([["$ ", GREEN], ["anything-anything new ", TEXT], [shownWord, spinning ? MUTED : PINK, !spinning]]);
    if (lt > 0.55) lines.push([["✓ ", GREEN, true], [word + "-anything", TEXT, true], [" works: " + what + ", " + ex + " examples", MUTED]]);
    else lines.push([["", TEXT]]);
  }
  if (t > 4.0) {
    const n = Math.min(RAPID.length, Math.floor((t - 4.0) / 0.14) + 1);
    lines.push([["$ ", GREEN], ["new ", MUTED], ...RAPID.slice(0, n).map((w) => [w + " ✓  ", PINK])]);
  }
  if (t > 5.1) lines.push([["  (yes, every one passes its own tests)", VIOLET]]);
  term(lines);
}

// ---- scene 5: infinite zoom into the logo, then the wordmark ----
function tile(cx, cy, size, angle, fill) {
  g.save(); g.translate(cx, cy); g.rotate(angle); rr(-size / 2, -size / 2, size, size, size * 0.24); g.fillStyle = fill; g.fill(); g.restore();
}
const LAYER = ["#7c3aed", INK, "#9d36d6", "#1d0f33", "#be2fa8", "#2a1044", "#db2777", "#331148"];
function scene5(t) { // 0..3.4
  g.fillStyle = BG; g.fillRect(0, 0, W, H);
  const z = 4 * ease(t / 2.0) + t * 0.6; // how far in we've zoomed
  const base = 300, shrink = 0.78, turn = (9 * Math.PI) / 180;
  const cx = W / 2, cy = t > 2.0 ? H / 2 - 40 * ease((t - 2.0) / 0.5) : H / 2;
  for (let i = Math.floor(z) - 8; i < Math.floor(z) + 9; i++) {
    const size = base * Math.pow(shrink, i - z);
    if (size > 2600 || size < 6) continue;
    tile(cx, cy, size, turn * i, LAYER[((i % 8) + 8) % 8]);
  }
  // spark
  const s = 18 + 4 * Math.sin(t * 8);
  g.fillStyle = "#fff"; g.beginPath();
  for (let i = 0; i < 8; i++) { const a = (i * Math.PI) / 4 - Math.PI / 2; const r = i % 2 ? s * 0.28 : s; g.lineTo(cx + r * Math.cos(a), cy + r * Math.sin(a)); }
  g.fill();
  if (t > 2.0) {
    const k = ease((t - 2.0) / 0.5);
    g.globalAlpha = k * 0.85; g.fillStyle = BG; g.fillRect(0, H - 150, W, 150); g.globalAlpha = k;
    g.font = "54px 'Sans Bold'"; g.textBaseline = "middle"; g.textAlign = "left";
    const a = "anything", b = "-anything";
    const wa = g.measureText(a).width, wb = g.measureText(b).width;
    const x0 = (W - wa - wb) / 2;
    g.fillStyle = TEXT; g.fillText(a, x0, H - 108); g.fillStyle = VIOLET; g.fillText(b, x0 + wa, H - 108);
    text("Literally anything.", W / 2, H - 62, "26px 'Sans Italic'", PINK, "center");
    text("npm i -g @el4cteo/anything-anything", W / 2, H - 28, "15px Mono", MUTED, "center");
    g.globalAlpha = 1;
  }
}

const gif = GIFEncoder();
let prev = null;
function emit(data, delay) {
  // pixels that didn't change since the last frame become transparent: far smaller file, same picture
  const palette = quantize(data, 255);
  const index = applyPalette(data, palette);
  if (prev) for (let i = 0, p = 0; i < index.length; i++, p += 4) {
    if (data[p] === prev[p] && data[p + 1] === prev[p + 1] && data[p + 2] === prev[p + 2]) index[i] = 255;
  }
  while (palette.length < 256) palette.push([0, 0, 0]);
  gif.writeFrame(index, W, H, prev ? { palette, delay, transparent: true, transparentIndex: 255, dispose: 1 } : { palette, delay, repeat: 0 });
  prev = Uint8ClampedArray.from(data);
}
const SCENES = [[scene1, 4.4], [scene2, 2.8], [scene3, 3.0], [scene4, 6.0], [scene5, 4.0]];
const FADE = 0.2;

let frames = 0;
for (const [scene, len] of SCENES) {
  const n = Math.round(len * FPS);
  for (let f = 0; f < n; f++) {
    const t = f / FPS;
    scene(t);
    const fade = Math.min(1, t / FADE, (len - t) / FADE);
    if (fade < 1 && !(scene === SCENES[0][0] && t < FADE)) { g.globalAlpha = 1 - Math.max(0, fade); g.fillStyle = BG; g.fillRect(0, 0, W, H); g.globalAlpha = 1; }
    emit(g.getImageData(0, 0, W, H).data, 1000 / FPS);
    frames++;
  }
  // hold the last frame of the last scene a moment longer
}
scene5(4.0);
emit(g.getImageData(0, 0, W, H).data, 1800);
gif.finish();
writeFileSync(process.argv[2] ?? "trend.gif", gif.bytes());
// stills for checking
for (const [i, [scene, at]] of [[scene1, 4.2], [scene2, 2.5], [scene3, 2.7], [scene4, 5.6], [scene5, 3.8]].entries()) {
  scene(at); writeFileSync("still-" + (i + 1) + ".png", c.toBuffer("image/png"));
}
console.log("frames", frames + 1);
