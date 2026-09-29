# 原生模板计算接入原 Runtime 引擎端口

2026-09-28，Musterwork 独立工作区将[来源渲染](product-template-render.md)和[显式模板定义](product-template-source.md)接入原 `NativePresentationEngine` 端口及 Device 进程组合。计算继续来自已安装公共 SDK；没有新增服务、二进制、依赖或任务持久化系统。自动上传维护阶段尚未接入，不能将本项视为模板流程完成。

## 版本与调用

独立档案为 `presentation.office-template@1`，结果为 `presentation-office-template-engine/1`。它们与 `presentation.html-ooxml@1` 及旧结果版本分开验证。原传输的请求/输出字节预算、ContentRef 摘要校验、deadline 和内容提交责任继续保留。

| 计算操作 | 输入与真实输出 |
| --- | --- |
| `office_environment` | 核对固定字体；返回 SDK 设置、renderer、实际字体及 settingsDigest，没有内容文件。 |
| `office_template_source` | 精确 source_ref、document_id、settings_digest 和已授权 PPTX；返回 SDK 快照、原生来源交付、逐页预览和原样 SDK 回执。 |
| `office_template_define` | 同一来源与环境，加明确的公共 TemplateDefinition；重新导入并校验来源版本、语义摘要及参数，返回 package/3、描述和对应资源。 |

后两项只接受完整的单份来源引用及实际字节，不接收任意宿主路径或缺失参数的替代值。环境身份不匹配、deadline 过期、来源或定义不合法时拒绝。结果中的 `completed` 仅表示这一计算操作完成，不是维护 Run 完成、质量通过或模板发布。

`OfficeTemplateEngine` 使用原 `OfficeHost` 的共享额度。解析、计算、输出读取处于同一个有界工作生命周期；取消的阻塞任务退出前继续持有额度。输出按块读取并验证实际摘要，JSON 使用有界序列化。SDK 资产 ID 到原传输 role 的映射保留完整语义；同摘要、同长度、同媒体类型的材料只传输一次。

## Device 接线与后续调用方

`applications/device-runtime/src/presentation.rs` 的原引擎选择组合 `OfficeTemplateEngineRouter`。声明为原生模板档案的请求使用同一 OfficeHost；旧 HTML 档案进入原处理器。缺少 OfficeHost、计算失败或环境变更不会触发远端或旧引擎回退。普通旧环境解析接口仍保留原语义；原生环境解析通过新适配器明确取得。

此阶段没有给旧 `presentation_template` 工具附加新的隐式动作，也没有复用旧 HTML 的 prepare_stage、layouts 或质量报告伪装完成。下一步仍需在原维护 Runtime 中加入明确的原生继任 policy、整份来源的参数阶段、实际压力及交付验证，再由原 CompilationJournal 与目录事务提交。Cloud/Web 路由、模型输入/输出封存和自动上传入口也必须分别完成。

MusterOffice 本仓库的计算核心没有获得权限、内容库、业务任务或用户页面职责；以上均是接入产品自身的适配实现。

## 验证

[阶段证据](../reviews/evidence/2026-09-28-product-template-engine-verification.json)记录 10 项模板测试、1 项档案隔离测试、1 项真实 worker 引擎专项及 Device 全部 target 编译。接入库全部 target 与 Artifact 库目标的严格 Clippy 分别通过。

真实专项使用原 Invocation 测试材料，验证来源渲染、模板定义、重试字节一致、坏环境/定义/过期拒绝、输出篡改拒绝及无回退。它没有运行真实模型或自动上传协调。12 个 SDK 资产描述和字节与前阶段完全一致；传输从 100344 字节、13 份内容降为 69730 字节、11 份内容，减少重复快照及未变化 PPTX 的复制和传输。此数字仅属于该自有样本，不是 RSS、整体耗时或桌面包体积结论。原始 PPTX 为 10735 字节、21 部件；SDK 质量声明未提升范围。

扩展到 Artifact 全部 target 的 Clippy 仍失败：未改动的 `tests/projected_application.rs` 存在 30 个既有 unwrap/unimplemented 诊断，文件与工作区 HEAD 相同。证据保留失败，未通过 allow 隐藏，也未将单独库目标成功计为全部 target 成功。去重专项首次还发现快照使用通用 JSON 媒体类型，现已改为既有 Office 文稿媒体类型并验证。

SDK/worker/依赖锁和候选 `device.v54 / author@7 / Skill v10` 均未变；没有切换 stable。完整模板维护、页面组装、历史迁移、一期高级能力及 P00/E0–E3 仍未完成。

后续[原维护 Agent 定义阶段](product-template-maintenance.md)已通过独立策略 /3 调用此端口，并新增 `office_template_contract` 获取同源 SDK Schema。阶段完成仍由原维护工具独立验证；自动上传、压力与目录提交尚未启用。
