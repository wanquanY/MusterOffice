# Musterwork 原生 Agent 工具接入

2026-09-28，接续[纯计算 SDK 与原生预览](product-sdk-preview.md)。所有产品实现位于独立 `codex/musteroffice-integration` worktree；`MW:` 表示产品相对路径。没有写入用户正在开发的 Musterwork 主工作区，也没有把产品账号、存储或页面移进内核。

## 实现与发布状态

新增明确的 `artifact.presentation.author@4`，以 `{operation, parameters}` 承载产品操作；文稿 action 直接使用固定 SDK 的 `DocumentAction`。`author@1/@2/@3` 不改变含义，历史 HTML 不被新模型解释。`device.v51` 是单独的待验收合同；活动目录仍为 `device.v50`，默认 PPT 尚未切换。

Device dispatcher 已登记新处理器，组合原能力校验、OfficeHost、Invocation 内容端口、原草稿 reader 与 fenced committer。生产组合编译通过；本次实际测试从登记的 v51 合同构造调用，经同一个原子处理器执行，验证 SQLite、公共 SDK 和固定 worker。它没有运行真实模型会话、桌面安装包或完整发现/Skill 链路，不能据此宣称用户已默认使用新内核。

| 操作 | 当前产品执行 |
| --- | --- |
| `describe` | 公共 SDK schema 与摘要，支持 JSON Pointer 和有界返回 |
| `begin` | SDK create/import → 原生 revision → 原草稿事务；产品分配确定性 draft/Artifact ID |
| `apply` | 同时检查产品 revision pin 与内核 revision，执行真实原子编辑 |
| `export` | 固定原生 worker → 字节验证 → 同一 Artifact 的新版本，草稿继续可编辑 |
| `read` | 按 Pointer 读取草稿元数据、Snapshot 和资源绑定；过大结果给出键/数量和缩小查询提示，不截断后冒充完整内容 |
| `abandon` | 原事务将草稿关闭，不删除已交付 Artifact |

`MW:crates/infrastructure/musteroffice/src/agent/` 只定义产品 envelope 与编排，不复制 PPT 模型、布局或格式算法。请求中的 content ID 由产品解析为当前调用或该草稿已持有的精确 ContentRef；内核只看资源别名与已验证字节。导入读取明确绑定的 PPTX，源文件作为文稿资源及 source ref 保留。

本次接入固定 SDK 清单仍为 `550e264393618a838c8d0cf722184ef939373bd0dfc91d2c4a8c2da87f5dda31`，导出 worker 仍为 `b93e718a02d8620b56c5627ad0c2066cd84cdb694a6fe2d7818399d752a557d0`。没有把尚未完成的核心序列导航工作混入这次通过记录。

## 执行和资源边界

新工具采用原 Runtime 的 `runtime_transaction / IdempotentByKey`。计算输出是可回收候选，草稿、Artifact、Tool 成功回执与 Ledger 由同一事务提交；没有独立发布的远程作业，因此不继承旧 HTML 异步 review 的 QueryBeforeRetry 合同。已有成功调用由原 journal 重放，不能按最新草稿重新计算一次成功。

原请求时间由 SQLite Invocation 记录提供，用于固定草稿计算元数据和 checkpoint 身份；重试不会因临时时钟值变化重复消耗输出额度。这是逻辑计算时间，不是导出耗时测量。产品能力处理器仍校验原 grant、Attempt、请求和 schema，当前期限和调用释放信号进入计算；取消后不能凭旧时间重新获得执行权。本阶段结束时，提交与完成观测仍混用了逻辑时间；后续已分离并验证事务超时与丢响应恢复，见[后续实现](product-native-recovery.md)，不倒记为本阶段证据。

解析、SDK schema 生成和计算使用进程共享的有界计算名额，大文稿解析放在阻塞执行器；调用释放后，已开始的计算仍持有名额直到结束。普通查询不启动渲染 worker。请求最大 32 MiB，模型可见查询值最多 32,000 字节，工具结果最多 64 KiB；产品输出沿用 Invocation 配额。它们是当前明确产品档案，不是完整一期大文稿能力验收结果。

创建/导入/编辑的模型、回执、导出资产、checkpoint 和 Tool result 先由原 Invocation 预留、存储并核对实际字节，再交提交者。结果保留原始五类质量声明，`not_proven` 不会因工具成功自动升级为 `passed`。

## 验证

实际新工具专项覆盖：重复准备和预算复用、准备不等于提交、原 journal 成功回执及引用闭包、真实编辑/查询、过期 pin 拒绝、关闭草稿、未授权资源拒绝、grant 替换拒绝、取消，以及两个正式导出版本和实际 PPTX 再导入。所有输入是自有合成文稿、图片和字体，不使用用户文稿或系统隐式字体。

导出的两份文件均通过 ZIP CRC，每份包内 21 个部件中的所有 XML 均成功解析；第二版标题为实际编辑值 `Agent exported revision 2`。重新导入保留第二版源 PPTX，得到两页来源文稿。五类证据中只有 structure 通过；没有 Office/WPS 打开编辑、视觉像素或播放通过结论。

命令、源码清单摘要、失败修正、严格检查与回归范围见[阶段证据](../reviews/evidence/2026-09-28-product-native-agent-verification.json)。测试运行时间是验证记录，不是性能基准；每项通过只覆盖其绑定源码。

本阶段各范围合计 127 项不同测试通过：101 项库测试、8 项适配器测试、15 项普通 Office 集成及另外 3 项显式 worker 新工具测试。普通集成中的 12 项忽略包含后者 3 项；其余 9 项原 worker 测试保留上一阶段证据，本轮没有重跑。四个相关生产库的严格 Clippy 和适配器 all-targets 严格 Clippy 通过。导出/导入通过后，read 结果做了 Box 布局修正并重跑受影响的编辑/查询测试；证据逐项保留修改前后摘要，不合并成未经执行的全工作区验收。

## 接下来

1. 完成模型侧原生 Skill/发现、固定字体与导出环境解析、模板/历史文稿接入，并以实际 Agent 会话验证；避免让模型猜 renderer 身份、字体来源或历史转换规则。
2. 时间分离、提交前超时与受控丢响应后的数据库重新打开已在[后续阶段](product-native-recovery.md)验证；继续完整产品进程重启和实际会话恢复，不能用逻辑时间推算执行耗时。
3. 完成桌面外部 Office/WPS 编辑后的产品再导入、播放与媒体、共享 Home/PostgreSQL 路由及发行材料；本次只验证 Device 私有 Invocation。
4. 按完整一期能力、Native/WASM、Office/WPS、性能、体积及 P00/E0–E3 分别验收，再切换默认引擎。

新工具入口已实现和局部实测，不等于完整 PPT 功能、默认可用或替换完成；目标继续保持进行中。
