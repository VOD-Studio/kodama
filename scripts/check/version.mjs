// 发版由 release-please 改版本号;本脚本只校验各处版本一致,防止漂移。
//   node scripts/check/version.mjs get              打印各处版本
//   node scripts/check/version.mjs check [v0.2.0]   各处版本必须一致;给出标签时也必须与标签一致
// 另校验 wasm-bindgen:crate 锁定的版本必须与 CI、开发镜像（docker/dev.Dockerfile）安装的 wasm-bindgen-cli 相同,否则生成的绑定不可用。
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;

function read() {
	const toml = fs.readFileSync(path.join(root, "Cargo.toml"), "utf8");
	const cargo = toml.match(/\[workspace\.package\][^[]*?\bversion\s*=\s*"([^"]+)"/s)?.[1];
	const sdk = JSON.parse(fs.readFileSync(path.join(root, "packages/pet-sdk/package.json"), "utf8")).version;
	return { cargo, sdk };
}

function wasmBindgen() {
	const at = (f) => fs.readFileSync(path.join(root, f), "utf8");
	const found = {
		"crates/runtimes/pet-wasm/Cargo.toml": at("crates/runtimes/pet-wasm/Cargo.toml").match(/^wasm-bindgen\s*=\s*"=([^"]+)"/m)?.[1],
		"docker/dev.Dockerfile": at("docker/dev.Dockerfile").match(/^ARG WASM_BINDGEN_VERSION=(\S+)/m)?.[1],
	};
	const wf = path.join(root, ".github/workflows");
	for (const f of fs.readdirSync(wf)) {
		const v = fs.readFileSync(path.join(wf, f), "utf8").match(/^\s*WASM_BINDGEN_VERSION:\s*(\S+)/m)?.[1];
		if (v) found[`.github/workflows/${f}`] = v;
	}
	return found;
}

const [cmd, tag] = process.argv.slice(2);
const v = read();
if (cmd === "get") {
	console.log(`Cargo workspace: ${v.cargo}\n@violetjs/pet:   ${v.sdk}`);
	for (const [f, ver] of Object.entries(wasmBindgen())) console.log(`wasm-bindgen ${ver.padEnd(8)} ${f}`);
} else if (cmd === "check") {
	const problems = [];
	if (!v.cargo || !SEMVER.test(v.cargo)) problems.push(`Cargo workspace 版本无效: ${v.cargo}`);
	if (v.cargo !== v.sdk) problems.push(`版本不一致: Cargo ${v.cargo}, SDK ${v.sdk}`);
	if (tag !== undefined && v.cargo !== tag.replace(/^v/, "")) problems.push(`标签 ${tag} 与仓库版本 ${v.cargo} 不一致`);
	const wb = wasmBindgen();
	if (new Set(Object.values(wb)).size !== 1) problems.push(`wasm-bindgen 版本不一致: ${JSON.stringify(wb)}`);
	if (problems.length) {
		console.error(problems.join("\n"));
		process.exit(1);
	}
	console.log(v.cargo);
} else {
	console.error("用法: version.mjs <get|check [tag]>");
	process.exit(2);
}
