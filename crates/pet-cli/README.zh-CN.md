# pet-cli

[English](https://github.com/VOD-Studio/kodama/blob/main/crates/pet-cli/README.md) | 简体中文

[kodama](https://github.com/VOD-Studio/kodama)（住在站点里的电子宠物）的原生命令行入口。它与浏览器、Node SDK 运行同一份 Rust 规则核心，同样的输入在任何平台上得到逐字节相同的输出。

- `pet-cli live`：在终端里动起来的宠物。按 `q`、Ctrl+C 或收到 SIGTERM 时恢复屏幕、光标与终端模式。
- `pet-cli exec`：标准输入每行一个 JSON 命令，标准输出每行一个 JSON 结果，供脚本与 agent 调用。
- `pet-cli capabilities`：机器可读的能力目录，与 SDK 的 schema 同源。

## 安装

每个 [GitHub Release](https://github.com/VOD-Studio/kodama/releases) 都附带预编译的二进制。归档里只有一个二进制、许可证与本说明；Linux 版本为静态链接，在任何发行版（包括 Alpine）上都能运行。

| 平台 | 归档 |
|---|---|
| Linux x64（静态，musl） | `pet-cli-x86_64-unknown-linux-musl.tar.gz` |
| Linux arm64（静态，musl） | `pet-cli-aarch64-unknown-linux-musl.tar.gz` |
| macOS Apple 芯片 | `pet-cli-aarch64-apple-darwin.tar.gz` |

```sh
target=aarch64-apple-darwin   # 按上表选择
curl -fsSLO "https://github.com/VOD-Studio/kodama/releases/latest/download/pet-cli-$target.tar.gz"
tar -xzf "pet-cli-$target.tar.gz"
install "pet-cli-$target/pet-cli" ~/.local/bin/   # 或 PATH 里的任意目录
pet-cli --version
```

校验下载：与同一 Release 中的 `SHA256SUMS` 比对，或用 GitHub CLI 验证构建来源证明：

```sh
gh attestation verify "pet-cli-$target.tar.gz" --repo VOD-Studio/kodama
```

其他平台可以用 Rust 从源码构建：

```sh
cargo install --locked --git https://github.com/VOD-Studio/kodama pet-cli
```

暂不支持 Windows：它的终端处理（备用屏、信号、尺寸变化）需要单独实现。

## 用法

```sh
pet-cli live                     # 动态宠物；q 退出，n 切换物种
pet-cli capabilities             # exec 接受的命令与错误码
echo '{"op":"frame","species":1,"genes":[1,2,3,4,5,6,7,8],"pose":"idle","charset":"ascii","w":10,"h":3,"t":"0"}' | pet-cli exec
pet-cli --version                # 发布版本与规则版本
```

`exec` 每行读一个命令、按相同顺序每行写一个结果，结果为 `{"ok":true,"value":…}` 或 `{"ok":false,"code":…,"message":…}`。请按 `code` 分支，`message` 只供人读。命令、结果类型与错误码见[能力目录](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/docs/capabilities.md)，以 [JSON Schema](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/schema/pet.schema.json) 为准；64 位整数以十进制字符串编码。

### 终端行为

| 条件 | 行为 |
|---|---|
| 输出不是终端 | `live` 输出一行 JSON 状态后退出，不含控制码 |
| 设置了 `NO_COLOR`，或 `TERM` 为 `dumb`、为空 | 不输出颜色 |
| `COLORTERM=truecolor` 或 `TERM` 含 `256color` | 256 色；否则用 16 色调色板 |
| 区域设置不是 UTF-8 | 只输出 ASCII |
| 终端尺寸变化 | 画面随新尺寸调整，极小的终端也不会崩溃 |

只有无法捕获的终止（SIGKILL）会让终端停留在备用屏，此时执行 `reset` 即可。

## 嵌入到自己的工具

想在自己的终端工具里显示宠物时，不要启动 `pet-cli live`，而是嵌入渲染，让宿主保有自己的输入与屏幕：

- Node 宿主：[`@violetjs/pet/terminal`](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/README.zh-CN.md)
- Rust 宿主：依赖 `pet-core` 与 `pet-term` 两个 crate（见仓库的 `consumers/rust-host`）

## 许可

MIT，见 [LICENSE](https://github.com/VOD-Studio/kodama/blob/main/LICENSE)。
