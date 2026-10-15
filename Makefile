# 开发、检查与验收入口。CI 调用同名目标，本地结果与 CI 一致。
# 本机没有 Rust/Node 时，任何目标都可以加 docker- 前缀在开发容器里运行，例如 make docker-test。
.DEFAULT_GOAL := help
SUB ?= alice
DEV_CHROMIUM ?= false
NO_STD_TARGET := thumbv7em-none-eabihf
SHELL_SCRIPTS := $(shell git ls-files '*.sh')

.PHONY: help fmt fmt-check clippy check-arch lint-scripts lint-workflows check-versions deny lint \
	test test-pg three-end schema schema-check sdk-test tty e2e e2e-pg consumers ci \
	term preview dev-init dev token wasm cli-archive image image-smoke clean \
	docker-build docker-shell docker-dev docker-token docker-down docker-clean

##@ 日常开发

help: ## 列出全部目标
	@awk 'BEGIN {FS = ":.*## "} /^##@/ {printf "\n%s\n", substr($$0, 5)} /^[a-z0-9%-]+:.*## / {printf "  %-16s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

term: ## 在终端里养一只宠物
	cargo run -p kodama -- live

preview: ## 在终端里预览像素画：ARGS="forms 3" 全部外形，"poses 3" 姿态，"outfits 3" 装扮，"individuals 3" 不同个体，"ascii" 字符画
	cargo run -q -p kodama-term --example preview -- $(ARGS)

dev-init: ## 生成 .dev/ 里的开发密钥与环境(已存在则保留)
	node scripts/dev/dev.mjs init

dev: ## 启动本地服务(SQLite,127.0.0.1:8787),模拟接入方由 scripts/dev/dev.mjs 提供
	node scripts/dev/dev.mjs serve

token: ## 以 SUB(默认 alice)换取访问令牌,供 curl 使用;需先 make dev
	@node scripts/dev/dev.mjs token $(SUB)

fmt: ## 格式化 Rust 代码(含 tests/consumers/rust-host)
	cargo fmt --all
	cargo fmt --manifest-path tests/consumers/rust-host/Cargo.toml

schema: ## 由 Rust 类型重新生成 schema、TypeScript 类型与能力文档
	packages/kodama-sdk/scripts/gen.sh

##@ 静态检查(make lint 汇总,CI 的 Lint 作业)

lint: fmt-check clippy check-arch check-versions lint-scripts lint-workflows deny ## 全部静态检查

fmt-check: ## 格式检查,不改文件
	cargo fmt --all --check
	cargo fmt --manifest-path tests/consumers/rust-host/Cargo.toml --check

clippy: ## clippy,警告即失败
	cargo clippy --workspace --all-targets -- -D warnings

check-arch: ## 目录结构白名单、crate 分层与依赖方向、核心依赖范围,并在无 std 目标上编译核心
	node scripts/check/check-arch.mjs
	cargo build -q -p kodama-core -p kodama-term --target $(NO_STD_TARGET)

check-versions: ## 各处版本号与 wasm-bindgen 版本一致
	node scripts/check/version.mjs check

lint-scripts: ## shellcheck 全部 shell 脚本
	shellcheck --severity=warning $(SHELL_SCRIPTS)

lint-workflows: ## actionlint 检查 GitHub 工作流
	actionlint

deny: ## cargo-deny:许可、来源、重复依赖与安全公告(DENY_CHECKS 可只选部分)
	cargo deny check $(DENY_CHECKS)

##@ 测试与验收

test: ## 全部 Rust 测试(SQLite)
	cargo test --workspace

test-pg: ## 存储契约的 Postgres 部分;未设置 KODAMA_TEST_PG_URL 时用 Docker 起一次性数据库
	sh scripts/dev/with-postgres.sh cargo test -p kodama-store -- --include-ignored

three-end: ## 原生、浏览器 WebAssembly、Node WebAssembly 三端输出逐字节比较
	sh tests/parity/run.sh

schema-check: ## 生成物不得与 Rust 类型不同源;转录校验与类型测试
	cd packages/kodama-sdk && pnpm install --frozen-lockfile
	packages/kodama-sdk/scripts/check-schema.sh

