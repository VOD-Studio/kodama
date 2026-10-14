<!--
PR 标题须符合 Conventional Commits，release-please 据此决定版本号与 CHANGELOG：
  feat(scope): …  新功能，升次版本      fix(scope): …  修复，升补丁版本
  docs / ci / test / refactor / style / chore: …  不触发发版
破坏性变更在冒号前加 !，例如 feat(sdk)!: …

合并方式（不允许 merge commit）：
  Squash and merge  PR 标题成为唯一的提交 —— 小改动，或提交历史零碎时
  Rebase and merge  每个提交原样进入 main，各自成为一条 CHANGELOG —— 提交已整理好的大功能，
                    此时每个提交也须符合上述格式（「Commit messages」检查会提示）
-->

## 改动

## 验证

<!-- 跑过哪些检查：make lint、make test、make three-end……没有 Rust 环境可用 make docker-<目标> -->
