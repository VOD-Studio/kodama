# Contributing

English | [简体中文](CONTRIBUTING.zh-CN.md)

## Toolchain

You can work either with a local toolchain or with only Docker.

**Local.** Install:

- Rust stable with `rustfmt`, `clippy` and the targets `wasm32-unknown-unknown` and `thumbv7em-none-eabihf`
- `wasm-bindgen-cli` 0.2.129. It must match the version pinned in `crates/runtimes/kodama-wasm`, and `make check-versions` verifies that.
- Node 22 or later and pnpm 10
- Python 3, for the real-TTY tests
- Docker, only for `test-pg`, `e2e-pg` and `image` when you have no Postgres of your own
- Optionally `cargo-deny`, `actionlint` and `shellcheck`, which `make lint` uses

**Docker only.** Prefix any make target with `docker-` to run it in the dev container, which has exactly the CI toolchain:

```sh
make docker-build      # build the image kodama-dev (about 1.9 GB) once, and again when docker/dev.Dockerfile changes
make docker-test       # make test in the container
make docker-ci         # the checks of the CI Lint and Core jobs, with Postgres from docker/compose.yaml
make docker-dev        # the local service, reachable from the host at 127.0.0.1:8787
make -s docker-token   # an access token for SUB (default alice) from that service
make docker-shell      # a shell in the container
make docker-down       # stop Postgres
make docker-clean      # also delete the build cache volumes
```

- The container runs with your UID and GID, so files it creates in the repository are owned by you.
- `target/`, `node_modules` and the cargo cache are named volumes, kept apart from the host's build output.
- Chromium is only needed by `make consumers` (the TanStack Start check) and is left out by default. Build with `make docker-build DEV_CHROMIUM=true` to include it.
- **Disk:** the image plus the build volumes need about 4 GB. Docker Desktop's default disk can be too small; raise it under Settings → Resources.
- **Restricted networks:** containers cannot reach a proxy that listens on the host's `127.0.0.1`. Set the proxy in Docker Desktop under Settings → Resources → Proxies, using `host.docker.internal` as the host.

## Before you open a pull request

```sh
make fmt
make lint test       # or: make docker-ci
```

Run the narrower checks that match your change as well: `make three-end` for rules, `make schema` and `make schema-check` for the wire protocol, `make e2e` for the service, `make tty` for the CLI. [docs/architecture.md](docs/architecture.md) has a verification map and a table of which files to touch for common changes.

- **Formatting** is plain rustfmt (`rustfmt.toml` sets only stable options). The one-time reformatting commit is listed in `.git-blame-ignore-revs`; run `git config blame.ignoreRevsFile .git-blame-ignore-revs` so that `git blame` skips it. GitHub's blame view already does.
- **Generated files** (`packages/kodama-sdk/schema/`, `src/types.d.ts`, `docs/capabilities.md`) are committed. Regenerate them with `make schema` and never edit them by hand.
- **Architecture rules** are checked by `make check-arch`. To add a crate or a dependency between crates, update the table in `scripts/check/check-arch.mjs` and `docs/architecture.md` in the same pull request.

## Commits and pull requests

[release-please](https://github.com/googleapis/release-please) derives versions and the CHANGELOG from the commits on `main`, so commit messages and pull request titles follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>
```

| Type | Release effect |
|---|---|
| `feat` | Minor version |
| `fix` | Patch version |
| `perf`, `refactor`, `docs`, `style`, `test`, `build`, `ci`, `chore`, `revert` | No release on their own |

Add `!` before the colon for a breaking change, for example `feat(sdk)!: …`. Before 1.0, a breaking change bumps the minor version. Common scopes are `core`, `term`, `store`, `service`, `cli`, `sdk`, `release` and `deps`. Descriptions may be in Chinese or English.

**Merging.** All three methods are enabled; pick by the kind of pull request:

| Pull request | Method | Why |
|---|---|---|
| Features, fixes, docs and the like | **Create a merge commit** | Every commit lands on `main` as it is, so history stays complete and each `feat` or `fix` becomes its own CHANGELOG entry. The merge commit is titled "Merge pull request #N from …", so release-please does not count the change twice. `git log --first-parent` shows one line per pull request |
| The release pull request (`chore(main): release x.y.z`) | **Squash and merge** | It holds one version commit; squashing names it after the pull request title |
| Small single-commit pull requests (for example Dependabot) | Any | The result is the same |

Do not squash a feature pull request: its commits would collapse into one, losing the history and the per-commit CHANGELOG entries.

**Commit messages.** Shape each pull request as a series of atomic commits with Conventional Commits messages. Write the body as a list, not as paragraphs:

```
feat(service): friends and visits

- Friendships start from an invite code; each side confirms and either can withdraw
- Visits return only the public projection, never inventory or account details

Closes #19
```

List items start with `- `, and continuation lines are indented by two spaces; trailers such as `Closes #1`, `Refs #1` and `BREAKING CHANGE: …` can stand on their own lines. The required **Commit messages** check validates every commit's title and body, and the pull request title, which becomes the commit title on a squash. To fix a message, rewrite it with `git rebase -i` and force-push the branch. Merge commits from `main` are not allowed inside a pull request: rebase onto `main` instead.

Dependabot opens grouped update pull requests monthly. Cargo updates use the `fix(deps)` prefix, because Rust dependencies ship inside every artifact, so merging them triggers a patch release; npm dev tooling, Actions and image updates use `chore(deps)` or `ci` and do not. `wasm-bindgen` is excluded from Dependabot: update it by hand in `crates/runtimes/kodama-wasm/Cargo.toml`, the workflows and `docker/dev.Dockerfile` together. `make check-versions` catches any mismatch.

## Releases

Merging to `main` makes release-please open or update a release pull request with the next version and the CHANGELOG. Merging that pull request tags `vX.Y.Z` and creates the GitHub Release. `release.yml` then publishes:

- `@violetjs/kodama` to npm, after a maintainer approves the `npm` environment;
- the `kodama` archives, `SHA256SUMS` and build provenance to the GitHub Release;
- the multi-arch `kodama-service` image to `ghcr.io/vod-studio/kodama-service`.

The process, the required repository settings and the supported platforms are described in [docs/operations/distribution.md](docs/operations/distribution.md) (Chinese).

## Reporting security issues

Do not open a public issue for a vulnerability. Report it privately through GitHub's [security advisories](https://github.com/VOD-Studio/kodama/security/advisories/new).
