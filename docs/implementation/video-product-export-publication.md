# 产品正式视频发布

2026-10-08。产品提交 `831847b47b7ddc69932c6ce33b023b5cee442923`。Musterwork 已实现手动导出的内部原子发布端口，接续[持久候选](video-product-export-content.md)。计算、产品任务和文件权限继续各归原所有者；本增量没有修改 Office 核心、SDK/worker/PPT pins、v0.1.0 稳定标签或对外发行。

## 实现

发布先检查当前原尝试、完整 verifying 进度和固定输入，再经原内容 owner 有界读取整个候选，重新核对长度/SHA-256、来源、取消与当前 writer。慢内容读取不占 SQLite writer，也不把整个视频装进内存。

最终原写事务重新验证工程与真实目标会话权限，通过原 Artifact mutation 和会话 Ledger 同时写入正式 Video Artifact/版本、Created/VersionCommitted、不可变发布回执、执行尝试结束与任务 Completed/outbox。手动导出使用 Product 来源与真实用户 Participant，不制造 Agent Run。Tool 来源必须以后接入原 Invocation/journal/liveness，不能借用手动端口。

Schema 203 保留原历史并增加发布回执与 Completed 的双向延迟外键。数据库不能单独提交“已完成但无发布”或“已有发布但任务仍运行”。重复和并发请求返回同一正式文件；恢复旧候选保留旧 candidate-attempt/ContentId，实际完成尝试另行记录。

完成后通过原 FileReference/FileQuery 读取视频；旧 export attempt 与 ready 元数据都不是文件读取权限。重启后可读取历史发布回执，无需旧执行者或原素材。发布前的隐私删除会阻止发布；正式成片是独立内容，实际访问继续接受原文件授权和内容删除/校验规则。

## 证据与交付边界

测试覆盖并发幂等、冻结版本跨后继编辑、三个提交阶段故障全部回滚、取消与会话访问撤销、素材/候选删除及物理损坏、旧 writer 拒绝、新尝试选择旧候选、重开数据库后的完成回执，以及两个不完整事务的数据库拒绝。真实 schema 202 的已有任务/候选/outbox 迁移保留，迁移末尾失败完整回滚。

原 Ledger 字节经过原 protobuf decoder、共享 ProductProjectionReducer 和 ConversationFileCatalog，生成正式 Artifact FileReference，解析到原内容；投影与独立文件目录不产生重复文件，退出会话后旧引用失效。这些事务用例使用明确标记的不透明媒体 fixture，实际编码/独立解码另行回归，不能因此宣称正常登录产品全链路验收已通过。

本增量 79 项不同测试通过：57 项内容存储、7 项原任务、7 项发布与文件投影、4 项迁移、4 项真实 Native 导出。3 项既有特殊 fixture 的直接 ignored 不计通过；两个 crash helper 由父测试调用，外部旧版数据库直接验收未运行。生产库 strict Clippy、整个 Device 装配编译和真实候选独立全片解码通过；210 项原产品 WIP 的字节、完整 diff 和展开状态保留。

产品文档 `docs/architecture/native-video-export-publication.md`；版本、源码和验证摘要见 `.codex-work/video-export-publication-report.json`。

后续[原生执行与恢复](video-product-export-execution.md)已连接真实编码/保存/提交及 resident 装配。原 Agent Invocation 归属、公共 RPC/工具版本、消息下文件卡片和右侧播放/下载仍需继续接线。长片/大对象治理、Browser 及完整生成/Admin/计费/编辑验收目标保持不变。
