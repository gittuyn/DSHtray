import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { Resvg } from "@resvg/resvg-js";

const icons = [
  {
    name: "tray-deepseek-blue.png",
    url: "https://cdn.simpleicons.org/deepseek/2563EB",
  },
  {
    name: "tray-deepseek-black.png",
    url: "https://cdn.simpleicons.org/deepseek/000000",
  },
];

const outputDirectory = resolve("src-tauri/icons");
await mkdir(outputDirectory, { recursive: true });

for (const icon of icons) {
  const response = await fetch(icon.url);
  if (!response.ok) {
    throw new Error(`Failed to download ${icon.url}: HTTP ${response.status}`);
  }
  const svg = await response.text();
  const png = new Resvg(svg, {
    fitTo: { mode: "width", value: 32 },
    background: "rgba(0, 0, 0, 0)",
  }).render().asPng();
  const output = resolve(outputDirectory, icon.name);
  await mkdir(dirname(output), { recursive: true });
  await writeFile(output, png);
  console.log(`Wrote ${output}`);
}
