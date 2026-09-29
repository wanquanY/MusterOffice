# 产品原生模板来源渲染

2026-09-28，在[原生模板生产入口](product-template-source.md)上增加冻结渲染环境和实际来源交付。实现位于 Musterwork 独立工作区的 `infrastructure/musteroffice/src/template/render.rs`，继续使用已有嵌入 SDK、worker、显式字体与 `OfficeHost` 计算额度，没有新增组件或依赖。

后续已接入[原 Runtime 计算端口和 Device 分派](product-template-engine.md)。下文保留来源渲染阶段事实；自动上传维护仍继续实施。

## 计算与结果责任

`OfficeTemplateComputer::environment` 使用 SDK 的 `DeliverySettings::input_digest` 固定设置、renderer 身份和实际字体字节摘要。环境显示名称与产品 Content ID 不进入计算身份。没有明确配置的环境拒绝执行，不读取系统隐式字体。

`render_source` 接受上述摘要，在调用 worker 前核对当前环境并重新核算快照、资源别名与字体的输入预算。实际输出也受宿主数量、单项和总字节预算限制。取消和被丢弃的排队调用继续持有原共享额度，直至工作退出；没有建立新的任务或持久存储。

结果保留完整来源、冻结环境和 SDK `NativeExportCandidate`。回执必须对应同一 document、revision、semanticDigest 和 settingsDigest。宿主读取、保存或放弃真实候选文件，并继续将来源交给显式定义入口。来源预览、模板定义和目录提交各自保留原职责。

SDK 的质量声明原样保留：当前专项的结构检查通过，但完整布局、编辑往返、播放及目标应用兼容性均为 `not_proven`。内核生成了预览不等于已通过独立 Office/WPS 视觉比较；本项不制造旧 HTML 质量报告，也不把输入/输出字节预算当作进程 RSS 证明。

## 验证范围与后续

见[本阶段证据](../reviews/evidence/2026-09-28-product-template-render-verification.json)。10 项模板单元测试（含本轮新增 3 项）、1 项实际 worker 与原 Agent 原生实例专项、接入库全部 target 的严格 Clippy 通过。初次测试代码的 needless borrow 检查失败已修正并保留记录；没有放宽检查。前阶段的 19 项 Agent 与 57 项后端回归未重复计为本轮执行。

实际来源交付的 12 项资产均核对字节、摘要和回执；10735 字节、21 个部件的 PPTX 与来源完全一致。两张 640×360 预览分别为 14915 和 11174 字节，与自有基准逐字节一致；后续模板实例也与前阶段一致。SDK、worker 和依赖锁未变化。这里没有发行体积、运行性能或真实模型会话的结论。

自动上传仍未接入此入口。下一步沿原维护 Runtime、内容端口、CompilationJournal 和目录事务接入原生继任编译档案、真实参数压力与完成验证。单份来源直接保留原生文稿，不需要 HTML 重建。完整高级能力、页面组装、历史迁移和 P00/E0–E3 继续实施；候选 `device.v54 / author@7 / Skill v10` 与 stable 状态不变。
