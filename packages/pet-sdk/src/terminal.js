import { run } from "./index.js";

const FRAME_MS = 100;

/**
 * 在宿主划定的矩形内绘制宠物。只使用保存/恢复光标与绝对定位;
 * 不读取 stdin、不进入备用屏、不隐藏光标、不修改终端模式。非 TTY 时什么都不做。
 * 宿主负责让自己的输出避开该区域(例如设置滚动区域),并在退出前调用 dispose()。
 * charset 为 "blocks" 时以半格像素画绘制进化外形(需要 256 色),每个字符带前景与背景色,行尾复位颜色。
 */
export function mountTerminalPet({ stream = process.stdout, region, pet, getPose = () => "idle", charset = "ascii" }) {
  if (!stream.isTTY) return { dispose() {} };
  let t = 0;
  let timer = null;
  const draw = () => {
    const w = Math.min(region.width, Math.max(0, (stream.columns ?? 0) - region.col + 1));
    const h = Math.min(region.height, Math.max(0, (stream.rows ?? 0) - region.row + 1));
    if (w < 1 || h < 1) return;
    const lineage = pet.lineage ?? [];
    const r = run({ op: "frame", species: pet.species, genes: pet.genes, pose: getPose(), charset, w, h, t: String(t++), stage: pet.stage ?? lineage.length, lineage });
    if (!r.ok) return;
    let out = "\x1b7";
    r.value.rows.forEach((row, y) => {
      out += `\x1b[${region.row + y};${region.col}H`;
      if (charset !== "blocks") {
        out += row;
        return;
      }
      [...row].forEach((ch, x) => {
        const fg = r.value.colors[y][x];
        const bg = r.value.bg[y][x];
        out += `${fg ? `\x1b[38;5;${fg}m` : "\x1b[39m"}${bg ? `\x1b[48;5;${bg}m` : "\x1b[49m"}${ch}`;
      });
      out += "\x1b[0m";
    });
    stream.write(out + "\x1b8");
  };
  const clear = () => {
    let out = "\x1b7";
    for (let y = 0; y < region.height; y++) out += `\x1b[${region.row + y};${region.col}H${" ".repeat(region.width)}`;
    stream.write(out + "\x1b8");
  };
  draw();
  timer = setInterval(draw, FRAME_MS);
  timer.unref?.();
  return {
    dispose() {
      if (timer === null) return;
      clearInterval(timer);
      timer = null;
      clear();
    },
  };
}
