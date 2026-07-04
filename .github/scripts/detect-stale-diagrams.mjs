// Compare a git diff against the diagram catalog and report which diagrams
// are likely stale. Output goes to stdout (one slug per line) and to
// $GITHUB_OUTPUT when running inside GitHub Actions.
//
// Usage:
//   node detect-stale-diagrams.mjs                 # diff origin/main..HEAD
//   node detect-stale-diagrams.mjs <base> <head>   # custom range
//
// The script always exits 0. It is informational, not gate-able.

import { readFile, appendFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import yaml from "js-yaml";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..", "..");
const CATALOG_PATH = path.join(
  REPO_ROOT,
  ".github",
  "skills",
  "diagrams",
  "catalog.yml",
);

function diffFiles(base, head) {
  try {
    const out = execFileSync(
      "git",
      ["diff", "--name-only", `${base}...${head}`],
      { cwd: REPO_ROOT, encoding: "utf8" },
    );
    return out.split(/\r?\n/).filter(Boolean);
  } catch (err) {
    console.error(`git diff failed for ${base}...${head}: ${err.message}`);
    return [];
  }
}

// Minimal glob-to-regex for the patterns we actually use in catalog.yml:
//   foo/bar.rs        -> exact match
//   foo/**            -> any path under foo/
//   foo/**/*.ts       -> any .ts file under foo/
function globToRegex(glob) {
  let re = "^";
  let i = 0;
  while (i < glob.length) {
    const c = glob[i];
    if (c === "*" && glob[i + 1] === "*") {
      // ** matches any number of path segments
      re += ".*";
      i += 2;
      if (glob[i] === "/") i++;
    } else if (c === "*") {
      re += "[^/]*";
      i++;
    } else if (c === "?") {
      re += "[^/]";
      i++;
    } else if (".+()|[]{}\\^$".includes(c)) {
      re += "\\" + c;
      i++;
    } else {
      re += c;
      i++;
    }
  }
  re += "$";
  return new RegExp(re);
}

function matchesAny(file, globs) {
  return globs.some((g) => globToRegex(g).test(file));
}

async function loadCatalog() {
  const raw = await readFile(CATALOG_PATH, "utf8");
  const parsed = yaml.load(raw);
  return parsed?.diagrams ?? [];
}

async function main() {
  const base = process.argv[2] ?? "origin/main";
  const head = process.argv[3] ?? "HEAD";
  const changed = diffFiles(base, head);
  if (changed.length === 0) {
    console.error(`no changed files between ${base} and ${head}`);
    return;
  }

  const diagrams = await loadCatalog();
  const stale = [];
  for (const entry of diagrams) {
    const { slug, tracks } = entry;
    if (!slug || !Array.isArray(tracks) || tracks.length === 0) continue;
    const hits = changed.filter((f) => matchesAny(f, tracks));
    if (hits.length > 0) {
      stale.push({ slug, hits });
    }
  }

  for (const entry of stale) {
    console.log(entry.slug);
  }

  if (process.env.GITHUB_OUTPUT) {
    const slugs = stale.map((e) => e.slug).join(",");
    const json = JSON.stringify(
      stale.reduce((acc, e) => {
        acc[e.slug] = e.hits;
        return acc;
      }, {}),
    );
    await appendFile(
      process.env.GITHUB_OUTPUT,
      `stale_slugs=${slugs}\nstale_count=${stale.length}\nstale_detail<<EOF\n${json}\nEOF\n`,
    );
  }

  console.error(
    `detected ${stale.length} stale diagram(s) from ${changed.length} changed file(s)`,
  );
}

main().catch((err) => {
  console.error("detect-stale crashed:", err);
  // Still exit 0 — informational only.
  process.exit(0);
});
