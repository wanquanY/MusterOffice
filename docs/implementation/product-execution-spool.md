# Musterwork 原生宿主与暂存维护

2026-09-27：接续[原生导出暂存恢复](execution-spool-recovery.md)，将固定 SDK/worker 和恢复协议接入实际 Device 宿主。`MW:` 表示 Musterwork 仓库内的路径，产品私有源码仍由产品仓库管理。

## 固定材料与宿主组合

产品通过原有 `prepare-musteroffice-development.mjs` 导入 SDK manifest `35e9a712d7bde8342783fcdf7a284335fd907d0a0f8acbfd4ad8a6c40650fb07`，实际 Cargo 路径同步到该不可变代际。macOS ARM64 worker 的 SHA-256 为 `47a2f4ff55ed7df0272870e645e02604a252dc355faef8dd6d687921015682e7`，大小 10,940,400 字节；这不是完整安装包体积。旧材料保留，新材料没有就地修改 vendor 或放宽准备校验。

SDK 包含 23 个自有生产库，新增的来源解析库及平台条件依赖按原 SDK 闭包消费。产品锁文件中的 rustix 从 1.1.4 更新为该 SDK 已固定的 1.1.5，沿用[组件许可与来源证据](../../components/rustix/component.json)。此升级需要重跑实际消费者与存储回归，旧阶段测试不自动覆盖新版本。

`MW:apps/agent-runtime/crates/infrastructure/musteroffice/src/host.rs` 的 `OfficeHost` 持有固定原生导出器、共享计算名额、输入/输出限制和单个维护名额。创建/编辑与导出工厂仍只接受 `InvocationContentStore`，每次调用由原 Runtime 注入具体 Invocation 的权限；host 不保存用户内容授权，不建立数据库、任务队列或提交器。

## Device 生命周期

`BackgroundHostConfig` 新增可选 `office_runtime`，只接收受信任的 worker 绝对路径及必填摘要。旧配置继续有效；配置本身不来自 Agent 请求，也不允许指定暂存目录。Device 在产品激活前初始化组件，验证 worker 字节，创建 `state/office-executions` 下的受保护目录，并执行一批启动恢复。配置错误返回独立组件错误，不能静默回退成已配置成功。

暂存目录必须是实际目录，Unix 权限为 0700，拒绝符号链接；Windows 分支拒绝 reparse point，但尚无 Windows Device 运行验收。计算参数暂使用当前已验证产品 profile：一个共享计算名额、120 秒导出期限、最多 1024 个材料身份、单材料 128 MiB、单集合 512 MiB。这些限制不等于 RSS、整个作业磁盘额度或完整产品性能承诺。

启动后的恢复接入原 `private_workers::retention`，不创建另一个定时任务。每次恢复最多回收 32 个执行目录；OS 参与者锁继续独立保护活跃 worker 与候选读取。registry 竞争返回可延期结果，真实 I/O 错误保留失败语义。未知文件、旧非受管目录和正式 Content Store 数据不属于此清理入口。

暂存路径不进入产品持久 checkpoint。可恢复候选仍先把交付文件转入原 Content Store，重读验证后再由原事务提交；后续 Artifact 恢复依赖保存的 ContentRefs。清理死进程暂存不会被当成业务 Tool 成功、失败或取消，也不撤销正式内容引用。

维护使用独立单名额，避免取消的调用仍在 blocking executor 排队时再次无限入队。取消在排队阶段阻止删除，实际操作开始后直到 blocking closure 结束才释放名额；已进入系统调用的文件操作不承诺可即时中断。产品关闭取消原维护 future，计算和候选仍沿用各自已有所有权。

## 验证状态

宿主专项 3 项与 Device 生命周期专项 3 项通过，设备进程入口 9 项测试通过；宿主的一个 ignored 项是被父测试显式执行的子进程入口。Device 测试使用实际存储并调用生产初始化和同一维护函数，没有启动需要账号凭据的完整后台宿主进程。

6 项真实导出准备、3 项真实提交和原 SQLite→SDK→worker→SQLite 回读已通过。原创作回归 60 项、内容存储回归 41 项通过；创作回归中的 9 项新 worker 测试均已另外显式执行，3 项旧外部环境测试仍跳过。存储回归中两个 ignored 项是已由父测试调用的子进程入口，一个旧版外部数据库测试仍未执行。没有运行完整产品测试套件。

两个生产库严格 Clippy 和全工作区 all-targets 编译通过，后者保留两个未改动测试文件中的 10 条未使用导入告警。准备脚本 15 项测试、固定 SDK/worker 复验以及所改文件的格式和空白检查通过。库与 Runtime 检查绑定相同公共输入集合；后续补齐的进程入口由最终 binary 测试和全工作区编译单独绑定，不把较早库测试说成已覆盖尚未修改的 binary。

新 worker 的回执不能与旧 worker 的整份回执混用。两页交付的 8 个资产保持旧版字节，其中包括原生 PPTX 和两页 PNG；4 份元数据变化精确限定在 renderer/settings 身份、预览证据 `/1`→`/3` 的计划身份以及相应证据引用。产品结果与先前独立 SDK 产生的同版本 12 份资产逐字节一致；第二版实际 PPTX 标题为修订值。五类声明继续只有结构为 passed。

验证保留首次选错旧 worker 回执基准的失败记录，没有移除全回执相等断言；核对上述差异后改用已封存的匹配 SDK 基准重跑。全工作区检查另发现 Device binary 遗漏新增错误类型的映射，补齐独立启动错误身份与测试后重跑通过。

源码摘要、实际交付文件、命令日志、匹配 SDK 基准与旧版逐项差异见[阶段证据](../reviews/evidence/2026-09-27-product-execution-spool-verification.json)。该证据使用固定 SDK 构建输入，不为核心工作区同期的其他语义/渲染改动提供新验收。

## 未完成边界

该接入准备了实际宿主资源和维护入口，没有启用新 Agent 工具或切换旧 PPT 路由。正式工具仍须把 host 工厂、原 Invocation provider、准备器和原提交事务组合起来；不能把工厂存在视为用户链路已经完成。

业务内容终态回收、Invocation 账户退役、共享 Home/PostgreSQL、Viewer/Player、历史文稿迁移、完整高级内容、Office/WPS 编辑往返、跨平台运行、完整性能/安装包以及 P00/E0–E3 仍在实施。物理暂存恢复不替代这些验收。
