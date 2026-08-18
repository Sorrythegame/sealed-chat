import assert from "node:assert/strict";
import test from "node:test";

async function render() {
  const workerUrl = new URL("../dist/server/index.js", import.meta.url);
  workerUrl.searchParams.set("test", `${process.pid}-${Date.now()}`);
  const { default: worker } = await import(workerUrl.href);

  return worker.fetch(
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
}

test("renders the production download page", async () => {
  const response = await render();
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type") ?? "", /^text\/html\b/i);

  const html = await response.text();
  assert.match(html, /<html[^>]*lang="zh-CN"/i);
  assert.match(html, /<title>技术交流 - 安全、克制的沟通工具<\/title>/i);
  assert.match(html, /让重要沟通/);
  assert.match(html, /technology-communication_0\.1\.1_x64-setup\.exe/);
  assert.match(html, /technology-communication_0\.1\.1_aarch64\.dmg/);
  assert.match(html, /technology-communication_0\.1\.1_x64\.dmg/);
  assert.match(html, /Apple 芯片版/);
  assert.match(html, /Intel 芯片版/);
  assert.doesNotMatch(html, /macOS 版准备中/);
  assert.match(html, /端到端加密/);
  assert.doesNotMatch(html, /Your site is taking shape|Sealed Chat/i);
});
