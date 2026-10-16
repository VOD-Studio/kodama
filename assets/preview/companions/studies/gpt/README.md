# GPT 造型稿

- 幼年：`gpt-baby.png`；少年：`gpt-young.png`；提示词见 `prompts-baby-young.txt`。
- 成年：下面两张研究图。

这两张是内置 `image_gen` 生成的原创造型研究，只作为设计参考。运行用的帧由 `scripts/assets/gpt/` 的生成器按 DeepSeek 的规格（69×66 画布、49×48 半身像）重新绘制，再由 `scripts/assets/import-gpt.py` 编码。幼年和少年尚无设计，对应阶段保持未开放。

- `gpt-v2-style-study.png`：细化脸、成组发束、衣褶与饰物的高分辨率像素风插画，仅作参考，不参与导入。
- `gpt-v3-grid-study.png`：收紧翼尾与轮廓的低分辨率像素簇版本，是成年形态的主要造型参考。

## v3 实际生成提示词

Edit image 1 into a REAL LOW-RESOLUTION DESKTOP GAME SPRITE. The previous result is a detailed high-resolution illustration and is NOT usable as pixel art. Preserve the exact GPT white dragon girl identity, silver-lavender palette, two white segmented horns, pointed ears, lavender eyes, long hair, small dragon wings, curled white tail, white long dress and silver knot/tassel ornaments, seated compact pose. But redraw its rendering completely on a true fixed 80x80 logical pixel grid, deliberately designing each pixel cluster for readability at 1x native resolution. Every single logical pixel must be a large perfectly flat hard-edged square displayed at the same integer enlargement across the entire image, including eyes and outlines. Requested canvas is 1280x1280 representing exactly 80x80 logical pixels at 16x display enlargement; no smaller marks than one 16x16 block. Image 2 is the actual existing whale-girl sprite strip: ONE character there occupies approximately 49x48 native pixels within a 69x66 frame, its face uses about 10x10 pixels. Use the same level of native game-sprite simplification and pixel discipline. Keep the dragon girl body inside about 58x60 logical pixels, face about 13x13 logical pixels, wing membranes small and folded close beside shoulders, tail curling compactly around feet rather than extending sideways, horns compact enough that the face stays big. Translate the attractive face, asymmetric layered fringe and dress folds of image 1 into elegant deliberate pixel clusters, NOT many hairline strokes. Dark purple selectively colored 1px outlines; about 48-64 carefully chosen flat RGB colors, 5 distinct lavender hair/white-cloth tones, lavender eyes with dark lashes and 1px highlights, tiny peach blush, tiny mouth. Chibi 2-head proportion, large dimensional head, short compact seated body. The image must visibly read as chunky but refined hand-drawn low-res contemporary anime game sprite, NOT smooth vector, high-res pixel-filtered anime, antialiased drawing, tiny noisy dithering, or geometric stick-figure RPG. Absolutely binary transparency: every character pixel fully opaque alpha255, background pixels fully transparent alpha0. No translucent character fill, no halo, no colored edge noise, no shadow. No text, no guides, no checkerboard printed into image. One character only, no extra sprite frames. Preserve original established character, change only to the production low-res sprite rendering and tighter silhouette.

输入 1 是 v2 造型研究；输入 2 是原站鲸娘 idle 帧条。提示词中的目标尺寸和 alpha 要求是制作目标，不代表生成结果已经符合。
