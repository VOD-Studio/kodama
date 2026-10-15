# Changelog

## [0.5.0](https://github.com/VOD-Studio/kodama/compare/v0.4.0...v0.5.0) (2026-10-15)


### ⚠ BREAKING CHANGES

* 服务与 CLI 只读取 KODAMA_* 环境变量；npm 包、镜像与 CLI 归档改用新名称。

### Code Refactoring

* 包名统一改为 kodama，不再使用 pet-* 前缀 ([3b507ad](https://github.com/VOD-Studio/kodama/commit/3b507ad8d21090839a7334c5d1add72b786e93f3))

## [0.4.0](https://github.com/VOD-Studio/kodama/compare/v0.3.0...v0.4.0) (2026-10-15)


### Features

* **art:** 个体花纹与稀有度光效 ([310a8d8](https://github.com/VOD-Studio/kodama/commit/310a8d84f00164b00d8796600de0426e6aa3f7d4))
* **art:** 个体花纹与稀有度光效，同物种同外形的每一只也一眼能认出来 ([8f0f00f](https://github.com/VOD-Studio/kodama/commit/8f0f00f6480dc6c01fc7491c2a51f34f86cd95ad))
* **term:** make preview 在终端里打印半格像素画，与网页和 live 是同一套画面 ([6bac1fe](https://github.com/VOD-Studio/kodama/commit/6bac1fe6097b5e04b02aa07665d14cacb0c06054))

## [0.3.0](https://github.com/VOD-Studio/kodama/compare/v0.2.0...v0.3.0) (2026-10-14)


### Features

* **art:** 进化改变整副身体，进化分支决定外形，动作全部重做 ([4c2112b](https://github.com/VOD-Studio/kodama/commit/4c2112b613bd8d2ed02cda7c2dd8745d59752404))
* **catalog:** 每个物种 16 种外形的中英文名称 ([7fce38e](https://github.com/VOD-Studio/kodama/commit/7fce38e97761594caff6d71c505ceedee2ad4cb7))
* **cli:** 终端以半格像素画显示进化外形，live 按 g 切换外形 ([52e8896](https://github.com/VOD-Studio/kodama/commit/52e88963a8689bfab62ec4444684ba489a4c99c5))
* **cli:** 账号模式，pet-cli 经设备授权登录，与网页读写同一只宠物 ([f806f42](https://github.com/VOD-Studio/kodama/commit/f806f427939247cad24ee8641d41338803bf81c2)), closes [#15](https://github.com/VOD-Studio/kodama/issues/15)
* **sdk:** 浏览器本地模式 @violetjs/pet/local，宠物存在 IndexedDB 里离线养成 ([2a73245](https://github.com/VOD-Studio/kodama/commit/2a732458efed256ed5045017ddb4c61a25bfb6b2)), closes [#13](https://github.com/VOD-Studio/kodama/issues/13)
* **sdk:** 终端嵌入支持半格像素画与进化外形 ([5077fcf](https://github.com/VOD-Studio/kodama/commit/5077fcf08dfff75c5ba999b25341e14265628541))
* **sdk:** 网页渲染 @violetjs/pet/web，在 canvas 上播放进化后的像素宠物 ([d4505cf](https://github.com/VOD-Studio/kodama/commit/d4505cfdc0ce094c6cba649a3910ec77b8773f31))
* **service:** 好友、拜访与联合探险，双方确认、随时撤回、不转移任何资源 ([46e66b1](https://github.com/VOD-Studio/kodama/commit/46e66b1d6a055603c5320b46489914d0e1a55daf)), closes [#19](https://github.com/VOD-Studio/kodama/issues/19)
* **service:** 宠物投影带上外形，任何客户端都能画出同一只宠物 ([5c890a4](https://github.com/VOD-Studio/kodama/commit/5c890a4e3449649db02415d34ac072e634802df2))
* **service:** 活动事实收件与奖励结算，账号宠物有了食物来源 ([cfbda0b](https://github.com/VOD-Studio/kodama/commit/cfbda0bff2c84eaa6e600b92c526c5b50d26a95b)), closes [#12](https://github.com/VOD-Studio/kodama/issues/12)
* **service:** 登录会话与设备授权，CLI 可以经网页确认登录 ([08c50d0](https://github.com/VOD-Studio/kodama/commit/08c50d0bbb19c4b2dbc709431b050fc2d0e4f219))
* **service:** 纪念列表与详情，逝去的宠物永久可读 ([0a2130a](https://github.com/VOD-Studio/kodama/commit/0a2130a84760ed29fb254b7cb67f1dd5da1b59fc)), closes [#11](https://github.com/VOD-Studio/kodama/issues/11)
* **store:** run_grant 发放已裁决的奖励，与 run_act 共用读取—结算—提交的循环 ([afe56b0](https://github.com/VOD-Studio/kodama/commit/afe56b091c04529bdd083c8604c2620f4a93cf2e))
* 主题探险与收藏，出发、领取、召回，见闻由独立随机通道复算 ([6a06b6a](https://github.com/VOD-Studio/kodama/commit/6a06b6a24161ced07093cc864e6067465f3693f2)), closes [#18](https://github.com/VOD-Studio/kodama/issues/18)
* 装扮与场景，升级解锁、按身体结构穿戴，网页与终端同一套画面 ([87099b0](https://github.com/VOD-Studio/kodama/commit/87099b0330964d286f7a7439fbc567bab9c47316)), closes [#17](https://github.com/VOD-Studio/kodama/issues/17)


### Bug Fixes

* **deps:** sha2 升到 0.11、hmac 升到 0.13，一起迁移到 digest 0.11 ([64fe9c8](https://github.com/VOD-Studio/kodama/commit/64fe9c8bc0f65c9045ce62399b220de1782ae259))
* **sdk:** 本地存档保存外形，重新打开页面后能画出同一只宠物 ([f9ef2b8](https://github.com/VOD-Studio/kodama/commit/f9ef2b84e7fb60e31d768878888c995a65d131c5))


### Performance Improvements

* **service:** 复用数据库连接，Postgres 只在启动时建表 ([bd507d7](https://github.com/VOD-Studio/kodama/commit/bd507d78447b71b1eafbbf640b5931834e7296ce)), closes [#10](https://github.com/VOD-Studio/kodama/issues/10)

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
