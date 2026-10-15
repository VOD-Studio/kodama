// 用 Chrome DevTools 协议打开页面，等页面自己报告完成后输出整页 DOM，再正常关闭浏览器。
// 不用 --dump-dom：它的虚拟时间不等 IndexedDB 这类异步存储，而本地模式的存档需要真实完成写入；
// 正常关闭让同一浏览器档案的第二次打开能读到存档。
//
//   node browser.mts <chrome> <profile-dir> <url> <ready-js-expression>
import { spawn } from "node:child_process";
import { readFile, rm } from "node:fs/promises";
import { setTimeout as sleep } from "node:timers/promises";

const [chrome, profile, url, ready] = process.argv.slice(2) as [string, string, string, string];
const flags = (process.env.CHROME_FLAGS ?? "").split(" ").filter(Boolean);
const deadline = Date.now() + 30_000;
const fail = (msg: string): never => {
  console.error(`browser.mts: ${msg}`);
  proc.kill("SIGKILL");
  process.exit(1);
};

// 上一次打开留下的端口文件已经失效
await rm(`${profile}/DevToolsActivePort`, { force: true });
const proc = spawn(
  chrome,
  ["--headless=new", "--disable-gpu", ...flags, `--user-data-dir=${profile}`, "--remote-debugging-port=0", "about:blank"],
  { stdio: "ignore" },
);

let port: string | undefined;
while (!port) {
  if (Date.now() > deadline) fail("Chrome did not open a debugging port");
  port = await readFile(`${profile}/DevToolsActivePort`, "utf8")
    .then((s) => s.split("\n")[0])
    .catch(() => undefined);
  if (!port) await sleep(100);
}

// Chrome DevTools 协议的响应是任意 JSON
type Reply = any;
async function connect(wsUrl: string) {
  const ws = new WebSocket(wsUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  let id = 0;
  const pending = new Map<number, (reply: Reply) => void>();
  ws.onmessage = (e) => {
    const m = JSON.parse(e.data);
    pending.get(m.id)?.(m);
    pending.delete(m.id);
  };
  return (method: string, params = {}) =>
    new Promise<Reply>((resolve) => {
      pending.set(++id, resolve);
      ws.send(JSON.stringify({ id, method, params }));
    });
}

const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = await connect(targets.find((t: { type: string }) => t.type === "page").webSocketDebuggerUrl);
await page("Page.navigate", { url });

const evaluate = async (expression: string) => (await page("Runtime.evaluate", { expression, returnByValue: true })).result?.result?.value;
// 导航刚开始时页面还是 about:blank：只在目标页面上判断是否就绪
const onPage = `location.href === ${JSON.stringify(url)} && document.readyState === "complete"`;
while (!(await evaluate(`${onPage} && Boolean(${ready})`))) {
  if (Date.now() > deadline) fail(`page never became ready: ${ready}`);
  await sleep(100);
}
process.stdout.write(await evaluate("document.documentElement.outerHTML"));

const { webSocketDebuggerUrl } = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
const browser = await connect(webSocketDebuggerUrl);
browser("Browser.close");
await new Promise((resolve) => proc.once("exit", resolve));
process.exit(0);
