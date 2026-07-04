// Render every diagram listed in .github/skills/diagrams/catalog.yml to
// docs/<slug>.svg. Skips slugs whose .excalidraw source file is missing and
// reports them as warnings rather than failures.

import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import yaml from "js-yaml";
import { renderExcalidrawToSvg } from "./render-excalidraw.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..", "..");
const CATALOG_PATH = path.join(
  REPO_ROOT,
  ".github",
  "skills",
  "diagrams",
  "catalog.yml",
);
const DOCS_DIR = path.join(REPO_ROOT, "docs");

async function loadCatalog() {
  const raw = await readFile(CATALOG_PATH, "utf8");
  const parsed = yaml.load(raw);
  if (!parsed || !Array.isArray(parsed.diagrams)) {
    throw new Error(`${CATALOG_PATH} is missing a top-level "diagrams" list`);
  }
  return parsed.diagrams;
}

async function main() {
  const diagrams = await loadCatalog();
  let rendered = 0;
  let skipped = 0;
  let failed = 0;

  for (const entry of diagrams) {
    const { slug } = entry;
    if (!slug) {
      console.warn("catalog entry missing slug; skipping:", entry);
      continue;
    }
    const input = path.join(DOCS_DIR, `${slug}.excalidraw`);
    const output = path.join(DOCS_DIR, `${slug}.svg`);
    if (!existsSync(input)) {
      console.warn(`skip ${slug}: source not found (${input})`);
      skipped++;
      continue;
    }
    try {
      await renderExcalidrawToSvg(input, output);
      console.log(`rendered ${slug}`);
      rendered++;
    } catch (err) {
      console.error(`failed ${slug}: ${err.message}`);
      failed++;
    }
  }

  console.log(
    `summary: rendered=${rendered} skipped=${skipped} failed=${failed}`,
  );
  // Exit 0 even when individual renders fail — the workflow will surface
  // failures via comments. A non-zero exit would block PRs on diagram
  // rendering issues, which we explicitly do not want.
  process.exit(0);
}

main().catch((err) => {
  console.error("render-all crashed:", err);
  process.exit(1);
});
