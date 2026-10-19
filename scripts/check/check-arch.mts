// 架构与目录守护，见 docs/architecture.md 的「目录结构」与「依赖规则」。
//   node scripts/check/check-arch.mts
// 检查四件事：
//   1. 根目录只有白名单里的文件与目录，新内容必须放进按用途划分的目录；
//   2. 只能放子目录的位置（crates/、crates/<层>/、tests/、scripts/、packages/）没有散落的文件；
//   3. 每个 crate 位于 crates/<层>/<crate 名>/，层与登记一致；
//   4. crate 之间的依赖方向、规则核心的依赖范围与 no_std 声明。
// 只读取 Cargo.toml 声明（cargo metadata --no-deps）与 git 文件列表，不解析依赖树、不联网。
// 核心 crate 确实不依赖 std，由 Makefile 的 check-arch 在无 std 目标上编译来保证。
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const problems: string[] = [];

// ---------- 目录结构 ----------

// 根目录白名单：工具约定必须在根目录的文件，以及按用途划分的顶层目录。
const ROOT_ENTRIES = new Set([
	// 目录
	".agents", ".github", "assets", "crates", "docker", "docs", "packages", "scripts", "tests",
	// Rust 工具约定的位置
	"Cargo.toml", "Cargo.lock", "rustfmt.toml", "deny.toml",
	// 入口与说明
	"Makefile", "README.md", "README.zh-CN.md", "CONTRIBUTING.md", "CONTRIBUTING.zh-CN.md",
	"CHANGELOG.md", "CONTEXT.md", "LICENSE",
	// 仓库级配置
	".editorconfig", "tsconfig.json", ".gitattributes", ".gitignore", ".git-blame-ignore-revs",
]);
// 这些目录下只允许子目录（分组），文件放进具体的分组里。
const DIRS_ONLY = ["crates", "tests", "scripts", "packages"];
// crate 的层：目录名即层名，含义见架构文档。
const LAYER_DIRS = ["core", "service", "runtimes", "tools"];

// 已提交的文件，加上尚未提交但未被忽略的新文件（本地运行时也能提前发现）。
const files = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard"], { cwd: root, encoding: "utf8" })
	.split("\n")
	.filter((f) => f && fs.existsSync(path.join(root, f)));

for (const top of new Set(files.map((f) => f.split("/")[0]))) {
	if (!ROOT_ENTRIES.has(top)) problems.push(`${top}: 根目录不允许新增内容，请放进按用途划分的目录（见 docs/architecture.md「目录结构」）`);
}
for (const dir of DIRS_ONLY) {
	for (const f of files.filter((f) => f.startsWith(`${dir}/`) && f.split("/").length === 2)) {
		problems.push(`${f}: ${dir}/ 下只放分组目录，文件请放进对应的子目录`);
	}
}
for (const f of files.filter((f) => f.startsWith("crates/") && f.split("/").length === 3)) {
	problems.push(`${f}: crates/<层>/ 下只放 crate 目录`);
}
for (const layer of new Set(files.filter((f) => f.startsWith("crates/")).map((f) => f.split("/")[1]))) {
	if (!LAYER_DIRS.includes(layer)) problems.push(`crates/${layer}/: 未知的层，允许的是 ${LAYER_DIRS.join("、")}`);
}

// ---------- crate 与依赖 ----------

// 每个 crate 的层，以及允许依赖的 workspace crate。新增 crate 必须在这里登记，并同步架构文档。
interface CrateSpec {
	layer: string;
	deps: string[];
}
const CRATES: Record<string, CrateSpec> = {
	"kodama-term": { layer: "core", deps: [] },
	"kodama-core": { layer: "core", deps: ["kodama-term"] },
	"kodama-store": { layer: "service", deps: ["kodama-core"] },
	"kodama-issuer": { layer: "service", deps: ["kodama-core"] },
	"kodama-service": { layer: "service", deps: ["kodama-core", "kodama-store", "kodama-issuer"] },
	"kodama": { layer: "runtimes", deps: ["kodama-core", "kodama-term"] },
	"kodama-wasm": { layer: "runtimes", deps: ["kodama-core"] },
	"kodama-schema": { layer: "tools", deps: ["kodama-core", "kodama-term"] },
};
// 层之间的方向：每层只能依赖自己与列出的层。核心不知道服务与运行入口的存在。
const LAYER_DEPS: Record<string, string[]> = {
	core: ["core"],
	service: ["core", "service"],
	runtimes: ["core"],
	tools: ["core"],
};
// 三端一致的前提：核心层只用摘要、HMAC、签名验证与序列化（PRD-0036 前提 1）。
// 只约束正常依赖；测试用的 dev-dependencies 不进入产物。
const CORE_EXTERNAL = ["sha2", "hmac", "ed25519-dalek", "serde", "serde_json", "schemars"];
const CORE_ATTRS = ["#![no_std]", "#![forbid(unsafe_code)]"];

interface Dependency {
	name: string;
	kind: string | null;
}
interface CargoPackage {
	name: string;
	manifest_path: string;
	dependencies: Dependency[];
}

const meta: { packages: CargoPackage[] } = JSON.parse(execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps", "--locked"], { cwd: root, encoding: "utf8" }));
const members = new Map<string, CargoPackage>(meta.packages.map((p): [string, CargoPackage] => [p.name, p]));

for (const name of members.keys()) {
	if (!(name in CRATES)) problems.push(`${name}: 未在 scripts/check/check-arch.mts 的 CRATES 中登记`);
}
for (const name of Object.keys(CRATES)) {
	if (!members.has(name)) problems.push(`${name}: 已登记但不在 workspace 中`);
}

for (const [name, pkg] of members) {
	const spec = CRATES[name];
	if (!spec) continue;
	const dir = path.relative(root, path.dirname(pkg.manifest_path)).split(path.sep).join("/");
	if (dir !== `crates/${spec.layer}/${name}`) problems.push(`${name}: 应位于 crates/${spec.layer}/${name}，实际在 ${dir}`);
	for (const dep of pkg.dependencies) {
		if (!members.has(dep.name)) continue;
		const kind = dep.kind ? ` (${dep.kind})` : "";
		if (!spec.deps.includes(dep.name)) {
			problems.push(`${name} -> ${dep.name}${kind}: 不允许的依赖，允许的是 [${spec.deps.join(", ") || "无"}]`);
		}
		const depLayer = CRATES[dep.name]?.layer;
		if (depLayer && !LAYER_DEPS[spec.layer].includes(depLayer)) {
			problems.push(`${name} -> ${dep.name}${kind}: ${spec.layer} 层不能依赖 ${depLayer} 层`);
		}
	}
	if (spec.layer === "core") {
		for (const dep of pkg.dependencies) {
			if (dep.kind === "dev" || members.has(dep.name)) continue;
			if (!CORE_EXTERNAL.includes(dep.name)) problems.push(`${name} -> ${dep.name}: 核心层不允许的外部依赖`);
		}
		const lib = fs.readFileSync(path.join(path.dirname(pkg.manifest_path), "src/lib.rs"), "utf8");
		for (const attr of CORE_ATTRS) {
			if (!lib.includes(attr)) problems.push(`${name}: src/lib.rs 缺少 ${attr}`);
		}
	}
}

if (problems.length) {
	console.error(problems.join("\n"));
	process.exit(1);
}
console.log(`arch OK: ${members.size} crates in ${LAYER_DIRS.length} layers, dependency rules and directory layout hold`);
