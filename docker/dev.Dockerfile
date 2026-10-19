# 开发环境镜像：与 CI 相同的工具链，本机不装 Rust、Node 也能运行全部 make 目标。
# 基于 slim 镜像以控制体积（约 1.5 GB）；构建产物在命名卷里，另需数 GB 磁盘。
# 不直接使用，经 docker/compose.yaml 与 Makefile 的 docker-* 目标调用，见 CONTRIBUTING.md。
# 工具版本与 CI 一致；wasm-bindgen 必须与 crates/runtimes/kodama-wasm 锁定的版本相同（scripts/check/version.mts 校验）。
FROM node:22-bookworm-slim AS node

FROM rust:1-slim-bookworm
ARG WASM_BINDGEN_VERSION=0.2.129
ARG CARGO_DENY_VERSION=0.20.2
ARG ACTIONLINT_VERSION=1.7.12
ARG PNPM_VERSION=10
# Chromium 只有 TanStack Start 消费方验收（make consumers）需要，约 500 MB，默认不装：
#   make docker-build DEV_CHROMIUM=true
ARG INSTALL_CHROMIUM=false

RUN echo 'Acquire::Retries "5";' > /etc/apt/apt.conf.d/80retries \
 && apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl git make python3 shellcheck jq \
 && if [ "$INSTALL_CHROMIUM" = "true" ]; then apt-get install -y --no-install-recommends chromium; fi \
 && rm -rf /var/lib/apt/lists/*

COPY --from=node /usr/local/bin/ /usr/local/bin/
COPY --from=node /usr/local/lib/node_modules/ /usr/local/lib/node_modules/
RUN npm install -g "pnpm@${PNPM_VERSION}" && npm cache clean --force

RUN rustup component add rustfmt clippy \
 && rustup target add wasm32-unknown-unknown thumbv7em-none-eabihf

# 预编译的二进制，按镜像架构选择；不从源码编译，构建镜像只需几分钟。
RUN set -eu; \
    case "$(uname -m)" in \
      x86_64) triple=x86_64-unknown-linux-musl; go=amd64 ;; \
      aarch64) triple=aarch64-unknown-linux-musl; go=arm64 ;; \
      *) echo "unsupported arch $(uname -m)" >&2; exit 1 ;; \
    esac; \
    gh=https://github.com; \
    curl -fsSL "$gh/wasm-bindgen/wasm-bindgen/releases/download/${WASM_BINDGEN_VERSION}/wasm-bindgen-${WASM_BINDGEN_VERSION}-${triple}.tar.gz" \
      | tar -xz -C /usr/local/bin --strip-components=1 "wasm-bindgen-${WASM_BINDGEN_VERSION}-${triple}/wasm-bindgen"; \
    curl -fsSL "$gh/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}/cargo-deny-${CARGO_DENY_VERSION}-${triple}.tar.gz" \
      | tar -xz -C /usr/local/bin --strip-components=1 "cargo-deny-${CARGO_DENY_VERSION}-${triple}/cargo-deny"; \
    curl -fsSL "$gh/rhysd/actionlint/releases/download/v${ACTIONLINT_VERSION}/actionlint_${ACTIONLINT_VERSION}_linux_${go}.tar.gz" \
      | tar -xz -C /usr/local/bin actionlint; \
    wasm-bindgen --version; cargo-deny --version; actionlint --version | head -1

# 容器以宿主的 UID 运行（docker/compose.yaml 的 user），生成的文件在宿主上仍归本人所有。
# 因此 cargo 缓存、HOME 与挂载点必须对任意 UID 可写；新建的命名卷会继承这里的权限。
ENV HOME=/home/dev \
    CHROME=/usr/bin/chromium \
    CHROME_FLAGS=--no-sandbox
RUN mkdir -p /home/dev /work/target /work/packages/kodama-sdk/node_modules /usr/local/cargo/registry /usr/local/cargo/git \
 && chmod -R a+rwX /usr/local/cargo \
 && chmod 1777 /home/dev /work/target /work/packages/kodama-sdk/node_modules \
 && git config --system --add safe.directory /work
# 容器只用来运行检查，不需要完整调试信息与增量编译缓存；target 卷因此从约 2 GB 降到几百 MB
ENV CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_DEV_DEBUG=line-tables-only
# node_modules 在独立的卷上，pnpm 默认会把 store 建在仓库根目录；改放到 HOME 卷里。
# 无 TTY 时（CI、管道）不询问是否清空 node_modules。
ENV npm_config_store_dir=/home/dev/.pnpm-store \
    npm_config_confirm_modules_purge=false
WORKDIR /work
CMD ["bash"]
