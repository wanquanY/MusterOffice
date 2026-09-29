# 决策与评审状态

更新日期：2026-09-27。

## 已确认

- 产品名称：**MusterOffice**。
- 建立独立本地项目，主分支 `main`，不创建额外 worktree。
- 2026-09-27 用户另行要求 **Musterwork 接入工作使用独立 worktree**，释放其产品主工作区；此要求不改变 MusterOffice 自身的主工作区安排。迁移及执行边界见[产品工作区记录](../implementation/product-worktree.md)。
- 面向 Agent，强调高性能、轻量化、多端跨平台。
- 第一期为完整演示文稿内核，包含动画/转场、音视频、SmartArt、公式的创建、编辑和播放/呈现，并完成 Musterwork PPT 能力替换；产品可扩展其他办公场景。
- 计划开源，不采购 Apryse；核心质量和可编辑性要求保留。
- Rust 主体、原生/WASM 目标、TS 薄接入，允许经过验证的 C/C++ 底层组件，不设纯 Rust 限制。
- 为 Musterwork 和其他 AI 产品提供标准化、便捷且独立的 Agent 接入方式；本轮已定义 MCP/Skill/Plugin/SDK 合同。
- 专注办公计算与薄接入；账号权限、持久存储、业务任务及产品页面由接入方实现。普通 SDK 调用不要求自建持久宿主；简化接入不得削减 PPT 功能、质量或可编辑性，见 [ADR 0007](0007-kernel-only-integration-boundary.md)。职责已确认，公共合同拆分和旧实现依赖收敛仍待完成。
- 用户已明确授权完整一期实现、必要依赖与选型实验，持续推进至可以启动 Musterwork PPT 替换验收；见 [ADR 0006](0006-implementation-authorization.md)。组件采用仍需记录版本、许可和验证证据，授权不代表验收通过。

接受记录见 [ADR 0001](0001-project-identity.md)、[ADR 0002](0002-phase-one-complete-presentations.md) 和 [ADR 0003](0003-language-and-component-strategy.md)。[ADR 0004](0004-agent-integration-surfaces.md)记录已确认接入目标与本轮具体架构提案。接受方向不代表实现与兼容验收已经完成。

## 仍待确定

| 事项                       | 当前建议                                  | 决策状态 |
| -------------------------- | ----------------------------------------- | -------- |
| 具体基础组件与跨语言边界   | Rust/TS 已确认；v0.4 明确模块/窄 ABI、字体及绘制候选；组件版本依 E0 证据锁定 | Proposed  |
| 接入协议与发行档案         | 薄 SDK/MCP/CLI/Skill/Plugin 与 I01–I20；计算/宿主合同拆分、具体版本/客户端/包名及实际开销门槛待冻结 | Proposed |
| 一期能力明细与兼容矩阵     | v0.4 已展开104项能力、裁定高级扩展与格式边界；标准枚举实现、目标构建和外部证据待落实 | Proposed  |
| 未来领域的具体范围         | 分别制定文档/表格设计，不提前实现         | Pending  |
| 原有代码迁移               | 复用语义与测试，先审查来源和依赖          | Pending  |
| 原生平台和浏览器版本       | v0.4 固定目标族及锁文件字段；实际 build/设备待登记 | Proposed |
| 默认字体与离线分发         | 开放许可、可编辑嵌入、按家族分发          | Pending  |
| 公开许可证与贡献声明       | Apache-2.0 是候选                         | Pending  |
| 远程托管、组织、公开时点   | 尚未指定                                  | Pending  |
| 包名、命令名、版本策略     | 在接口和语言决定后冻结                    | Pending  |
| 性能预算及验收设备         | v0.4 固定工作负载与计量口径；完整动态包预算和实测待 E0 | Proposed |

演示文稿技术待决项见[现行设计 D01–D09](../design/presentations.md#s22)；[v0.2 历史建议](../archive/presentations-v0.2.md#s22)仅用于追踪，不再维护。这里维护产品级状态。

## 下一阶段

用户已授权将来源设计迁入本仓库并继续完善方案。已形成 v0.3 及七份演示文稿专项规格，保留 v0.2 和六份来源快照；详见[迁移清单](../references/musterwork/README.md)。随后用户确认语言/组件策略，并要求明确开放接入，本轮已增加[接入总设计](../architecture/agent-integration.md)和[接口规格](../design/agent-interfaces.md)。

设计阶段先明确子能力、目标应用、接入档案和预算，并形成[高风险验证计划](../design/presentations/verification-and-roadmap.md)。这些历史记录不代表当前仍处于等待开发授权状态。

随后形成 [v0.4 实施基线](../design/implementation/README.md)与 [ADR 0005](0005-presentation-implementation-baseline.md)，把子能力、数据/操作、算法、验收和实际 Musterwork 合同落到实施规格。用户的新指示已授权源码开发，当前执行[实施计划](../design/implementation/delivery-plan.md)；状态和实际证据见[实现进度](../implementation/progress.md)。合同校验不等于内核/互操作验证。

新的明确用户决定更新本页及对应 ADR；不得因文档里写有“推荐”就将状态改为 Accepted。

2026-09-27 用户明确排除自有权限、存储及产品页面后，当前接入责任以 [ADR 0007](0007-kernel-only-integration-boundary.md)及[内核与产品边界](../design/implementation/kernel-host-boundary.md)为准。ADR 0004 中标准持久宿主等安排保留为历史提案；现有代码和阶段证据不因文档修订而消失，也不代表已符合新边界。
