<!--
功能 PR 用 Create a merge commit 合并：本 PR 的每个提交都会原样进入 main，并各自成为一条 CHANGELOG；
只有发版 PR 用 Squash and merge。
请把提交整理成原子步骤，每个提交信息与 PR 标题都符合 Conventional Commits（「Commit messages」是必需检查）：
  feat(scope): …  新功能，升次版本      fix(scope): …  修复，升补丁版本
  docs / ci / test / refactor / style / chore / build / perf: …  不触发发版
破坏性变更在冒号前加 !，例如 feat(sdk)!: …
提交正文写成列表（「- 」开头，续行缩进两格），不写成段落；Closes #1 这类 trailer 单独成行。
-->

## 改动

## 验证

<!-- 跑过哪些检查：make lint、make test、make three-end……没有 Rust 环境可用 make docker-<目标> -->
