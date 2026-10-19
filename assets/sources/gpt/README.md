# GPT 成年运行源图

以项目已有的 `assets/preview/companions/studies/gpt/gpt-adult.png` 为身份参考，使用内置图像生成工具延展白龙娘关键姿势。沿用社区白龙拟人方向，不是官方吉祥物；不把 AI 生成源图描述为手工像素动画。

所有 atlas 为三列两行，按从左到右、从上到下排列：

| 文件 | 姿势 |
|---|---|
| expressions-chroma.png | 好奇、专注、害羞、喜悦、受惊、得意 |
| rest-chroma.png | 睡眠、半闭眼、闭眼、思考、按键、抱箱 |
| tasks-chroma.png | 进食、玩球、举手庆祝、竖拇指、摊掌询问、整理卡片 |
| motion-chroma.png | 展翼、半展翼、收翼、左看、**弃用镜像格**、扶额 |
| followthrough-chroma.png | 按下、松键、调整抱箱、咬点心、拨球、放卡片 |
| look-right-chroma.png | 独立单格右侧眼神，替代镜像格 |

`prompts.json` 保存制作提示词。绿幕是源图的工作底色，导入器去除后只保留二值透明度。motion 表第五格的发饰和龙尾方向错误，manifest 不引用它。三十张实际姿势共用 69×66 画布、固定取样网格和 RGB 调色板，网页和终端读取同一份生成数据。

`scripts/assets/gpt/manifest.json` 是导入依据，包含源图哈希、取样尺寸、裁切、头脚锚点和动作时长。导入器不会自动按每帧包围框缩放，也不通过镜像、矩形部件移动、程序画脸或悬浮图标补动作。动作由关键姿势切换构成，部分动作复用姿势。

重建与验收见 `docs/design/companion-motion-standard.md`。
