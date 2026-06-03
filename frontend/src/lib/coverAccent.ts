const cache = new Map<string, string | null>();

const SAMPLE_SIZE = 16;
const NEAR_BLACK = 28;
const NEAR_WHITE = 228;
const GREY_SPREAD = 22;
const MIN_SATURATION = 0.18;
const HUE_BUCKETS = 12;

interface Bucket {
  count: number;
  weight: number;
  hueSum: number;
  satSum: number;
  litSum: number;
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const lightness = (max + min) / 2;
  const delta = max - min;
  if (delta === 0) return [0, 0, lightness];
  const saturation = delta / (1 - Math.abs(2 * lightness - 1));
  let hue: number;
  if (max === rn) hue = ((gn - bn) / delta) % 6;
  else if (max === gn) hue = (bn - rn) / delta + 2;
  else hue = (rn - gn) / delta + 4;
  hue *= 60;
  if (hue < 0) hue += 360;
  return [hue, saturation, lightness];
}

function isNeutral(r: number, g: number, b: number): boolean {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  if (max <= NEAR_BLACK) return true;
  if (min >= NEAR_WHITE) return true;
  return max - min <= GREY_SPREAD;
}

function dominantFromPixels(data: Uint8ClampedArray): string | null {
  const buckets = new Map<number, Bucket>();
  for (let i = 0; i < data.length; i += 4) {
    const r = data[i];
    const g = data[i + 1];
    const b = data[i + 2];
    const a = data[i + 3];
    if (a < 128) continue;
    if (isNeutral(r, g, b)) continue;
    const [hue, sat, lit] = rgbToHsl(r, g, b);
    if (sat < MIN_SATURATION) continue;
    const key = Math.floor(hue / (360 / HUE_BUCKETS));
    const weight = sat * sat;
    const bucket = buckets.get(key) ?? { count: 0, weight: 0, hueSum: 0, satSum: 0, litSum: 0 };
    bucket.count += 1;
    bucket.weight += weight;
    bucket.hueSum += hue * weight;
    bucket.satSum += sat * weight;
    bucket.litSum += lit * weight;
    buckets.set(key, bucket);
  }
  let best: Bucket | null = null;
  for (const bucket of buckets.values()) {
    if (!best || bucket.weight > best.weight) best = bucket;
  }
  if (!best || best.weight === 0) return null;
  const hue = Math.round(best.hueSum / best.weight);
  const sat = Math.round(Math.min(1, Math.max(0.42, best.satSum / best.weight)) * 100);
  const lit = Math.round(Math.min(0.66, Math.max(0.46, best.litSum / best.weight)) * 100);
  return `hsl(${hue}, ${sat}%, ${lit}%)`;
}

export function coverAccent(url: string): Promise<string | null> {
  const cached = cache.get(url);
  if (cached !== undefined) return Promise.resolve(cached);
  return new Promise((resolve) => {
    const finish = (value: string | null): void => {
      cache.set(url, value);
      resolve(value);
    };
    try {
      const img = new Image();
      img.crossOrigin = "anonymous";
      img.onload = () => {
        try {
          const canvas = document.createElement("canvas");
          canvas.width = SAMPLE_SIZE;
          canvas.height = SAMPLE_SIZE;
          const ctx = canvas.getContext("2d", { willReadFrequently: true });
          if (!ctx) {
            finish(null);
            return;
          }
          ctx.drawImage(img, 0, 0, SAMPLE_SIZE, SAMPLE_SIZE);
          const { data } = ctx.getImageData(0, 0, SAMPLE_SIZE, SAMPLE_SIZE);
          finish(dominantFromPixels(data));
        } catch {
          finish(null);
        }
      };
      img.onerror = () => finish(null);
      img.src = url;
    } catch {
      finish(null);
    }
  });
}
