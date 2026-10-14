# pet-cli

English | [简体中文](https://github.com/VOD-Studio/kodama/blob/main/crates/runtimes/pet-cli/README.zh-CN.md)

The native command-line entry to [kodama](https://github.com/VOD-Studio/kodama), the electronic pet that lives in your site. It runs the same Rust rules core as the browser and Node SDK, so the same input produces byte-identical output on every platform.

- `pet-cli live`: an animated pet in your terminal. It restores the screen, cursor and terminal mode on `q`, Ctrl+C and SIGTERM.
- `pet-cli exec`: one JSON command per line on stdin, one JSON result per line on stdout. Use it from scripts and agents.
- `pet-cli capabilities`: the machine-readable capability catalog. It comes from the same source as the SDK schema.

## Install

Prebuilt binaries are attached to every [GitHub Release](https://github.com/VOD-Studio/kodama/releases). Each archive holds one binary plus the license and this README. The Linux binaries are statically linked, so they run on any distribution, including Alpine.

| Platform | Archive |
|---|---|
| Linux x64 (static, musl) | `pet-cli-x86_64-unknown-linux-musl.tar.gz` |
| Linux arm64 (static, musl) | `pet-cli-aarch64-unknown-linux-musl.tar.gz` |
| macOS Apple silicon | `pet-cli-aarch64-apple-darwin.tar.gz` |

```sh
target=aarch64-apple-darwin   # pick one from the table
curl -fsSLO "https://github.com/VOD-Studio/kodama/releases/latest/download/pet-cli-$target.tar.gz"
tar -xzf "pet-cli-$target.tar.gz"
install "pet-cli-$target/pet-cli" ~/.local/bin/   # or anywhere on your PATH
pet-cli --version
```

To check the download, compare it with `SHA256SUMS` from the same release, or verify its build provenance with the GitHub CLI:

```sh
gh attestation verify "pet-cli-$target.tar.gz" --repo VOD-Studio/kodama
```

On other platforms, build from source with Rust:

```sh
cargo install --locked --git https://github.com/VOD-Studio/kodama pet-cli
```

Windows is not supported yet, because its terminal handling (alternate screen, signals, resize) needs a separate implementation.

## Usage

```sh
pet-cli live                     # animated pet; q to quit, n for the next species
pet-cli capabilities             # what exec accepts, with error codes
echo '{"op":"frame","species":1,"genes":[1,2,3,4,5,6,7,8],"pose":"idle","charset":"ascii","w":10,"h":3,"t":"0"}' | pet-cli exec
pet-cli --version                # release version and rules version
```

`exec` reads one command per line and writes one result per line, in the same order. Each result is either `{"ok":true,"value":…}` or `{"ok":false,"code":…,"message":…}`. Branch on `code`; `message` is for humans. The commands, their result types and error codes are listed in the [capability catalog](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/docs/capabilities.md), and the [JSON Schema](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/schema/pet.schema.json) is the source of truth. 64-bit integers are encoded as decimal strings.

### Terminal behavior

| Condition | Behavior |
|---|---|
| Output is not a terminal | `live` prints one JSON status line and exits, with no escape codes |
| `NO_COLOR` is set, or `TERM` is `dumb` or empty | No colors |
| `COLORTERM=truecolor` or a `256color` `TERM` | 256 colors; otherwise the 16-color palette |
| Locale is not UTF-8 | ASCII only |
| Terminal is resized | The frame follows the new size; very small terminals do not crash |

Only uncatchable termination (SIGKILL) can leave the terminal in the alternate screen; run `reset` if that happens.

## Embedding instead

To show the pet inside your own terminal tool, do not spawn `pet-cli live`. Embed the renderer so that your tool keeps its input and its screen:

- Node hosts: [`@violetjs/pet/terminal`](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/README.md)
- Rust hosts: depend on the `pet-core` and `pet-term` crates (see `tests/consumers/rust-host` in the repository)

## License

MIT. See [LICENSE](https://github.com/VOD-Studio/kodama/blob/main/LICENSE).
