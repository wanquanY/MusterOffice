# 有序页面到原生文稿的高层创建

2026-09-30。依据 [Agent 接口设计](../design/agent-interfaces.md)的类型化初始内容与高层展开要求，
新增纯计算 `DocumentAction::Compose` 和 Rust `Presentation::compose`。这是减少创建时的
重复字段，不是新的文档事实源、HTML 渲染器或产品模板平台。

## 输入与展开

`compose.presentation` 包含文稿 ID、标题、EMU 页面尺寸和有序 `slides`。每页包含稳定 ID、
背景及按绘制顺序排列的 `elements`。对象声明位置/尺寸、几何、填充、轮廓和可选纯文本。
默认几何为矩形；未指定填充/轮廓明确为 none；未指定文字属性继承宿主显式交付默认值。
示例从真实类型生成并执行验证，见[三页调用](../../fixtures/presentations/compose/invocation.json)。

初期便捷输入支持文字和已有原生形状几何。图片、表格、组合、自定义字体资源和高级内容
继续使用完整 `create`/`apply` 合同；没有删除能力或声称便捷输入已覆盖全部对象。
这些便捷输入不自动补内容、缩小字体或改变画布来掩盖溢出，仍须走真实布局诊断。

- 页面和对象 ID 由作者提供；父子关系、映射表和顺序引用由展开器一次性构造。
- 文字样式使用可选属性，显式 false 与未指定保持区别；字号仍使用精确 EMU。
- LF、CRLF、CR 分隔段落，制表符产生原生 tab run；空段和尾部空段保留。
- paragraph/run ID 从文稿 ID、对象 ID、段落序号经域分离摘要确定。对象移动和文字替换
  不改写这些既有定位键；创建后的修改完全使用原事务与查询，不再次运行展开器。
- 默认文字溢出策略为 `report`，零边距、自动换行；不做截图或路径文字替代。
- 输入和展开结果受原 32 MiB 预算约束，文字展开时逐 run/段落检查剩余额度，防止很小的
  制表符输入膨胀成无界中间结构。重复 ID、错误引用/几何、取消和超限不产生成功候选。

最终只有现有 `Document` 与 Snapshot 进入后续处理。`compose` 和完整 `create` 对同一
展开文档生成相同 revision/semanticDigest；操作请求摘要各自绑定原输入。
Native、WASM、SDK 与 CLI 共用该实现，MCP 使用同一计算入口。已退休持久宿主的冻结
Schema/准入不新增此动作，MusterOffice 不新增存储或产品权限。

## 已验证与未验证

本地相关 Rust 回归通过，覆盖实际三页原生 PPTX 写出、再读取、原事务文字修改、过期
revision 拒绝、SDK 内存恢复、控制字符、稳定身份、预算和取消。两个计算 Schema 及
对应 TS 类型已再生成并检查；相关严格 Clippy 通过。

`tools/verification/compose-parity.mjs` 在实际 release CLI 和 WASM 中执行 13 个成功/拒绝
用例；成功回执的完整 JSON 字节一致，失败类型和详情一致。完整创建的模型等价性及
编辑后的失效 revision 同时检查。实际 MCP 进程另在 `2025-11-25` 与 `2026-07-28`
两种协议模式各执行创建、编辑、过期 revision 和重复 ID 拒绝，共 8 次计算；两端文稿摘要
一致，输出由调用方持有，没有数据库或业务提交。冻结证据见[本阶段记录](../reviews/evidence/2026-09-30-compact-authoring.json)。

这不是排版质量、Office/WPS 互操作或产品效率验收。该实现记录时尚未升级 Musterwork
固定 SDK 或 Tool/Skill，实际模型测试另见[产品验收](live-agent-acceptance.md)。后续须在
产品的版本化接口接通，并实测查询次数、错误、耗时和视觉效果，不能从输入变短推断收益。
