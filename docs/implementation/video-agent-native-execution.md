# 原 Agent 调用与原生编码的实际连接

2026-10-08。产品提交 `951d543beebeeffe8578062fc7e692c190c77d19`。接续[原运行归属与原子提交](video-product-export-tool.md)。Musterwork 的可信 Agent 执行端口已实际串联固定 SDK 工程、真实 VFR/H.264/AAC 素材、原生编码、候选持久化及原 Artifact/Tool 原子提交。下文前半记录该可信端口增量；后继产品提交 `a68693cfa` 已完成正式工具/技能和原物理执行器装配，见本页最后一节。正常对话 UI 尚未验收，整个目标仍为执行中。

## 责任和结果

产品共享已有手动导出的计算、进度观察、恢复验证和物理回收链，按 Product 或原 Agent Invocation 选择对应原权限和提交端口。原 Run 继续持有唯一 Tool 生命周期；没有给 Office 内核增加任务平台、权限、文件目录或业务 UI。

Agent 导出准入与启动在一个 SQLite writer 事务内完成，原调用已停止或绑定失败不会留下新的待执行记录。编码后仅预写小型工具结果 JSON；视频保留原导出内容身份。正式文件、工具成功、原 liveness terminal 与 Completed 在同一原事务完成。文件通过结果中的原 FileReference 和原有效会话投影读取；不为了兼容而把 Runtime 文件塞进仅接收 Product/导入来源的补充目录。

进度只报告真实工作推进。取消先停止实际编码并等待清理；总任务名额保留至保存/发布结束。完整旧候选可以在同一仍有效 Invocation 的新导出尝试中恢复，恢复仍核验固定配置、工程和实际字节，不重新编码。

## 证据范围

真实用例使用已准入的 `video_author@3` 创建/编辑工程、检查并导入真实音视频描述，再通过可信导出端口获得正式文件。验证正式 FileReference 可在原 `ConversationFileCatalog` 解析，删除原素材后成片仍独立可读。该用例的素材授权是受控夹具，导出暂借原 author Invocation 验证 Tool 所有权；不声称新的公开 export 合同或模型对话已完成。

另验证容量拒绝、无效预检、实际编码中取消、发布等待仍占用总名额、中断后复用旧候选、原租约失效不得发布，以及静止进度轮询不产生假活动。持久层以 owner 绑定失败注入证明准入/尝试/outbox 整体回滚。原手动导出与工程编辑/撤销回归继续通过。

独立 FFmpeg 整片解码检查真实正式文件的 75 帧、128×96、3 秒、48 kHz 双声道和 144000 authored samples，AAC 解码 padding 独立计数。本轮 75 项不同测试通过（36 项 Agent/工程/导出组合、30 项原 SQLite、9 项手动实际导出），Device 编译与生产库 strict Clippy 通过；真实 Agent 正式文件、手动正式文件及候选各自通过独立整片解码。精确命令、结果、源码及日志摘要在产品 `.codex-work/video-agent-execution-report.json`；设计说明在产品 `docs/architecture/native-video-agent-execution.md`。

本增量没有修改 Office 计算代码、固定 SDK/worker/PPT pin、稳定 `v0.1.0` 或任何外部发行。下一步是原 CooperativeExecution、不可变公开工具/技能以及正常消息卡片/右侧播放器的产品连接。长素材、大对象、Browser 媒体/导出、全部编辑和生成/Admin/计费的完整验收仍须完成。


## 正式工具和原执行器（后继增量）

产品提交 `a68693cfa` 增加 `artifact.video.export@1` / `video_export`，device.v72 追加目录、技能 v4 由 skill release v55 激活。模型只提交精确工程版本、标题及常用输出选项；Actor、原 Invocation、导出身份、writer、SDK/worker、预算和文件路径都由宿主持有。仅更新 musteroffice 开发验收 profile；稳定 active heads 与全部历史工具/技能保持原值。

Device 按每次原调用建立结果元数据内容权限；Agent 与手动驻留导出共享真实原生执行器和容量，原后台不接管 Tool。原 `CooperativeExecution` 报告实际进度，保留首次停止原因：用户取消持久取消，进程退出等保留 Interrupted。发布被阻塞时仍持有物理资源，停止后等待原操作退出；成功仍由原 Artifact/Tool 原子事务提交。

实际公开合同测试走原 `MusterworkToolExecutor`、真实 Dispatcher、SQLite liveness reactor、固定原生编码器：完成成片与 FileReference；占满计算额度后重放相同结果、不重新编码；编码后发布前的原用户取消阻止 Artifact 发布并在退出后释放资源。素材授权使用受控文件夹具，没有真实模型推理。23 项不同测试通过（14 项原生工程/导出、4 项目录、2 项停止原因、3 项 Device 装配），生产库 strict Clippy 通过。独立整片解码验证75帧、3秒、双声道48 kHz与原音频对照，非正常对话界面或物理播放验收。

证据保存在产品 `.codex-work/video-public-export-report.json`，详细合同在 `docs/architecture/native-video-public-export.md`。本次没有修改 Office 计算源码和SDK/worker/PPT固定版本，也没有修改稳定标签或外部发布。继续实施手动导出服务、简洁文件卡片/右侧播放器、Browser 本地媒体/导出、全部编辑与生成/Admin/计费完整验收。
