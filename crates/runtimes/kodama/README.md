# kodama

English | [简体中文](https://github.com/VOD-Studio/kodama/blob/main/crates/runtimes/kodama/README.zh-CN.md)

The native command-line entry to [kodama](https://github.com/VOD-Studio/kodama), the electronic pet that lives in your site. It runs the same Rust rules core as the browser and Node SDK, so the same input produces byte-identical output on every platform.

- `kodama live`: an animated pet in your terminal. It restores the screen, cursor and terminal mode on `q`, Ctrl+C and SIGTERM.
- `kodama exec`: one JSON command per line on stdin, one JSON result per line on stdout. Use it from scripts and agents.
- `kodama capabilities`: the machine-readable capability catalog. It comes from the same source as the SDK schema.
- `kodama login`, `status`, `act`, `live --account`: your account pet from a pet service, the same pet you see on the web.

## Install

Prebuilt binaries are attached to every [GitHub Release](https://github.com/VOD-Studio/kodama/releases). Each archive holds one binary plus the license and this README. The Linux binaries are statically linked, so they run on any distribution, including Alpine.

| Platform | Archive |
|---|---|
| Linux x64 (static, musl) | `kodama-x86_64-unknown-linux-musl.tar.gz` |
| Linux arm64 (static, musl) | `kodama-aarch64-unknown-linux-musl.tar.gz` |
| macOS Apple silicon | `kodama-aarch64-apple-darwin.tar.gz` |

```sh
target=aarch64-apple-darwin   # pick one from the table
curl -fsSLO "https://github.com/VOD-Studio/kodama/releases/latest/download/kodama-$target.tar.gz"
tar -xzf "kodama-$target.tar.gz"
install "kodama-$target/kodama" ~/.local/bin/   # or anywhere on your PATH
kodama --version
```

To check the download, compare it with `SHA256SUMS` from the same release, or verify its build provenance with the GitHub CLI:

```sh
gh attestation verify "kodama-$target.tar.gz" --repo VOD-Studio/kodama
```

On other platforms, build from source with Rust:

```sh
cargo install --locked --git https://github.com/VOD-Studio/kodama kodama
```

Windows is not supported yet, because its terminal handling (alternate screen, signals, resize) needs a separate implementation.

## Usage

```sh
kodama live                     # animated pet; q quit, n next species, g next evolution form, u try outfits
kodama capabilities             # what exec accepts, with error codes
echo '{"op":"frame","species":1,"genes":[1,2,3,4,5,6,7,8],"pose":"idle","charset":"ascii","w":10,"h":3,"t":"0"}' | kodama exec
kodama --version                # release version and rules version
```

`exec` reads one command per line and writes one result per line, in the same order. Each result is either `{"ok":true,"value":…}` or `{"ok":false,"code":…,"message":…}`. Branch on `code`; `message` is for humans. The commands, their result types and error codes are listed in the [capability catalog](https://github.com/VOD-Studio/kodama/blob/main/packages/kodama-sdk/docs/capabilities.md), and the [JSON Schema](https://github.com/VOD-Studio/kodama/blob/main/packages/kodama-sdk/schema/kodama.schema.json) is the source of truth. 64-bit integers are encoded as decimal strings.

### Terminal behavior

| Condition | Behavior |
|---|---|
| UTF-8 locale, at least 256 colours and 11 rows | Half-block pixel art from the same sprite grid as the web, two pixels per character cell; a small region crops without scaling |
| Output is not a terminal | `live` prints one JSON status line and exits, with no escape codes |
| `NO_COLOR` is set, or `TERM` is `dumb` or empty | No colors |
| `COLORTERM=truecolor`/`24bit`, or a `direct`/`truecolor` `TERM` | Original 24-bit RGB, labelled `RGB pixels` |
| A `256color` `TERM` without truecolor | Same pixel grid with reduced colours, labelled `256-color fallback`; otherwise the 16-color palette and character art |
| Locale is not UTF-8 | ASCII only |
| Terminal is resized | The frame follows the new size; very small terminals do not crash |

Only uncatchable termination (SIGKILL) can leave the terminal in the alternate screen with the cursor hidden and input in raw mode; run `reset` if that happens.

## Account mode

Sign in once to the pet service of a site that hosts kodama. The CLI and the web then read and change the same pet with the same inventory; neither keeps its own copy.

```sh
kodama login --service https://pet.example.com   # or set KODAMA_SERVICE
# To sign in, open this page and confirm the code BCDF-GHJK: https://example.com/pet/device?code=BCDF-GHJK
kodama status                        # your pet as one JSON line
kodama act feed                      # feed|rest|play|heal|wait|return|recall|evolve BRANCH|equip ITEM|unequip SLOT|depart TOPIC TIER
kodama act feed --request-id tap-1   # retrying the same id replays the first result
kodama live --account                # animated; f feed, p play, r rest, h heal, q quit
kodama logout                        # revoke the session and delete the saved credentials
```

- Login uses device authorization: open the page that `login` prints, sign in to the site and confirm the code. Check that the code matches what your terminal shows.
- Each command prints one JSON result on stdout, `{"ok":true,"value":…}` or `{"ok":false,"code":…,"message":…}`, and hints on stderr. It exits 0 on success, 1 on failure and 2 on a usage error. `not_logged_in` means run `kodama login`.
- Credentials are saved in `~/.config/kodama/credentials.json` (or `$XDG_CONFIG_HOME/kodama`, or `$KODAMA_CONFIG_DIR`), readable only by you. Tokens refresh on their own. Several `kodama` processes can run at once.
- HTTPS uses your operating system's trusted certificates, including roots that a company proxy installs.
- You can sign the CLI out from the site's list of signed-in devices; it then needs `login` again.

## Embedding instead

To show the pet inside your own terminal tool, do not spawn `kodama live`. Embed the renderer so that your tool keeps its input and its screen:

- Node hosts: [`@violetjs/kodama/terminal`](https://github.com/VOD-Studio/kodama/blob/main/packages/kodama-sdk/README.md)
- Rust hosts: depend on the `kodama-core` and `kodama-term` crates (see `tests/consumers/rust-host` in the repository)

## License

MIT. See [LICENSE](https://github.com/VOD-Studio/kodama/blob/main/LICENSE).
