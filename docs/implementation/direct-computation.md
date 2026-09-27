# 直接计算入口：SDK 与 CLI

2026-09-27 · 延续[计算合同拆分](computation-boundary.md)与 [ADR 0007](../decisions/0007-kernel-only-integration-boundary.md)。这是现有文稿能力的直接调用实现，完整一期、薄 MCP 和产品替换仍在实施。

## 共用调用路径

`mo-presentation-operations::Invocation` 包含 `request` 和可选 `snapshot`。`request` 沿用 `musteroffice.computation/1-draft`；创建/导入不接收旧快照，编辑/导出必须明确提供基准快照。资源字节独立传入，不编码进 JSON，也不把路径、账号、权限或业务任务放进计算请求。

`mo-embedded-sdk::execute` 接收该调用、不可变资源读取器、可选原生 exporter 和取消回调。其创建/导入/编辑/导出复用既有计算与 worker。返回的 `Execution` 持有相关联的 `ComputationReceipt` 和可选二进制候选；调用方可借用收据和字节，也可用 `into_parts()` 转移所有权，不额外克隆完整文稿。

计算收据包含原 `requestId`、计算请求摘要，以及修改后的快照/收据或导出收据。它表示计算完成，不提供持久幂等、任务查询或产品成功提交。普通直接调用不创建数据库。

调用信封最大 65 MiB；请求及快照分别保持原 32 MiB 预算。字段长度检查使用可取消的计数序列化，不为检查长度再生成一个完整 JSON 副本。新调用及收据的 Schema/TS 类型仍由 Rust 生成，不维护第二套领域模型。

## CLI 使用

```sh
mo-cli compute invocation.json inputs.json protected-spool-directory new-output-directory
mo-cli compute invocation.json inputs.json protected-spool-directory new-output-directory worker worker-sha256
mo-cli compute-schema computation-invocation
mo-cli compute-schema computation-receipt
```

导出需要第二种形式的固定 worker；其他当前操作不要求 worker。临时空间由调用方明确指定并保护，输出目录必须不存在。CLI 不创建产品存储服务、历史库、后台任务或 UI。

`inputs.json` 是 CLI 的文件桥接清单，复用现有 `AssetInfo` 并补充 `file` 路径；相对路径以该清单所在目录为基准。没有资源时使用 `[]`。路径由启动 CLI 的调用方授权，未进入计算合同；此清单不能直接作为远程 MCP 接受任意路径的依据。MCP 的宿主资源桥接需要独立完成。

CLI 在计算前核对精确资源覆盖、重复身份、单项/总量/字体预算，再按 64 KiB 分块读取并校验实际摘要，封存为本次执行的不可变输入。声明、路径或 `verification` 字段本身不代替实际字节验证。计算用完后显式释放临时输入；原始文件和业务保留策略仍归调用方。

创建/编辑/导入输出 `result.json` 与空的 `files.json`；导出另有 PPTX、PNG 等实际资产、文件索引与最终文件检查报告。结果 JSON 分块写入并回读核对摘要；导出资产通过同一交付检查器再次检查已保存字节。已有目录不覆盖，普通失败清理本次新建目录。强制结束进程可能留下未完成的调用方输出，恢复和保留由调用方处理，不假称是已提交的业务结果。

成功时 stdout 只返回请求关联信息、结果文件、结果摘要/长度与资产数；`productCommitted` 明确为 `false`。失败返回非零退出码，stderr 为包含 `outcome: failed` 和结构化 `error` 的 JSON，保留核心诊断。没有收到完整成功响应时，调用方不能把过程输出当作产品已成功提交。

SDK 支持合作式取消；CLI 进程生命周期由调用方控制，worker 保留既有超时与清理行为。当前 CLI 没有新增持久取消服务或重启恢复的任务语义。

## 当前验证

专项已覆盖直接创建/原子编辑、请求关联、取消点、信封/分项预算、真实 PPTX 导入、损坏/重复/缺失资源、旧版本冲突、已有目录保护及暂存清理。实际 worker 的两页导出/12 个资产及内容篡改拒绝测试已通过。

当前固定源码上的 21 步检查通过：925 项工作区 Rust 测试、Schema/TS 生成检查、客户端检查、原生构建和专项、旧 MCP 18 项回归、纯计算/旧兼容 WASM 编译及 27 组 Native/WASM 文稿结果一致。6 份计算 Schema 在 Native、WASM 和生成文件之间逐项一致；原 117 份 Schema 和 121 份生成类型字节未变。工作区 20 个忽略项中，14 个真实 worker 专项已分别显式执行，另 6 个是父测试启动的子进程入口；原生 worker 的 12 项与普通工作区有重叠，不能相加声称新增覆盖。

CLI 独立执行创建、可见文字修改、两版真实导出及两版 PPTX 重新导入，成功响应、结果摘要、最终文件和空暂存均核对。两页交付由独立程序检查 12 资产、15 个原生对象、8 段文字和 10 个相关 XSD 部件；这只证明该夹具的结构与原生内容，不能代替 Office/WPS 或完整视觉验收。合成字体只覆盖限定字符：首次尝试未覆盖的英文修改被具体字体诊断拒绝，未产生输出或残留执行目录；正向修改使用该夹具已有的 `A` 字符，没有静默替换字体。

新独立 SDK 仍为 23 个自有库、54 个 registry 包，472 份打包文件；外部消费工程严格检查并实际导出，其 12 份资产与 CLI 逐字节相同。历史 PPTX、预览及其他共 10 份内容资产不变，仅上下文/质量两份元数据因固定 worker 摘要及派生引用变化；依赖闭包没有旧宿主服务、SQLite、Tokio 或 rmcp。源包文件数和归档体积不是桌面安装包或运行性能。

整体回归暴露的暂存锁问题已从释放生命周期修复，见[锁所有权记录](execution-spool-recovery.md)。确定性用例修正前失败、修正后通过；包含真实进程终止的库测试连续 16 轮通过。Linux/Windows 的相关库 all-targets 严格检查通过，运行证据限 macOS。独立文件检查器初次将自身生成报告计入输入，输入不变检查正确拒绝该记录；明确输入清单后重验，所有失败记录保留。

源码、命令、文件和未完成范围见[阶段证据](../reviews/evidence/2026-09-27-direct-computation-verification.json)。底层文字/绘制/播放命令仍保留原实现，薄 MCP/Skill/Plugin、其余高层接口、完整高级内容、Office/WPS 和 Musterwork 替换验收尚未完成。本轮没有修改 Musterwork，也没有宣称已可完整替换。
