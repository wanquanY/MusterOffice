# Musterwork 设计资料迁移清单

迁移日期：2026-09-24。目标：使 MusterOffice 可以在本仓库独立阅读其来源与接入依据。内核方案的唯一维护位置为[现行设计](../../design/presentations.md)；Musterwork 的产品运行时合同仍由其原仓库维护。

## 迁移范围

| 本地快照 | 内容 | 用途 |
| --- | --- | --- |
| [文稿合同](presentation-document-contract.md) | 原有声明式文稿、身份和编辑合同 | 输入迁移和行为回归 |
| [客户端接入](presentation-client-integration.md) | Runtime、客户端、资源和提交边界 | 接入责任与任务所有者 |
| [客户端渲染](presentation-client-rendering.md) | 旧转换路线、候选及产品约束 | 历史技术评估，不是当前选型 |
| [轻量运行时](presentation-lightweight-runtime.md) | 旧轻量候选资源和质量要求 | 资源/依赖问题背景 |
| [HTML 创作与模板编译](presentation-html-authoring-and-template-compilation.md) | 文稿、模板、编辑与验收场景 | 产品语义与模板消费者 |
| [原项目 ADR 0005](0005-presentation-html-authoring-and-private-runtime.md) | 原项目创作/执行/发行责任 | 保留原决定的适用范围 |

上列文件是**历史快照**。其中“已采纳”“当前”“已实现”、商业 SDK 选择以及将来时目标只说明来源项目当时的状态；不会成为本项目的批准、实现或验证结论。引用的第三方产品能力仍是原文当时的研究，未在本次迁移逐项重新评估。

## 来源与完整性

来源项目代码 HEAD：`eb0eefcd6d1783789e4f2049181c2914178547fe`。各文件是否有工作区修改、原字节 SHA-256、迁入文件摘要与来源路径记录在[机器可读清单](migration-manifest.json)。未提交文档不由 HEAD 唯一标识，必须同时使用摘要。

迁入时只做三项转换：增加历史状态说明；将互相引用的已迁入文档改成本地链接；将未迁入源码/文档链接改为显式 `Musterwork:相对路径` 文字。没有复制源码、客户文稿、字体、商业 SDK 或媒体，也没有伪造缺失的实现证据。

原内核 v0.1 在 Musterwork 的路径目前已是导航页。本仓库已有完整 v0.2，此次将其工作区评审基线和范围提示归档为 [presentations-v0.2.md](../../archive/presentations-v0.2.md)，再把现行设计更新为 v0.3。不能把导航页当作原始 v0.1，也不能用 v0.2 冒充逐字相同的 v0.1。v0.1 的既有来源记录保留在[历史依据](../musterwork-baseline.md)。

## 维护规则

- 修改内核方案：在 MusterOffice 的 `docs/design/` 和决策记录中进行。
- 修改 Musterwork 的 API、工具、任务与发行合同：由 Musterwork 仓库拥有，必要时更新本项目接入合同。
- 这些快照与摘要固定，不跟随产品日常开发同步改写。新来源版本另建带日期的记录，不覆盖历史。
- 实现/性能测试报告只按其原日期、环境及范围引用；本次未重新运行历史测试，迁入设计不意味着迁入认证。

本次迁移由用户明确授权。原产品仍需使用的接入规范不删除；原内核位置只保留导航，避免两仓库同时维护演进中的内核设计。
