# 原生交互编辑计算实施记录

2026-10-02，实施中。决策依据为 [ADR 0009](../decisions/0009-product-editor-computation.md)。
首批实现基线为 main `45cb7c6a2a323b96b241fe4bb58a4b94a64a9bd1`。
按用户澄清，Office 实现已迁回主工作区 main（首批提交 `5b5c815`），多余 worktree 已删除；仅 Musterwork 保持隔离。

## 已实现并执行的首批检查

- `mo-presentation-edit::prepare_history` 重算原事务、验证声明前置条件、生成新 revision；
  撤销和重做走既有全图验证及布局失效计算，不恢复旧 revision 或覆盖无关声明。
- `KernelRequest::PrepareHistory` 在同一个 Native/WASM JSON 入口开放此计算；
  Rust Schema 和 TypeScript 合同由生成器生成。
- 薄 `PresentationEditor` 提供初始化、事务、历史、现有页面 placement 及字素边界查询，
  固定 WASM 包运行时增加 `editor` 属性。

检查结果：

| 命令 | 结果与范围 |
|---|---|
| `cargo test --locked -p mo-presentation-edit` | 37 项通过，其中 6 项历史回归；覆盖 Unicode、级联删除、独立修改、冲突、重放、取消和损坏历史输入 |
| `cargo clippy --locked -p mo-presentation-edit --all-targets -- -D warnings` | 通过 |
| `cargo run --locked -p mo-contract-codegen -- write contracts/generated` / `pnpm generate:types` | 生成当前合同 |
| `pnpm check:types` | 通过，包括 editor-client |
| `cargo build --locked -p mo-cli` / WASM release build | 本机 macOS arm64 原生与 wasm32 构建通过，bindgen 0.2.126 |
| WASM bundle build / generated bundle TypeScript consumer | 新开发包构建与真实导出类型检查通过；未作为产品固定发行接入 |
| `pnpm test:editor-client` | 3 项通过；真实 Native/WASM 编辑、撤销、重做候选/回执及错误逐字段一致；实际字素和 placement 查询 |

上述是组件证据，尚无完整浏览器编辑或产品验收结论。没有声称延迟、内存、渲染或兼容性达到专项性能目标。

## 仍需完成

1. 交互编辑所需的完整文字范围/局部样式/段落操作、命中/光标查询、对象与页面复制和精确能力矩阵。
2. 动画导航与 viewport 改变时的状态保持合同。
3. 固定新 SDK/worker/WASM 开发发行，由 Musterwork 统一清单接入；不发布远程包。
4. 产品侧关联原生草稿、持续发布、UI、独立放映和双端恢复/授权/性能验收。

Musterwork 拥有整体专项计划；本文件仅记录本仓内核工作，不创建第二个产品完成标准。
