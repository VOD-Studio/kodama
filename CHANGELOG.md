# Changelog

## [0.2.0](https://github.com/VOD-Studio/kodama/compare/v0.1.1...v0.2.0) (2026-10-14)


### ⚠ BREAKING CHANGES

* **release:** CLI 改由 GitHub Release 分发，服务镜像发布到 GHCR ([#31](https://github.com/VOD-Studio/kodama/issues/31))

### Features

* **release:** CLI 改由 GitHub Release 分发，服务镜像发布到 GHCR ([#31](https://github.com/VOD-Studio/kodama/issues/31)) ([7c5dd12](https://github.com/VOD-Studio/kodama/commit/7c5dd128481ec90766834136e628ed237449029e))
* **sdk:** 导出 @violetjs/pet/wasm，Node 里不再按路径读取 .wasm ([997c7eb](https://github.com/VOD-Studio/kodama/commit/997c7eba8d6f52d2b3415579de2ef7be621b8127))


### Bug Fixes

* **sdk:** npm 页面补上说明、关键字与中英文 README ([2035ba1](https://github.com/VOD-Studio/kodama/commit/2035ba18cc435d0029f1bbc1bef11e3262d5e7a8))

## [0.1.1](https://github.com/VOD-Studio/kodama/compare/v0.1.0...v0.1.1) (2026-10-14)


### Bug Fixes

* **npm:** 包元数据声明 repository ([001b902](https://github.com/VOD-Studio/kodama/commit/001b902990ce6960dc2323ed1dc310fb3cd22d2b))

## 0.1.0 (2026-10-14)


### Features

* **pet:** 出生证书支持密钥轮换与撤销 ([c065915](https://github.com/VOD-Studio/kodama/commit/c065915d4c1f0e1dcef3287afa0134cd6d3f73ad))
* **pet:** 添加 CLI 的 npm 按平台分发与验收 ([f1f4948](https://github.com/VOD-Studio/kodama/commit/f1f4948599f3bdee41e000966c28327d17dea04b))
* **pet:** 添加 Node 终端嵌入与宿主验收 ([fa4abb8](https://github.com/VOD-Studio/kodama/commit/fa4abb8b5ab37fa8b165226b318cf4c81d0ac850))
* **pet:** 添加 Postgres 存储实现与共享契约测试 ([b20b956](https://github.com/VOD-Studio/kodama/commit/b20b95638dd31ef732095a31f24570e24eee61f2))
* **pet:** 添加 Rust 规则核心切片与三端一致性场景 ([c095ffa](https://github.com/VOD-Studio/kodama/commit/c095ffad3df68dad64f128f736d96c9c0774ad56))
* **pet:** 添加 SDK 发布包与外部消费验证 ([7f5f3e7](https://github.com/VOD-Studio/kodama/commit/7f5f3e7ecf2d111dbd81649c5cb65cb9d1b13c66))
* **pet:** 添加 SDK 类型声明生成与类型测试 ([2bf6126](https://github.com/VOD-Studio/kodama/commit/2bf61263546d341b85ce2db7ac81e66badad7a2e))
* **pet:** 添加像素画风与 sprite 命令 ([1d9c1ee](https://github.com/VOD-Studio/kodama/commit/1d9c1ee6e2202e1dbf67fd39c65697f89dc2aee9))
* **pet:** 添加出生承诺—揭示原型与证书验证器 ([d516e6c](https://github.com/VOD-Studio/kodama/commit/d516e6c0477f2190d3a24975a0396f6f4414e9c4))
* **pet:** 添加存储契约与 SQLite 实现 ([ed7bb71](https://github.com/VOD-Studio/kodama/commit/ed7bb71b1b939237dff3177c9fdb43c4b3f32e01))
* **pet:** 添加成长规则：经验、性格漂移与进化分支 ([50c30ea](https://github.com/VOD-Studio/kodama/commit/50c30eacce12f41357e533d78b8f47f18e033b96))
* **pet:** 添加服务镜像与部署说明 ([2b9b626](https://github.com/VOD-Studio/kodama/commit/2b9b626b121cbaccd234162127f613ca1e306d47))
* **pet:** 添加独立宠物服务 ([baa7969](https://github.com/VOD-Studio/kodama/commit/baa7969832937f1f43c2b510f1b0eb89fc6bb216))
* **pet:** 添加线上协议 JSON Schema 生成 ([9ced30f](https://github.com/VOD-Studio/kodama/commit/9ced30fc6eeb489c9699ddb6fd9acf3ca0eb4150))
* **pet:** 添加终端帧合成与 CLI live 模式 ([0a55fe7](https://github.com/VOD-Studio/kodama/commit/0a55fe7a205afa288d0763ca96a84ce1f9fc16c6))
* **pet:** 添加能力文档生成与错误码类型测试 ([b148807](https://github.com/VOD-Studio/kodama/commit/b148807f8f91af6bf7ccd4dd7c16006bef55ff87))
* **pet:** 添加规则版本字段与黄金出生向量 ([76e6b51](https://github.com/VOD-Studio/kodama/commit/76e6b51d0ed3c2212d3864dbb1a5373fed1e624f))
* **pet:** 添加错误码枚举与能力目录 ([0643232](https://github.com/VOD-Studio/kodama/commit/0643232388d558ebf3559e10471b7d0caebbca2a))
* **pet:** 终端画风补齐八个物种与全部姿态 ([bb170a3](https://github.com/VOD-Studio/kodama/commit/bb170a3b14301b037c484ad61c79c868850f257a))


### Bug Fixes

* **pet:** 修正离开后回来即死亡与高频结算冻结衰减 ([acf5f8b](https://github.com/VOD-Studio/kodama/commit/acf5f8b149e57a3a9673882a0f62301206e06080))
* **pet:** 动作请求的幂等指纹不再包含时间 ([6a0a509](https://github.com/VOD-Studio/kodama/commit/6a0a509aadb885566024ec7b493a3b65af28e0df))
* **pet:** 并发的同一次领取不再被当作已有主宠 ([3b52803](https://github.com/VOD-Studio/kodama/commit/3b52803e96952c6f6fdec473357fabc8faa04b6f))
