// Render a single .excalidraw file to SVG using a headless Chromium and the
// official @excalidraw/excalidraw package. No external network access at
// runtime — the Excalidraw UMD bundle is served from local node_modules over
// a loopback HTTP server.
//
// Usage:
//   node render-excalidraw.mjs <input.excalidraw> <output.svg>

import http from "node:http";
import { readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const NODE_MODULES = path.join(__dirname, "node_modules");
const EXCALIDRAW_DIST = path.join(
  NODE_MODULES,
  "@excalidraw",
  "excalidraw",
  "dist",
);

// Files we expose to the headless browser. Anything not in this allowlist is
// rejected by the loopback server. The excalidraw UMD bundles its own
// react/react-dom inside dist/, so we serve them directly from there.
const ASSET_ALLOWLIST = new Set([
  "/excalidraw.production.min.js",
  "/react.production.min.js",
  "/react-dom.production.min.js",
  "/harness.html",
]);

const HARNESS_HTML = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>render harness</title>
  <script src="/react.production.min.js"></script>
  <script src="/react-dom.production.min.js"></script>
  <script src="/excalidraw.production.min.js"></script>
  <style>html, body { margin: 0; padding: 0; }</style>
</head>
<body>
  <script>
    window.renderToSvg = async function(sceneJson) {
      const { exportToSvg } = window.ExcalidrawLib;
      const svg = await exportToSvg({
        elements: sceneJson.elements || [],
        appState: Object.assign(
          { exportBackground: true, viewBackgroundColor: "#ffffff" },
          sceneJson.appState || {},
        ),
        files: sceneJson.files || null,
      });
      return svg.outerHTML;
    };
    window.__harnessReady = true;
  </script>
</body>
</html>
`;

function contentTypeFor(urlPath) {
  if (urlPath.endsWith(".html")) return "text/html; charset=utf-8";
  if (urlPath.endsWith(".js")) return "application/javascript; charset=utf-8";
  return "application/octet-stream";
}

// Resolve an asset path inside an allowlisted location. Returns null when the
// file isn't on the allowlist or can't be located in node_modules.
function resolveAsset(urlPath) {
  if (!ASSET_ALLOWLIST.has(urlPath)) return null;
  if (urlPath === "/harness.html") return { kind: "inline", body: HARNESS_HTML };

  // React UMDs come from node_modules/react{,-dom}/umd/ (peer deps of
  // @excalidraw/excalidraw). Excalidraw's own UMD lives in its dist/.
  if (urlPath === "/react.production.min.js") {
    const p = path.join(NODE_MODULES, "react", "umd", "react.production.min.js");
    return existsSync(p) ? { kind: "file", path: p } : null;
  }
  if (urlPath === "/react-dom.production.min.js") {
    const p = path.join(
      NODE_MODULES,
      "react-dom",
      "umd",
      "react-dom.production.min.js",
    );
    return existsSync(p) ? { kind: "file", path: p } : null;
  }
  if (urlPath === "/excalidraw.production.min.js") {
    const p = path.join(EXCALIDRAW_DIST, "excalidraw.production.min.js");
    return existsSync(p) ? { kind: "file", path: p } : null;
  }
  return null;
}

async function startServer() {
  return new Promise((resolve, reject) => {
    const server = http.createServer(async (req, res) => {
      try {
        const asset = resolveAsset(req.url || "");
        if (!asset) {
          res.statusCode = 404;
          res.end("not found");
          return;
        }
        res.setHeader("Content-Type", contentTypeFor(req.url));
        if (asset.kind === "inline") {
          res.end(asset.body);
        } else {
          res.end(await readFile(asset.path));
        }
      } catch (err) {
        res.statusCode = 500;
        res.end(String(err));
      }
    });
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => resolve(server));
  });
}

export async function renderExcalidrawToSvg(inputPath, outputPath) {
  if (!existsSync(EXCALIDRAW_DIST)) {
    throw new Error(
      `Cannot find @excalidraw/excalidraw under ${NODE_MODULES}. Run \`npm install\` in .github/scripts first.`,
    );
  }

  const raw = await readFile(inputPath, "utf8");
  const scene = JSON.parse(raw);

  const server = await startServer();
  const port = server.address().port;
  const baseUrl = `http://127.0.0.1:${port}`;

  const browser = await puppeteer.launch({
    headless: true,
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });
  try {
    const page = await browser.newPage();
    await page.goto(`${baseUrl}/harness.html`, { waitUntil: "networkidle0" });
    await page.waitForFunction("window.__harnessReady === true", {
      timeout: 15_000,
    });
    const svg = await page.evaluate(
      async (sceneJson) => await window.renderToSvg(sceneJson),
      scene,
    );
    if (!svg || !svg.includes("<svg")) {
      throw new Error("renderer returned empty or invalid SVG");
    }
    await writeFile(outputPath, svg, "utf8");
  } finally {
    await browser.close();
    server.close();
  }
}

async function main() {
  const [, , input, output] = process.argv;
  if (!input || !output) {
    console.error("usage: node render-excalidraw.mjs <input.excalidraw> <output.svg>");
    process.exit(2);
  }
  try {
    await renderExcalidrawToSvg(input, output);
    console.log(`rendered ${input} -> ${output}`);
  } catch (err) {
    console.error(`render failed: ${err.message}`);
    process.exit(1);
  }
}

// Detect "am I being run as a script" robustly across platforms. On Windows
// process.argv[1] is a drive-letter path while import.meta.url is a file:///
// URL; comparing them as strings only works after normalizing through
// pathToFileURL.
const isMain = (() => {
  try {
    const argvUrl = new URL(`file://${path.resolve(process.argv[1] || "").replace(/\\/g, "/")}`);
    return new URL(import.meta.url).pathname.toLowerCase() ===
      argvUrl.pathname.toLowerCase();
  } catch {
    return false;
  }
})();

if (isMain) {
  await main();
}
