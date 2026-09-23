# Musterwork 历史依据与迁移边界

演示文稿详细设计最初在 Musterwork 中编写。本页记录来源，使 MusterOffice 文档可以独立阅读；以下是集成方历史路径，不是本仓库的依赖或已迁入文件。

## 来源版本

- 项目：Musterwork。
- 来源分支：`main`。
- 代码基线：`eb0eefcd6d1783789e4f2049181c2914178547fe`。
- 设计原稿：`docs/architecture/presentation-kernel/agent-native-presentation-kernel-design.md`，2026-09-24，v0.1；该原稿当时尚未提交，不由上面的代码 SHA 标识。
- 原稿 SHA-256：`e833caaeda35113583e45262a9e878532ac8ae43a282fbe569ccb29672e7c3c4`。

## 代码观察

| 来源路径                                                   | 设计参考                    |
| ---------------------------------------------------------- | --------------------------- |
| `packages/presentation-engine/src/document/schema.mjs`     | 文档和对象模型              |
| `packages/presentation-engine/src/document/operations.mjs` | 稳定 ID、原子操作、版本检查 |
| `packages/presentation-engine/src/document/style.mjs`      | 主题与布局属性              |
| `packages/presentation-engine/src/document/chart.mjs`      | 图表文本角色和数值格式      |
| `packages/presentation-engine/src/html/measure-dom.mjs`    | 需要替换的 DOM 测量职责     |
| `packages/presentation-engine/src/pptx/compile.mjs`        | 原生对象与资源写出行为      |
| `packages/presentation-engine/src/import/core.mjs`         | PPTX 来源映射与读取语义     |

## 历史文档

- `docs/architecture/agent-runtime/proposals/presentation-client-integration.md`：已有客户端、Runtime、字体资源和缓存合同。
- `docs/testing/reports/2026-09-23-presentation-lightweight-validation.md`：历史轻量候选的测试记录；曾发现图表分类标签、环形图几何和字重差异。候选通过部分样本不等于完整 Office/WPS 兼容。

这些资料仅提供设计依据。本次没有重跑历史性能或外部应用测试，也没有把原始源码、测试文件、SDK 或字体复制到 MusterOffice。后续迁出代码和样本需要单独审查权属和许可。

MusterOffice 从本仓库的[产品定位](../product/vision.md)、[整体架构](../architecture/overview.md)和[演示文稿设计](../design/presentations.md)继续演进，不要求使用者能访问 Musterwork 仓库。
