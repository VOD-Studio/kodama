# GPT 造型稿

- 幼年：`gpt-baby.png`；少年：`gpt-young.png`；成年：`gpt-adult.png`；幼年与少年的提示词见 `prompts-baby-young.txt`。
- 这些是内置图像生成工具制作的造型研究，沿用社区白龙拟人方向，不是 OpenAI 官方吉祥物。三张造型稿保持独立于运行用的像素帧。
- 目前仅成年有运行美术。幼年与少年已有造型稿，动画尚未开放。

成年运行资源以 `gpt-adult.png` 为身份参考，另行制作完整坐姿、表情、按键、抱箱、展翼等栅格关键姿势，保存在 `assets/sources/gpt/`。制作提示词和源图说明见该目录；导入清单见 `scripts/assets/gpt/manifest.json`。

运行画布仍为 69×66。导入器以固定网格采样源 PNG、统一调色板与锚点，再生成网页和终端共同读取的帧数据，不再用几何图元重画人物。26 个动作序列复用关键姿势，六种表情为固定姿势；这不等于 26 套独立逐帧手绘动画。

完整制作与验收约束见 `docs/design/companion-motion-standard.md`。
