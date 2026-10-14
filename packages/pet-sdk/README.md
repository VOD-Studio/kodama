# @violetjs/pet

English | [简体中文](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/README.zh-CN.md)

The browser and Node SDK for [kodama](https://github.com/VOD-Studio/kodama), an electronic pet that lives in your site. Pets are born, need care, grow, evolve and can die. The rules run in WebAssembly compiled from the same Rust core as the native CLI and the pet service, so the same input gives byte-identical output everywhere.

- **Deterministic.** Integer-only rules with no clock and no random source of their own: you pass in the time and the seeds. Output in the browser, in Node and in the native build is compared byte for byte in CI.
- **Explicit initialization.** Importing the package does not load or instantiate WebAssembly, so it is safe to import during SSR. Call `initPet()` on the client; after that, every call is synchronous.
- **Typed commands.** TypeScript types, the JSON Schema and the capability catalog are generated from the Rust types, so they cannot drift from the implementation.
- **Terminal embedding.** `@violetjs/pet/terminal` draws the pet inside a rectangle of your own Node terminal tool, without taking over its input or screen.

> The birth protocol has not had an independent security review yet. Until it does, do not describe births as formally verifiable.

## Install

```sh
npm install @violetjs/pet
```

Requires a runtime with WebAssembly support: any modern browser, or Node 22 or later.

## Quick start

In the browser, with any bundler that handles `new URL("…", import.meta.url)` (Vite, webpack 5, Rollup and others):

```ts
import { initPet, run } from "@violetjs/pet";

await initPet();

const born = run({
  op: "birth",
  world: "my-site",
  request_id: "adopt-1",
  server_seed: "aa".repeat(32), // 32 random bytes in hex, committed by your server
  client_seed: "bb".repeat(32),
  born_at: "0",
});
if (!born.ok) throw new Error(born.message);

const fed = run({ op: "act", pet: born.value.pet, action: "feed", now: "60", request_id: "feed-1" });
if (fed.ok) console.log(fed.value.pet.satiety);
else console.log(fed.code); // for example "no_food"
```

In Node, pass the WebAssembly bytes to `initPet`. Node's `fetch` cannot load `file:` URLs:

```js
import { readFile } from "node:fs/promises";
import { initPet, run } from "@violetjs/pet";

await initPet(await readFile(new URL(import.meta.resolve("@violetjs/pet/wasm"))));
```

## API

| Export | Description |
|---|---|
| `initPet(module?)` | Loads and instantiates the WebAssembly. Call it once; later calls return the same promise. `module` may be bytes, a compiled `WebAssembly.Module`, or a URL. |
| `run(command)` | Runs a command object. The result type follows from `command.op`. |
| `execute(json)` | The same, with a JSON string in and a JSON string out. |
| Types | `Command`, `Pet`, `ErrorCode`, `ResultOf<C>` and the other wire types |

Calling `run` or `execute` before `initPet()` has resolved throws.

Every result is either `{ ok: true, value }` or `{ ok: false, code, message }`. Branch on `code`; `message` is for humans. 64-bit integers, such as times, are decimal strings.

### Commands

| `op` | What it does |
|---|---|
| `birth` | Derives a pet's identity, rarity, species and genes from the birth input |
| `act` | Feeds, rests, plays with, heals or waits for a pet, and settles illness up to the given logical time |
| `grant` | Applies a reward that the service has already decided on |
| `evolution_options` | Lists evolution branches for the current stage, with what is still missing |
| `frame` | Renders a pose into a grid of character cells for terminals |
| `sprite` | Renders a pose into a pixel-art palette and grid. Pass the pet's `growth.lineage` as `lineage` to draw its evolved form; each stage has its own body and each branch its own features |
| `verify_birth` | Verifies a birth certificate's signature, commit-reveal and recomputed result |
| `capabilities` | Lists all commands and error codes |

The [capability catalog](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/docs/capabilities.md) (shipped in the package as `docs/capabilities.md`) lists each command's result type and possible errors. The JSON Schema is exported as `@violetjs/pet/schema.json`.

Commands that change state (`birth`, `act`, `grant`) return the new state; the SDK stores nothing. In account mode, the [pet service](https://github.com/VOD-Studio/kodama/blob/main/docs/operations/service.md) holds the authoritative state, and the client only renders it.

## Server-side rendering

The import has no side effects, and `run` throws until `initPet()` resolves. During SSR, render a placeholder and call `initPet()` in an effect on the client. CI verifies this with a production TanStack Start build: the server HTML contains no WebAssembly, and the pet appears after hydration.

## Terminal embedding

```js
import { mountTerminalPet } from "@violetjs/pet/terminal";

const pet = mountTerminalPet({
  region: { row: 1, col: 60, width: 11, height: 3 }, // 1-based
  pet: { species: born.value.species, genes: born.value.genes },
  getPose: () => "idle",
});
// …your tool keeps its own input and output…
pet.dispose(); // stops drawing and clears the region
```

The pet draws only inside `region`, using save and restore cursor plus absolute positioning. It never reads stdin, switches to the alternate screen, hides the cursor or changes terminal modes, and it does nothing when the stream is not a TTY. Your tool keeps its own output out of the region, for example with a scroll region, and calls `dispose()` before it exits.

For a standalone terminal pet, use the native [`pet-cli`](https://github.com/VOD-Studio/kodama/blob/main/crates/runtimes/pet-cli/README.md) instead.

## Versions

The package version follows semver. Rule changes are versioned separately as `rules_version`, which is stored in every pet: a newer package never silently re-derives an existing pet, and refuses with `unsupported_rules` instead of guessing. See [versioning](https://github.com/VOD-Studio/kodama/blob/main/docs/design/versioning.md).

## License

MIT. See [LICENSE](https://github.com/VOD-Studio/kodama/blob/main/LICENSE).
