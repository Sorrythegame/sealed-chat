import { cp, mkdir, rm, writeFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);
const output = new URL("../out/", import.meta.url);
const workerUrl = new URL("../dist/server/index.js", import.meta.url);

await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });

const { default: worker } = await import(
  `${workerUrl.href}?export=${Date.now()}`
);
const response = await worker.fetch(
  new Request("http://localhost/", {
    headers: { accept: "text/html" },
  }),
  {
    ASSETS: {
      fetch: async () => new Response("Not found", { status: 404 }),
    },
  },
  {
    waitUntil() {},
    passThroughOnException() {},
  },
);

if (!response.ok) {
  throw new Error(`Static export failed with HTTP ${response.status}`);
}

await writeFile(new URL("index.html", output), await response.text(), "utf8");
await cp(new URL("dist/client/", root), output, { recursive: true });
await cp(new URL("public/", root), output, { recursive: true });

console.log(`Static site exported to ${output.pathname}`);
