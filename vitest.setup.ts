import { readFile } from "node:fs/promises";

/**
 * wasm-bindgen 的 web 目标默认用 fetch 加载 .wasm，
 * 而 Node 的 fetch 不支持 file:// 协议。这里只给测试补一个兜底。
 */
const originalFetch = globalThis.fetch;

globalThis.fetch = (async (input: any, init?: any) => {
  const url =
    typeof input === "string"
      ? input
      : input instanceof URL
        ? input.href
        : input?.url;
  if (typeof url === "string" && url.startsWith("file:")) {
    const bytes = await readFile(new URL(url));
    return new Response(bytes, {
      status: 200,
      headers: { "Content-Type": "application/wasm" },
    });
  }
  return originalFetch(input, init);
}) as typeof fetch;
