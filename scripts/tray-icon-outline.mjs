const PATH_TAG_PATTERN = /<path\b[^>]*>/g;
const SVG_TAG_PATTERN = /<svg\b[^>]*>/i;
const VIEW_BOX_PATTERN = /\sviewBox="[^"]*"/i;
const TRAILING_TAG_PATTERN = /\s*\/?>(\s*)$/;

export const HAIRLINE_OUTLINE_WIDTH = "2.25";
export const VIEW_BOX_PADDING = 3;

function setAttribute(tag, name, value) {
  const attributePattern = new RegExp(`\\s${name}="[^"]*"`, "i");
  if (attributePattern.test(tag)) {
    return tag.replace(attributePattern, ` ${name}="${value}"`);
  }

  return tag.replace(
    TRAILING_TAG_PATTERN,
    ` ${name}="${value}"$&`,
  );
}

export function addHairlineOutline(svg) {
  let pathCount = 0;
  const outlined = svg.replace(PATH_TAG_PATTERN, (tag) => {
    pathCount += 1;
    return [
      ["stroke", "#FFFFFF"],
      ["stroke-width", HAIRLINE_OUTLINE_WIDTH],
      ["stroke-linejoin", "round"],
      ["stroke-linecap", "round"],
      ["paint-order", "stroke fill"],
    ].reduce((current, [name, value]) => setAttribute(current, name, value), tag);
  });

  if (pathCount === 0) {
    throw new Error("DeepSeek SVG did not contain a path element");
  }

  return outlined;
}

export function padSvgViewBox(svg, padding = VIEW_BOX_PADDING) {
  if (!Number.isFinite(padding) || padding <= 0) {
    throw new Error("SVG viewBox padding must be a positive number");
  }

  let foundSvg = false;
  const padded = svg.replace(SVG_TAG_PATTERN, (tag) => {
    foundSvg = true;
    const viewBox = tag.match(VIEW_BOX_PATTERN)?.[0];
    if (!viewBox) {
      throw new Error("DeepSeek SVG did not contain a viewBox attribute");
    }
    const values = viewBox
      .slice(viewBox.indexOf('"') + 1, viewBox.lastIndexOf('"'))
      .trim()
      .split(/[\s,]+/)
      .map(Number);
    if (values.length !== 4 || values.some((value) => !Number.isFinite(value))) {
      throw new Error("DeepSeek SVG viewBox must contain four finite numbers");
    }

    const [x, y, width, height] = values;
    const paddedViewBox = [x - padding, y - padding, width + 2 * padding, height + 2 * padding]
      .map((value) => Number(value.toFixed(6)))
      .join(" ");
    return tag.replace(VIEW_BOX_PATTERN, ` viewBox="${paddedViewBox}"`);
  });

  if (!foundSvg) {
    throw new Error("DeepSeek SVG did not contain an svg element");
  }
  return padded;
}