tty: ## 真实 pty 中验证 kodama live:动画、resize、退出恢复终端、降级
	cargo build --release -q -p kodama
	python3 tests/tty/tty_test.py

sdk-test: wasm ## SDK 的 JS 部分(网页渲染的生命周期与无障碍),基于刚构建的 dist
	cd packages/kodama-sdk && node --test test/*.test.mjs

e2e: ## 服务端到端(真实二进制、HTTP、重启),SQLite
	sh tests/e2e/run.sh

e2e-pg: ## 同上,另含 Postgres;未设置 KODAMA_TEST_PG_URL 时需要 Docker
	sh scripts/dev/with-postgres.sh sh tests/e2e/run.sh

consumers: ## 外部消费方验收:CLI 发布归档、Node 嵌入、Rust 宿主、TanStack Start(需要 Chrome)
	for t in cli-archive node-embed rust-host tanstack-start; do sh tests/consumers/$$t/run.sh || exit 1; done

ci: lint test test-pg three-end e2e-pg schema-check sdk-test tty ## CI 的 Lint 与 Core 作业(服务镜像除外)

##@ 构建

wasm: ## 构建 SDK(wasm-bindgen web target)到 packages/kodama-sdk/dist
	packages/kodama-sdk/build.sh

cli-archive: ## 打包本机平台的 kodama 发布归档到 target/dist(TARGET 可指定其他目标)
	sh scripts/release/package-cli.sh target/dist $(TARGET)

image: ## 构建服务镜像 kodama-service:dev(在宿主上运行,需要 Docker)
	docker build -f docker/service.Dockerfile -t kodama-service:dev .

image-smoke: image ## 构建服务镜像并验证它以非 root 用户启动、应答健康检查与能力目录
	sh scripts/release/smoke-service-image.sh kodama-service:dev

clean: ## 删除构建产物与 .dev/
	cargo clean
	rm -rf .dev packages/kodama-sdk/dist

##@ Docker 开发环境(本机只需 Docker)

COMPOSE := docker compose -f docker/compose.yaml
# 标准输入不是终端(CI、管道)时不分配 TTY
DOCKER_TTY := $(shell [ -t 0 ] || echo -T)
DOCKER_RUN = DEV_UID=$$(id -u) DEV_GID=$$(id -g) $(COMPOSE) run --rm $(DOCKER_TTY)
# 这些目标需要 Postgres:先启动 compose 里的数据库,再把连接串传进容器
DOCKER_PG_TARGETS := test-pg e2e-pg ci
DOCKER_PG_URL := host=postgres port=5432 user=postgres password=pet dbname=pet

docker-build: ## 构建开发镜像 kodama-dev;DEV_CHROMIUM=true 额外安装 make consumers 需要的 Chromium
	DEV_CHROMIUM=$(DEV_CHROMIUM) $(COMPOSE) build dev

docker-shell: ## 进入开发容器的 shell
	$(DOCKER_RUN) dev bash

docker-dev: ## 在容器里启动本地服务,宿主可访问 127.0.0.1:8787
	$(DOCKER_RUN) --service-ports --name kodama-dev -e KODAMA_LISTEN=0.0.0.0:8787 dev make dev

docker-token: ## 向 make docker-dev 启动的服务换取 SUB 的访问令牌
	@docker exec kodama-dev node scripts/dev/dev.mjs token $(SUB)

docker-down: ## 停止 compose 里的 Postgres 等后台服务
	$(COMPOSE) down

docker-clean: ## 停止服务并删除开发环境的命名卷(构建缓存、依赖)
	$(COMPOSE) down --volumes

docker-%: ## 在开发容器里运行 make <目标>,如 make docker-test、make docker-ci
	$(if $(filter $*,$(DOCKER_PG_TARGETS)),$(COMPOSE) up -d --wait postgres)
	$(DOCKER_RUN) $(if $(filter $*,$(DOCKER_PG_TARGETS)),-e KODAMA_TEST_PG_URL="$(DOCKER_PG_URL)") dev make $* SUB=$(SUB) DENY_CHECKS="$(DENY_CHECKS)"
