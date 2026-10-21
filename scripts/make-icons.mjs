// 生成 UACSkip 的应用图标：盾牌 + 对勾（纯 Node 实现，无第三方依赖）
// 用法: node scripts/make-icons.mjs
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "..", "src-tauri", "icons");

/* ---------------- PNG 编码 ---------------- */
const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

function encodePng(size, rgba) {
  const stride = size * 4;
  const raw = Buffer.alloc((stride + 1) * size);
  for (let y = 0; y < size; y++) {
    raw[y * (stride + 1)] = 0; // filter: none
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/* ---------------- 形状定义（坐标归一化到 [-1,1]） ---------------- */
function roundedRect(x, y, hw, hh, r) {
  const qx = Math.abs(x) - (hw - r);
  const qy = Math.abs(y) - (hh - r);
  const mx = Math.max(qx, 0);
  const my = Math.max(qy, 0);
  return Math.hypot(mx, my) + Math.min(Math.max(qx, qy), 0) - r;
}

const SHIELD = { top: -0.74, shoulder: -0.22, tip: 0.86, r: 0.34, w: 0.52 };

function shieldHalfWidth(y) {
  const { top, shoulder, tip, r, w } = SHIELD;
  if (y < top || y > tip) return -1;
  if (y <= shoulder) {
    const cy = top + r;
    if (y < cy) {
      const dy = cy - y;
      const dx = Math.sqrt(Math.max(0, r * r - dy * dy));
      return w - (r - dx);
    }
    return w;
  }
  const t = (y - shoulder) / (tip - shoulder);
  return w * Math.pow(1 - Math.pow(t, 2.3), 0.6);
}

function inShield(x, y) {
  const hw = shieldHalfWidth(y);
  return hw > 0 && Math.abs(x) <= hw;
}

function segDist(px, py, ax, ay, bx, by) {
  const dx = bx - ax;
  const dy = by - ay;
  const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)));
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
}

function inCheck(x, y) {
  const d = Math.min(
    segDist(x, y, -0.25, 0.03, -0.06, 0.22),
    segDist(x, y, -0.06, 0.22, 0.28, -0.2),
  );
  return d <= 0.088;
}

/* ---------------- 配色 ---------------- */
function lerp(a, b, t) {
  return a + (b - a) * t;
}

function bgColor(x, y) {
  const t = (y + 1) / 2;
  const base = [
    lerp(22, 9, t),
    lerp(30, 14, t),
    lerp(52, 26, t),
  ];
  // 左上角柔光
  const glow = Math.max(0, 1 - Math.hypot(x + 0.45, y + 0.55) / 1.15) ** 2 * 26;
  return [base[0] + glow * 0.5, base[1] + glow * 0.8, base[2] + glow];
}

function shieldColor(x, y) {
  const t = Math.max(0, Math.min(1, (y + 0.8) / 1.7));
  const r = lerp(56, 99, t);
  const g = lerp(189, 102, t);
  const b = lerp(248, 241, t);
  // 顶部高光
  const hi = Math.max(0, 1 - Math.hypot(x + 0.15, y + 0.55) / 0.9) ** 2 * 40;
  return [Math.min(255, r + hi), Math.min(255, g + hi * 0.7), Math.min(255, b + hi * 0.3)];
}

/* ---------------- 渲染 ---------------- */
function render(size) {
  const SS = 4;
  const samples = SS * SS;
  const half = size / 2;
  const radius = 0.235;
  const rgba = Buffer.alloc(size * size * 4);
  const drawCheck = size >= 24;

  for (let py = 0; py < size; py++) {
    for (let px = 0; px < size; px++) {
      let r = 0;
      let g = 0;
      let b = 0;
      let covered = 0;
      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const x = (px + (sx + 0.5) / SS - half) / half;
          const y = (py + (sy + 0.5) / SS - half) / half;
          if (roundedRect(x, y, 0.995, 0.995, radius) > 0) continue;
          const shield = inShield(x, y);
          const cut = drawCheck && inCheck(x, y);
          const color = shield && !cut ? shieldColor(x, y) : bgColor(x, y);
          r += color[0];
          g += color[1];
          b += color[2];
          covered++;
        }
      }
      const i = (py * size + px) * 4;
      if (covered > 0) {
        rgba[i] = Math.round(r / covered);
        rgba[i + 1] = Math.round(g / covered);
        rgba[i + 2] = Math.round(b / covered);
      }
      rgba[i + 3] = Math.round((covered / samples) * 255);
    }
  }
  return encodePng(size, rgba);
}

/* ---------------- ICO 打包 ---------------- */
function buildIco(entries) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(entries.length, 4);

  const dir = Buffer.alloc(16 * entries.length);
  let offset = 6 + dir.length;
  entries.forEach((entry, index) => {
    const at = index * 16;
    dir[at] = entry.size >= 256 ? 0 : entry.size;
    dir[at + 1] = entry.size >= 256 ? 0 : entry.size;
    dir[at + 2] = 0;
    dir[at + 3] = 0;
    dir.writeUInt16LE(1, at + 4);
    dir.writeUInt16LE(32, at + 6);
    dir.writeUInt32LE(entry.png.length, at + 8);
    dir.writeUInt32LE(offset, at + 12);
    offset += entry.png.length;
  });

  return Buffer.concat([header, dir, ...entries.map((entry) => entry.png)]);
}

/* ---------------- 输出 ---------------- */
mkdirSync(OUT, { recursive: true });

const pngSizes = [16, 24, 32, 48, 64, 128, 256, 512];
const cache = new Map();
for (const size of pngSizes) cache.set(size, render(size));

const files = {
  "32x32.png": 32,
  "128x128.png": 128,
  "128x128@2x.png": 256,
  "icon.png": 512,
  "16x16.png": 16,
  "24x24.png": 24,
  "48x48.png": 48,
  "64x64.png": 64,
};
for (const [name, size] of Object.entries(files)) {
  writeFileSync(join(OUT, name), cache.get(size));
}

const icoSizes = [16, 24, 32, 48, 64, 128, 256];
writeFileSync(
  join(OUT, "icon.ico"),
  buildIco(icoSizes.map((size) => ({ size, png: cache.get(size) }))),
);

console.log(`icons written to ${OUT}`);
