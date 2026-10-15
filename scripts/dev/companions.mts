#!/usr/bin/env node
// 先运行 make wasm；此入口只读提供已有文件，不触发构建或访问宠物数据。
import { createServer } from "node:http";
import { readFile, realpath, stat } from "node:fs/promises";
import { dirname, extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = await realpath(resolve(dirname(fileURLToPath(import.meta.url)), "../.."));
const address = "127.0.0.1";
const port = 8841;
const types = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".md": "text/plain; charset=utf-8",
  ".wasm": "application/wasm",
  ".woff2": "font/woff2",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".webp": "image/webp",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
};

function reply(response, status, message) {
  response.writeHead(status, { "Content-Type": "text/plain; charset=utf-8" });
  response.end(message);
}

const server = createServer(async (request, response) => {
  response.setHeader("X-Content-Type-Options", "nosniff");
  response.setHeader("Cache-Control", "no-store");
  response.setHeader("Content-Security-Policy", "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'");
  if (request.method !== "GET" && request.method !== "HEAD") {
    response.setHeader("Allow", "GET, HEAD");
    reply(response, 405, "只支持读取静态预览文件。");
    return;
  }

  let pathname;
  try {
    pathname = decodeURIComponent((request.url ?? "/").split("?")[0]);
  } catch {
    reply(response, 400, "无效的路径编码。");
    return;
  }
  if (!pathname.startsWith("/") || pathname.includes("\\") || pathname.includes("\0") || pathname.split("/").some((part) => part === "." || part === "..")) {
    reply(response, 403, "无法读取此路径。");
    return;
  }
  if (pathname === "/") pathname = "/assets/preview/companions/index.html";
  const sourceDocument = resolve(root, "docs/design/rare-ai-companions.md");
  const allowed = pathname.startsWith("/assets/preview/companions/") || pathname.startsWith("/packages/kodama-sdk/dist/") || pathname === "/docs/design/rare-ai-companions.md";
  if (!allowed) {
    reply(response, 404, "没有这个预览文件。");
    return;
  }

  try {
    const target = await realpath(resolve(root, `.${pathname}`));
    const allowedRoots = [resolve(root, "assets/preview/companions"), resolve(root, "packages/kodama-sdk/dist")];
    if (!target.startsWith(root + sep) || (!allowedRoots.some((directory) => target.startsWith(directory + sep)) && target !== sourceDocument)) {
      reply(response, 403, "无法读取此路径。");
      return;
    }
    const info = await stat(target);
    if (!info.isFile()) {
      reply(response, 404, "没有这个预览文件。");
      return;
    }
    const body = await readFile(target);
    response.writeHead(200, {
      "Content-Type": types[extname(target)] ?? "application/octet-stream",
      "Content-Length": body.length,
    });
    response.end(request.method === "HEAD" ? undefined : body);
  } catch (error) {
    if (error.code === "ENOENT" || error.code === "ENOTDIR") {
      reply(response, 404, "文件尚未生成；请先运行 make wasm，再重新载入预览。");
      return;
    }
    console.error("读取预览文件失败：", error.message);
    reply(response, 500, "暂时无法读取预览文件。");
  }
});

server.on("error", (error) => {
  console.error(`无法启动伙伴预览：${error.message}`);
  process.exitCode = 1;
});
server.listen(port, address, () => {
  console.log(`伙伴预览：http://${address}:${port}`);
  console.log("只读取已有静态文件；修改渲染器或 SDK 后请先运行 make wasm。");
});

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => server.close());
}
