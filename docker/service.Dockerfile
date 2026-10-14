# pet-service 镜像。构建上下文为仓库根：docker build -f docker/service.Dockerfile -t pet-service .
# 同目录的 service.Dockerfile.dockerignore 只为本镜像生效（BuildKit），上下文只保留 Cargo 清单与 crates/。
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# 只构建服务本体；SQLite 以 bundled 方式编译进二进制。
RUN cargo build --release --locked -p pet-service

FROM debian:bookworm-slim
# source 把 GHCR 上的镜像关联到本仓库，并继承仓库的访问权限
LABEL org.opencontainers.image.source="https://github.com/VOD-Studio/kodama" \
      org.opencontainers.image.description="kodama pet service: the authoritative store of account pets" \
      org.opencontainers.image.licenses="MIT"
RUN useradd --system --uid 10001 --home-dir /data --create-home pet
COPY --from=build /src/target/release/pet-service /usr/local/bin/pet-service
USER pet
WORKDIR /data
VOLUME ["/data"]
EXPOSE 8787
ENV PET_LISTEN=0.0.0.0:8787
ENTRYPOINT ["/usr/local/bin/pet-service"]
