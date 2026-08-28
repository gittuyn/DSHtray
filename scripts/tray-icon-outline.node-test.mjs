import test from "node:test";
import assert from "node:assert/strict";
import { inflateSync } from "node:zlib";
import { readFile } from "node:fs/promises";

import { addHairlineOutline, padSvgViewBox } from "./tray-icon-outline.mjs";

function decodeRgbaPng(buffer) {
  assert.deepEqual([...buffer.subarray(0, 8)], [
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
  ]);
  let offset = 8;
  let width;
  let height;
  let idat = Buffer.alloc(0);
  while (offset < buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString("ascii", offset + 4, offset + 8);
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      assert.equal(data[8], 8);
      assert.equal(data[9], 6);
      assert.equal(data[12], 0);
    } else if (type === "IDAT") {
      idat = Buffer.concat([idat, data]);
    } else if (type === "IEND") {
      break;
    }
    offset += 12 + length;
  }
  const stride = width * 4;
  const encoded = inflateSync(idat);
  const pixels = Buffer.alloc(height * stride);
  let sourceOffset = 0;
  for (let y = 0; y < height; y += 1) {
    const filter = encoded[sourceOffset++];
    const rowOffset = y * stride;
    for (let x = 0; x < stride; x += 1) {
      const raw = encoded[sourceOffset++];
      const left = x >= 4 ? pixels[rowOffset + x - 4] : 0;
      const above = y > 0 ? pixels[rowOffset - stride + x] : 0;
      const upperLeft = y > 0 && x >= 4 ? pixels[rowOffset - stride + x - 4] : 0;
      let value = raw;
      if (filter === 1) value = raw + left;
      if (filter === 2) value = raw + above;
      if (filter === 3) value = raw + Math.floor((left + above) / 2);
      if (filter === 4) {
        const estimate = left + above - upperLeft;
        const distanceLeft = Math.abs(estimate - left);
        const distanceAbove = Math.abs(estimate - above);
        const distanceUpperLeft = Math.abs(estimate - upperLeft);
        value = raw + (distanceLeft <= distanceAbove && distanceLeft <= distanceUpperLeft
          ? left
          : distanceAbove <= distanceUpperLeft ? above : upperLeft);
      }
      pixels[rowOffset + x] = value & 0xff;
    }
  }
  return { width, height, pixels };
}

test("formal tray icons keep every stroked edge inside transparent padding", async () => {
  for (const name of [
    "tray-deepseek-blue.png",
    "tray-deepseek-black.png",
    "tray-deepseek-red.png",
    "tray-deepseek-yellow.png",
  ]) {
    const image = decodeRgbaPng(await readFile(new URL(`../src-tauri/icons/${name}`, import.meta.url)));
    assert.deepEqual([image.width, image.height], [64, 64]);
    const alphaAt = (x, y) => image.pixels[(y * image.width + x) * 4 + 3];
    for (let index = 0; index < 64; index += 1) {
      assert.equal(alphaAt(index, 0), 0, `${name}: top edge`);
      assert.equal(alphaAt(index, 63), 0, `${name}: bottom edge`);
      assert.equal(alphaAt(0, index), 0, `${name}: left edge`);
      assert.equal(alphaAt(63, index), 0, `${name}: right edge`);
    }
  }
});

test("pads the viewBox so a stroked icon cannot touch the canvas edge", () => {
  const source =
    '<svg viewBox="0 0 24 24"><path fill="#2563EB" d="M0 0h24v24H0z" /></svg>';

  const padded = padSvgViewBox(source);

  assert.match(padded, /viewBox="-3 -3 30 30"/);
});

test("adds a 6-pixel white outline to each icon path", () => {
  const source =
    '<svg viewBox="0 0 24 24"><path fill="#2563EB" d="M1 1Z" /></svg>';

  const outlined = addHairlineOutline(source);

  assert.match(outlined, /<path\b[^>]*stroke="#FFFFFF"/);
  assert.match(outlined, /stroke-width="2\.25"/);
  assert.match(outlined, /stroke-linejoin="round"/);
  assert.match(outlined, /paint-order="stroke fill"/);
  assert.match(outlined, /fill="#2563EB"/);
});

test("renders the tray source at 64 by 64 pixels", async () => {
  const generator = await readFile(new URL("./fetch-tray-icons.mjs", import.meta.url), "utf8");

  assert.match(generator, /fitTo:\s*\{\s*mode: "width",\s*value: 64\s*\}/s);
});
