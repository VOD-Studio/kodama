# Companion artwork sources

The SDK implementation is MIT licensed. The optional AI companion artwork is
separate from that code license. It must not be described as original model-vendor
artwork or as artwork covered by this repository's MIT grant.

## Character references

The planned companion collection references the existing community character
designs collected at [AI gijinka reference](https://genderswapai.com/zh-CN/ai-gijinka).
The reference set is [ZipZipPipe's published group](https://www.bilibili.com/opus/1233069202138464289).
DeepSeek's underlying whale-girl character is by 上善无形; the maid interpretation
is by ZipZipPipe.

DeepSeek's original dsh-TUI whale and whale-girl clips are available, and GPT has an
original adult sprite (below). The other five identifiers are reserved; their artwork is pending and render
requests are rejected instead of displaying the rejected geometric drafts.
There are no accepted Kodama pixel adaptations for those six characters. DeepSeek's
early stages share the original Deepy clips as detailed below; the distinct young
stage remains unfinished. These evolution lines are not claimed as original-creator
or official model-vendor canon. The reference pages retain their individual attribution
and usage conditions. The DeepSeek character reference is marked
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/)
by its reference publisher. Other source conditions should be checked at the
creator's own posts; a reference site's availability is not a blanket permission.

## Selected DeepSeek pixel animation

### Original Deepy whale in the early stages

DeepSeek's baby stage and temporary young stage use the original
[Deepy whale animation by dsh-TUI](https://dshtui.com/pets/deepy-whale/),
linked from its [pet gallery](https://dshtui.com/pets/). Both stages currently
use the same source whale; a distinct young appearance has not been designed.
They are not redrawn Kodama whale designs.

The 20 original PNG sheets contain 894 exported 260×260 RGBA frames with 50ms
durations. `scripts/assets/import-deepy.py` verifies the pinned PNG and decoded
RGBA checksums and losslessly stores the exported frames. Exact source URLs,
hashes and the display conversion are recorded in
`crates/core/kodama-term/src/companions/deepy/SOURCES.json`.

Display uses explicit nearest samples at 5:1, then the same fixed source-coordinate
4×4 Bayer alpha pattern on both web and terminal. Visible samples retain their
original RGB. The source export includes subpixel positioning and partial alpha,
so the 52×52 display conversion is not described as lossless; the original RGBA
frames stay intact in the source atlas. The existing 100ms presentation clock
samples the original 50ms timeline rather than claiming full 20fps playback.
No invented facial-expression or body overlays are applied to this whale.

Deepy is described by its publisher as an unofficial fan theme based on a supplied
whale reference. Its separate redistribution terms have not been confirmed;
the repository's MIT code license does not grant rights to this source artwork.

### Whale-girl adult

The adult DeepSeek pixels and original frame durations are from
[dshtui's whale-girl animation set](https://dshtui.com/pets/whale-girl-emoji/),
published by dsh-TUI. The imported 22 source sheets contain 268 frames at 69×66
logical pixels. They are converted losslessly to indexed RLE data. Presentation
selects existing source clips without repainting faces, adding props or tinting
the original pixels; explicit expressions only select available source clips.

Exact source URLs, retrieval date, source SHA-256 digests and durations are in
`crates/core/kodama-term/src/companions/deepseek/SOURCES.json` in the source
repository. `scripts/assets/import-deepseek.py` reproduces the converted data.
The animation publisher's separate redistribution grant has not been confirmed;
the project's MIT code license must not be substituted for that grant. This
iteration is a local implementation and visual review, not a claim that these
community assets have been cleared for commercial distribution.

### GPT adult (original)

The GPT white dragon girl is an original design made for this project: a concept
image produced with an image generation tool, then reduced to an 80×92 native pixel
grid by `scripts/assets/import-gpt.py`. The source image is
`assets/preview/companions/studies/gpt-v3-grid-study.png`. It does not use any
third-party artwork or the community character designs listed above. Only the adult
stage exists; baby and young are not designed yet. The sprite has two key poses
(eyes open, eyes closed); actions are routed onto them rather than separately animated.

The DeepSeek characters above are fan interpretations. No affiliation with or endorsement by
the referenced AI vendors or original artists is claimed.
