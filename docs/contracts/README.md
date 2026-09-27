# 设计合同与可运行校验

v0.4 · **设计样例，未发布 wire 标准**。已有部分内核实现；此目录是设计基线，不是实现完成清单。

2026-09-27：[ADR 0007](../decisions/0007-kernel-only-integration-boundary.md)已明确纯计算与产品宿主分离。这里保留的设计 `1-draft` 信封及 accepted/job 示例属于旧合同，继续保留其原语义。实现已另行生成 `musteroffice.computation/1-draft` 的[计算请求 Schema](../../contracts/generated/computation-request.schema.json)及配套合同，见[实现记录](../implementation/computation-boundary.md)；没有把旧 job 语义重解释成无状态计算。普通 SDK/MCP 的目标接口无需身份/业务权限/持久任务前提，存储及提交由产品拥有，见[当前接口合同](../design/agent-interfaces.md)。设计校验通过不代表新边界已实现。

本目录把本轮关键边界落实为 [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)。`1-draft` 和 `urn:musteroffice:design:0.4:*` 都是设计标识，不代表已经有线上服务、域名、包名或运行时支持。

直接 SDK/CLI 另有 Rust 生成的[计算调用](../../contracts/generated/computation-invocation.schema.json)与[计算收据](../../contracts/generated/computation-receipt.schema.json)：前者只绑定计算请求和调用方基准快照，后者关联原请求与计算结果。资源读取器/字节独立传输；CLI 的显式文件桥接不进入该合同。实现和验证范围见[直接计算入口](../implementation/direct-computation.md)，旧 MCP 仍待迁移。

| 文件 | 定义/检查的边界 |
| --- | --- |
| [common.schema.json](common.schema.json) | ID/摘要/字节长度、资源角色、版本、预览状态、质量声明 |
| [export-request.schema.json](export-request.schema.json) | `mo_presentations_export` 的 arguments 信封，固定文稿版本与能力档案 |
| [export-result.schema.json](export-result.schema.json) | accepted/succeeded/failed 互斥行为，作业与产物状态 |
| [delivery.schema.json](delivery.schema.json) | 模型/PPTX/预览/播放/资源闭包以及五类质量声明 |
| [capabilities.schema.json](capabilities.schema.json) | 能力计划的状态、维度、N/A 原因及必需回归 |
| [capabilities.json](capabilities.json) | F01–F16 展开的 **104 项**一期能力计划；基线状态不代替[实际实现进度](../implementation/progress.md) |
| [fixtures](fixtures/) | 五个合成正例；所有 hash/长度/ID 都是占位，不对应真实文件或测量结果 |
| [validate.py](validate.py) | 离线 Schema 自检、正反例、清单引用/状态及文档追踪检查 |

这些 Schema 覆盖导出交付边界，不冒充完整 document/transaction/playback IDL。其余类型化记录与操作已在[数据合同](../design/implementation/data-and-operations.md)定义职责及行为；完整 IDL、生成器和标准效果枚举是 P00/E1 的实现交付，不能手写多个语言版本各自发展。

`arguments` 对应 MCP 工具业务参数，不包含 JSON-RPC/协议 `_meta`、身份或授权信息；MCP 2026/2025 wire envelope 由协议适配器负责。未知字段拒绝。字节长度用无损十进制字符串，最大 u64 及跨引用约束由语义清单校验补充。

Asset ID 是逻辑资源引用，hash 不是访问令牌；业务授权域、绑定资源 bytes、Content Store 和下载权限由接入产品负责，不成为内核权限模型。JSON Schema 无法证明 hash 对应真实内容、资源已授权、原生编辑成立或目标应用已运行。真实计算服务必须验证 bytes 和证据来源，不能信任模型提交的 `passed`。

五类声明为 structure、layout、native-editability、playback、target-application；对具体 profile 分开记录，不能“结构通过→Office通过”。成功导出必须有前三类必需检查通过；有动态内容还必须有 playback 验证和播放清单；外部应用未经验证保持 not_proven。用户明确要求已认证目标档案时，not_proven 也不能满足该请求。

F13/F14 是接入/运行工作流，其文档对象 read/create 等维度 N/A，**仍必须将 workflowStatus 和所有 requiredTests 验收到 verified**，不能从发行分母中剔除。其他对象能力也必须通过其 requiredTests；静态内容作为动画目标的组合在 F06 等项验证。

运行：

```sh
python3 docs/contracts/validate.py
```

需要 Python 3 和 `jsonschema`（支持 Draft 2020-12）/`referencing`；本轮使用环境中已有库，没有安装运行内核依赖。校验器仅从本地 registry 解析 `$ref`，不获取网络 schema。不具备依赖时应明确报缺失，不能跳过后宣称通过。

验证包括故意把 accepted 改成 succeeded、伪造 target passed、换输出 hash、悬空资源、重复身份、超 u64、丢证据等反例。它验证设计合同和校验边界，**不能当作 E0、内核测试、PPTX 格式认证或 Office/WPS 互操作结果**。
