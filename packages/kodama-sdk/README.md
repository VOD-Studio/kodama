# @violetjs/kodama

English | [简体中文](https://github.com/VOD-Studio/kodama/blob/main/packages/kodama-sdk/README.zh-CN.md)

The browser and Node SDK for [kodama](https://github.com/VOD-Studio/kodama), an electronic pet that lives in your site. Pets are born, need care, grow, evolve and can die. The rules run in WebAssembly compiled from the same Rust core as the native CLI and the pet service, so the same input gives byte-identical output everywhere.

- **Deterministic.** Integer-only rules with no clock and no random source of their own: you pass in the time and the seeds. Output in the browser, in Node and in the native build is compared byte for byte in CI.
- **Explicit initialization.** Importing the package does not load or instantiate WebAssembly, so it is safe to import during SSR. Call `initPet()` on the client; after that, every call is synchronous.
- **Typed commands.** TypeScript types, the JSON Schema and the capability catalog are generated from the Rust types, so they cannot drift from the implementation.
- **Canvas and terminal rendering.** `@violetjs/kodama/web` animates the pet on a `<canvas>`; `@violetjs/kodama/terminal` draws it inside a rectangle of your own Node terminal tool, without taking over its input or screen.
- **Local mode.** `@violetjs/kodama/local` keeps a pet in the browser's IndexedDB with no account or backend.

> The birth protocol has not had an independent security review yet. Until it does, do not describe births as formally verifiable.

## Install

```sh
npm install @violetjs/kodama
```

Requires a runtime with WebAssembly support: any modern browser, or Node 22 or later.

## Quick start

In the browser, with any bundler that handles `new URL("…", import.meta.url)` (Vite, webpack 5, Rollup and others):

```ts
import { initPet, run } from "@violetjs/kodama";

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
import { initPet, run } from "@violetjs/kodama";

await initPet(await readFile(new URL(import.meta.resolve("@violetjs/kodama/wasm"))));
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
| `act` | Feeds, rests, plays with, heals or waits for a pet, evolves it, puts on and takes off outfits (`equip`, `unequip`), or sends it on an expedition (`depart`, `return`, `recall`), and settles illness up to the given logical time. Each level from 2 to 10 unlocks an outfit item |
| `grant` | Applies a reward that the service has already decided on |
| `evolution_options` | Lists evolution branches for the current stage, with what is still missing |
| `frame` | Renders a pose into a grid of character cells for terminals |
| `sprite` | Renders a pose into a pixel-art palette and grid. Pass the pet's `growth.lineage` as `lineage` to draw its evolved form; each stage has its own body and each branch its own features. Pass `wardrobe.worn` as `outfit` to draw hats, glasses, scarves, capes and background scenes, and the birth `rarity` to draw its sparkle, outline and aura. Each individual also has its own markings from its genes |
| `verify_birth` | Verifies a birth certificate's signature, commit-reveal and recomputed result |
| `capabilities` | Lists all commands and error codes |

The [capability catalog](https://github.com/VOD-Studio/kodama/blob/main/packages/kodama-sdk/docs/capabilities.md) (shipped in the package as `docs/capabilities.md`) lists each command's result type and possible errors. The JSON Schema is exported as `@violetjs/kodama/schema.json`.

Commands that change state (`birth`, `act`, `grant`) return the new state; the commands store nothing. In local mode, `@violetjs/kodama/local` keeps the state in the browser. In account mode, the [pet service](https://github.com/VOD-Studio/kodama/blob/main/docs/operations/service.md) holds the authoritative state, and the client only renders it.

## Web rendering

`@violetjs/kodama/web` animates the pet on a `<canvas>`, with the same forms and actions as the `sprite` command:

```js
import { initPet } from "@violetjs/kodama";
import { mountPet } from "@violetjs/kodama/web";

await initPet();
const pet = mountPet(document.querySelector("canvas"), {
  pet: { species: born.value.species, genes: born.value.genes, lineage: born.value.pet.growth.lineage, outfit: born.value.pet.wardrobe?.worn },
  label: "Ember Kit",
  onActivate: () => console.log("petted"), // optional
});
// after an evolution: pet.setPet({ ...nextProjection })
pet.dispose();
```

- Pixels stay crisp: integer scaling (`scale`, default 4), multiplied by `devicePixelRatio` on high-DPI screens.
- Honours `prefers-reduced-motion`: one still frame, no walking or animation.
- Pauses while the page is hidden; `pause()` and `resume()` are also available.
- By default the canvas lets clicks through to the page (`role="img"`). With `onActivate` it becomes a focusable button that reacts to click, Enter and Space.
- Never reads or writes storage: pass the projection in, and replace it with `setPet()`. `dispose()` stops the clock, removes listeners and restores the canvas.
- The import has no side effects and is safe during SSR; call `mountPet` on the client.

A canvas of at least 56×36 logical pixels (the default is 96×40) never clips an original species form; the extra width is room to walk.

Pass an optional `companion` to render an AI companion instead. Its default canvas is 96×96; `setPet()` adjusts the default height when switching between companions and original species. An explicit `width` or `height` is retained.

```js
const companion = mountPet(document.querySelector("canvas"), {
  pet: { species: born.value.species, genes: born.value.genes, stage: 2, companion: { character: "deepseek" } },
  label: "DeepSeek",
});
companion.setAction("think");
companion.setAction(); // restore the action selected by pose
companion.setExpression("curious");
companion.setExpression();
companion.dispose();
```

Only `deepseek` artwork is currently available: the original Deepy whale for baby and young stages, and the selected whale-girl frames for the adult stage. The young stage currently shares Deepy. The other six enum values (`gpt`, `claude`, `grok`, `glm`, `kimi`, `gemini`) reserve identities for future artwork; raw `sprite` and `frame` requests return `{ ok: false, code: "bad_input", message: "<character> artwork not ready" }` for them. Rejected drafts are excluded from compilation and rendering. `setAction()` and `setExpression()` only change presentation; they do not change the supplied projection, birth identity, rarity or care state. The `sick`, `evolve`, `end` and `sleep` poses take precedence over companion actions.

## Local mode

`@violetjs/kodama/local` keeps a pet in the browser with no account, backend or network. Every action is settled by the same rules core as the service and written in one atomic commit to IndexedDB:

```js
import { initPet } from "@violetjs/kodama";
import { createLocalPet, openIndexedDbStore } from "@violetjs/kodama/local";

await initPet();
const local = createLocalPet({ store: await openIndexedDbStore() });
if (!(await local.get())) await local.adopt();
await local.claimDaily();             // the daily ration: 3 food, once per local day
const fed = await local.act("feed");  // { ok: true, value } or { ok: false, code, message }
await local.reward("read:post-42", { exp: 10, topic: "rust" }); // an activity on your site
const view = await local.get();       // settled up to now, for display only
mountPet(canvas, { pet: { ...view.appearance, lineage: view.pet.growth.lineage } }); // draw it
```

- Logical time is the device clock in minutes. Time away is settled by the rules: however long the pet is left, it comes back sick at worst, never dead. If the clock is set backwards, the pet's time stays at its last settlement instead of rewinding.
- Several tabs can act at once: a version conflict reloads and settles again, so the same food is never spent twice. Passing the same request id again returns the first result without charging twice (`act(action, requestId)`).
- Without a service, resources come from fixed local rules: `claimDaily()` gives 3 food once per local day (plus 1 medicine when the pet has none), and `reward(source, { exp, topic })` gives experience and topics for activities you report, once per `source`, at most 20 times and 100 experience per day. Rewards never give food. `get()` reports what is left today in `allowance`.
- Errors use the core's codes, the service's names for storage cases (`not_found`, `already_has_pet`, `request_conflict`, `busy`), and `already_claimed` and `daily_limit` for the local allowances.
- A local pet lives in the `"local"` world and has no birth certificate. It is not a pet issued by the service, and it is not uploaded or merged into an account.
- `createMemoryStore()` keeps everything in memory, for tests and previews. Both stores run the same contract vectors as the service's SQLite and Postgres stores (`@violetjs/kodama/store`).
- The import has no side effects and is safe during SSR; open the store on the client.

The rules and their reasons are in the [local mode design](https://github.com/VOD-Studio/kodama/blob/main/docs/design/local-mode.md). In account mode the SDK keeps no offline queue: while offline, keep animating the last projection with `mountPet`, and read the authoritative state again when back online.

## Server-side rendering

The import has no side effects, and `run` throws until `initPet()` resolves. During SSR, render a placeholder and call `initPet()` in an effect on the client. CI verifies this with a production TanStack Start build: the server HTML contains no WebAssembly, and the pet appears after hydration.

## Terminal embedding

```js
import { mountTerminalPet } from "@violetjs/kodama/terminal";

const pet = mountTerminalPet({
  region: { row: 1, col: 60, width: 11, height: 3 }, // 1-based
  pet: { species: born.value.species, genes: born.value.genes },
  getPose: () => "idle",
});
// …your tool keeps its own input and output…
pet.dispose(); // stops drawing and clears the region
```

With `charset: "blocks"` (and the pet's `lineage`), the pet is drawn as half-block pixel art. Truecolor terminals receive the original 24-bit sprite RGB; 256-colour output is a colour-reduced fallback on the same pixel grid. `colorMode` defaults to `"auto"` using `TERM`/`COLORTERM`, or can be `"truecolor"`, `"256"`, `"16"` or `"none"`; `NO_COLOR` always disables colour. With only 16 colours or no colour, `blocks` falls back to character art. The pet draws only inside `region`, using save and restore cursor plus absolute positioning. It never reads stdin, switches to the alternate screen, hides the cursor or changes terminal modes, and it does nothing when the stream is not a TTY. Your tool keeps its own output out of the region, for example with a scroll region, and calls `dispose()` before it exits.

Terminal pets also accept `pet.companion`, with optional `getAction` and `getExpression` callbacks evaluated each frame. `blocks` uses the same companion pixels as the web canvas; the other charsets show the character name and current action as ASCII labels. At the same tick, a `w` × `h` block frame represents the exact `w` × `2h` sprite grid; `fg_rgb`/`bg_rgb` carry original RGB (or `null` for transparent/default cells), while `colors`/`bg` retain the legacy 256-colour indices. A 69-column × 33-row region shows a 69×66 sprite at one times scale; smaller regions crop using the same sprite rules, without enlarging the character. The shared sprite renderer resolves source alpha before both outputs, so terminal and canvas use identical visible samples.

For a standalone terminal pet, use the native [`kodama`](https://github.com/VOD-Studio/kodama/blob/main/crates/runtimes/kodama/README.md) instead.

## Versions

The package version follows semver. Rule changes are versioned separately as `rules_version`, which is stored in every pet: a newer package never silently re-derives an existing pet, and refuses with `unsupported_rules` instead of guessing. See [versioning](https://github.com/VOD-Studio/kodama/blob/main/docs/design/versioning.md).

## License

MIT. See [LICENSE](https://github.com/VOD-Studio/kodama/blob/main/LICENSE).
