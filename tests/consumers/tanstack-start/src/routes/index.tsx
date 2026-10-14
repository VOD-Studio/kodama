import { createFileRoute } from "@tanstack/react-router";
import { initPet, execute } from "@violetjs/pet";
import { mountPet } from "@violetjs/pet/web";
import { useEffect, useRef, useState } from "react";

const birth = JSON.stringify({ op: "birth", world: "w1", request_id: "r1", server_seed: "aa".repeat(32), client_seed: "bb".repeat(32), born_at: "0" });

function probe(): string {
  try { execute(birth); return "executed-before-init"; } catch { return "not-initialised"; }
}

function Home() {
  const [id, setId] = useState("pending");
  const [painted, setPainted] = useState("none");
  const canvas = useRef<HTMLCanvasElement>(null);
  const ssrProbe = probe(); // 渲染期调用:SSR 与 hydration 首帧都不得已实例化
  useEffect(() => {
    let mounted: { dispose(): void } | undefined;
    initPet().then(() => {
      const born = JSON.parse(execute(birth)).value;
      setId(born.id);
      // 真实浏览器里挂载网页渲染：数一数画布上实际画出的不透明像素
      const el = canvas.current!;
      mounted = mountPet(el, { pet: { species: born.species, genes: born.genes, lineage: ["curious", "wanderer"] }, label: "pet", scale: 2, reducedMotion: true });
      const data = el.getContext("2d")!.getImageData(0, 0, el.width, el.height).data;
      let opaque = 0;
      for (let i = 3; i < data.length; i += 4) if (data[i] > 0) opaque++;
      setPainted(String(opaque));
    });
    return () => mounted?.dispose();
  }, []);
  return (
    <main>
      <p id="probe">{ssrProbe}</p>
      <p id="pet">{id}</p>
      <canvas ref={canvas} />
      <p id="painted">{painted}</p>
    </main>
  );
}

export const Route = createFileRoute("/")({ component: Home });
