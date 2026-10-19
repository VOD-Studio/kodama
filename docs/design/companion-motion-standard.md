# 伙伴动作与表情规范

DeepSeek 使用原站鲸娘帧。GPT 成年使用本项目造型稿延展出的白龙娘栅格关键姿势，两者共享 69×66 运行画布，不要求角色有相同的身体比例。GPT 保留完整的角、长发、龙翼、卷尾和长袍。

## 两端共同的约束

- 网页与终端读取同一份像素、RGB 调色板和帧时长。网页只作整数倍最近邻放大；真彩色终端用半块字符将两行像素放进一行，原生为 69 列×33 行。
- GPT 的透明度只有 0 或 255。绿幕只用于源图制作，导入时去除，不进入运行数据。
- 宿主的 `t` 每刻为 100 ms。GPT 所有帧时长必须是 100 ms 的正整数倍，保证眨眼等短姿势在一轮播放中能实际采样到。DeepSeek 原始帧时长保持不变。
- `end` 固定一张闭眼姿势。`sick`、`evolve`、`end`、`sleep` 等生命姿态优先于临时动作；宿主动作与表情不改变养成数值。
- 尚无运行美术的角色或阶段返回 `bad_input`。已有造型研究不等于已有可播放动画。

## GPT 的实际制作方式

源 PNG 在 `assets/sources/gpt/`，造型参考在 `assets/preview/companions/studies/gpt/`。源图由内置图像生成工具制作；这些是栅格关键姿势，不是逐帧手绘的完整动作。

`scripts/assets/gpt/manifest.json` 明确记录每张源图的 SHA-256、格子布局、取样格、裁切范围、头脚锚点，以及动作使用的姿势和时长。导入器只负责去绿幕、固定网格最近邻采样、锚点对齐、共享调色板与 RLE 编码，不再通过几何图元画人物、移动矩形身体部件或叠加悬浮符号。

格子尺寸定义取样比例；裁切只控制提取范围，不按每帧包围框拉伸人物。单格补图也必须声明固定取样尺寸和原点。全图共用调色板，不抖动，不为网页增加另一套高分辨率角色。

动作共 26 个，六种表情对待机和观察生效，任务动作保留各自的脸。动作共享关键姿势，不声称是 26 套独立逐帧美术：

| 类别 | 动作 | 主要表演 |
|---|---|---|
| 日常 | idle、blink、observe、sleep、wake | 开眼、半闭、闭眼和左右视线 |
| 交互 | eat、play、poke_left、poke_right、tickle、praise | 手持点心与咬口、拨球、受惊与笑脸、竖拇指 |
| 任务 | think、work、parallel、build、organize、carry | 托腮、双手按键与松键、整理卡片、抱箱与调整抱姿 |
| 反馈 | error、complete、await_confirmation、startled、sick | 惊讶、举手、摊掌、扶额 |
| 形态 | move、lift、evolve、end | 两种展翼姿势、收翼、静止闭眼 |

工作和搬箱的循环保持道具在手中；回到开头应接回同一姿势，不能在循环中途让键盘或箱子凭空消失。不同姿势的轮廓和细节仍可能变化，不能把关键姿势切换描述成连续骨骼动画。

## 重建与检查

```sh
python3 -m pip install -r scripts/assets/gpt/requirements.txt
python3 scripts/assets/import-gpt.py --validate-manifest
python3 scripts/assets/import-gpt.py --preview /tmp/gpt-native.png
python3 scripts/assets/import-gpt.py --animation-preview /tmp/gpt-clips
python3 scripts/assets/import-gpt.py --write
python3 scripts/assets/import-gpt.py --check
python3 -m unittest discover -s scripts/assets/gpt -p 'test_*.py'
make wasm
make sdk-test
```

导入会生成 `data.rs`、`frames.bin` 和 `SOURCES.json`。后者保存实际来源、取样参数、锚点、时长与生成物哈希，`--check` 验证重建一致。更换源图必须同时更新 manifest 中的哈希。

验收先看 1 倍原生像素，再看整数倍放大和完整循环：检查虹膜与睫毛、发丝与衣褶、手与道具的边界，尤其是眨眼、按键、抱箱及首尾切换。自动检查覆盖 RLE 边界、100 ms 可采样性、静止终态，以及终端 ANSI 字节还原后的 RGB 与像素帧一致；这些检查不能替代美术审查。
