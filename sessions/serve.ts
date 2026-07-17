// sessions/serve.ts — static preview server for sessions/dist (local only).
//   bun sessions/serve.ts        # http://127.0.0.1:8150
import { existsSync, statSync } from "node:fs";

const DIST = new URL("./dist/", import.meta.url).pathname;
const PORT = Number(process.env.PORT ?? 8150);
const MIME: Record<string, string> = {
  html: "text/html; charset=utf-8", js: "text/javascript; charset=utf-8",
  css: "text/css; charset=utf-8", json: "application/json",
  svg: "image/svg+xml", png: "image/png", ttf: "font/ttf",
};
function resolve(path: string): string | null {
  let p = DIST + path.replace(/^\/+/, "").replace(/\.\.+/g, "");
  if (p.endsWith("/")) p += "index.html";
  if (existsSync(p) && statSync(p).isFile()) return p;
  if (existsSync(p + "/index.html")) return p + "/index.html";
  if (existsSync(p + ".html")) return p + ".html";
  return null;
}
Bun.serve({
  hostname: "127.0.0.1",
  port: PORT,
  fetch(req) {
    const url = new URL(req.url);
    const file = resolve(url.pathname === "/" ? "/index.html" : url.pathname);
    if (!file) return new Response("not found: " + url.pathname, { status: 404 });
    const ext = file.slice(file.lastIndexOf(".") + 1);
    return new Response(Bun.file(file), {
      headers: { "content-type": MIME[ext] ?? "application/octet-stream", "cache-control": "no-store" },
    });
  },
});
console.log(`sessions.pocketjs.dev preview: http://127.0.0.1:${PORT}/`);
