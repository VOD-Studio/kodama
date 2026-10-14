// 架构守护：workspace 内 crate 的依赖方向与规则核心的依赖范围，见 docs/architecture.md。
//   node scripts/check/check-arch.mjs
// 只读取 Cargo.toml 声明（cargo metadata --no-deps），不解析依赖树、不联网。
// 核心 crate 确实不依赖 std，由 Makefile 的 check-arch 在无 std 目标上编译来保证。
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

// 每个 crate 允许依赖的 workspace crate。新增 crate 必须在这里登记，并同步架构文档。
const LAYERS = {
	"pet-term": [],
	"pet-core": ["pet-term"],
	"pet-issuer": ["pet-core"],
	"pet-store": ["pet-core"],
	"pet-service": ["pet-core", "pet-store", "pet-issuer"],
	"pet-wasm": ["pet-core"],
	"pet-cli": ["pet-core", "pet-term"],
	"pet-schema": ["pet-core", "pet-term"],
};

// 三端一致的前提：规则核心与帧合成只用摘要、HMAC、签名验证与序列化（PRD-0036 前提 1）。
// 只约束正常依赖；测试用的 dev-dependencies 不进入产物。
const CORE = {
	"pet-term": ["serde", "serde_json", "schemars"],
	"pet-core": ["pet-term", "sha2", "hmac", "ed25519-dalek", "serde", "serde_json", "schemars"],
};
const CORE_ATTRS = ["#![no_std]", "#![forbid(unsafe_code)]"];

const meta = JSON.parse(execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps", "--locked"], { encoding: "utf8" }));
const members = new Map(meta.packages.map((p) => [p.name, p]));
const problems = [];

for (const name of members.keys()) {
	if (!(name in LAYERS)) problems.push(`${name}: 未在 scripts/check/check-arch.mjs 的 LAYERS 中登记`);
}
for (const name of Object.keys(LAYERS)) {
	if (!members.has(name)) problems.push(`${name}: 已登记但不在 workspace 中`);
}

for (const [name, pkg] of members) {
	const allowed = LAYERS[name] ?? [];
	for (const dep of pkg.dependencies) {
		if (members.has(dep.name) && !allowed.includes(dep.name)) {
			problems.push(`${name} -> ${dep.name}${dep.kind ? ` (${dep.kind})` : ""}: 不允许的依赖方向，允许的是 [${allowed.join(", ") || "无"}]`);
		}
	}
	if (name in CORE) {
		for (const dep of pkg.dependencies) {
			if (dep.kind === "dev") continue;
			if (!CORE[name].includes(dep.name)) problems.push(`${name} -> ${dep.name}: 规则核心不允许的依赖`);
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
console.log(`arch OK: ${members.size} crates, dependency directions and core dependency allowlist hold`);
