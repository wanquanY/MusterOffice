# 产品导出持久生命周期

2026-10-08。产品提交 `429fda70ac9a039392388857a4edec9bd7c0523c`。Musterwork 在原 Runtime/SQLite 中增加冻结导出意图、执行尝试、进度、取消、事件重放和跨工程启动恢复。接续[真实导出候选](video-product-export.md)，本增量没有改变 Office SDK/worker/PPT pin、稳定标签或内核职责。

## 归属与实现

导出意图固定私有用户、工作区、工程版本/摘要、目标会话、SDK/worker manifest、可信适配器的有界配方与输出额度。相同原 CommandId 只能代表同一个意图；硬件重新编码可能产生不同字节，所以一次执行尝试有自己的身份，不能将旧半成品身份复用于新输出。

生命周期复用原 Runtime writer generation；没有第二套租约。单意图最多一个活动尝试，结束的尝试记录保留。失败/中断后重试使用新尝试 ID，进度重新开始，意图不变。产品 admission 拒绝 Tool 来源；Agent 必须经原 Invocation/journal 事务接入，不能伪造 Agent 活动或绕过其权限。

开始和推进检查精确历史版本及完整存活资源。后继编辑可以继续，素材隐私删除仍阻止导出。查看与取消不要求媒体仍可读取，以免删除后无法停止。活动取消先记为 cancelRequested，物理执行者退出后才提交 cancelled；已保存的取消优先于之后到来的失败结果。

SQLite schema 201 将状态、尝试结束与 outbox 放在同一短事务中。Runtime 真正 open 时，在原 writer generation 前进之后、Ready 之前处理全部旧尝试，包括未再次打开的工程：活动变 interrupted、已请求取消变 cancelled，排队意图保留。旧进程读写失效。事件按工程隔离并有界分页，状态重复不增加事件。

## 证据

8 项新增实际 SQLite/迁移测试通过，覆盖：原 Command 幂等与完整输入绑定；跨用户/租户/工作区隔离；并发唯一尝试；单调进度；取消/失败竞争；跨工程真实 store 重开；旧 writer 和旧 attempt 失效；新尝试重试；后继编辑与隐私删除；开始/停止 outbox 故障原子回滚；事件分页；记录规范和限额；历史 schema 200 迁移及 DDL 全量回滚。

原工程 12 项、PCM 9 项回归通过，共 29 项定向测试，无 ignored。Runtime Artifact/SQLite 生产库 strict Clippy 与整个 Device 装配编译通过。元数据 fixture 用于存储边界验证，不能冒充实际视频计算或全应用重启验收。

产品设计 `docs/architecture/native-video-export-tasks.md`，证据 `.codex-work/video-export-task-report.json`。原 210 个产品 WIP 文件、完整 diff 和展开状态逐项保留。

## 下一接线与未完成边界

当前只有持久生命周期存储接入 Runtime 启动恢复；可信配方生产/恢复、resident 调度、原候选计算进度、Agent/RPC/UI 尚待接入。存储不开放任意 Completed 状态，候选也不能变成虚假成功回执。

接下来需要原 ProductCommand/Invocation 内容账户按尝试预留输出、流式持久化已验证候选、恢复完整候选、原子提交 Artifact/文件引用/任务回执，再接文件卡片、普通右侧播放器与下载。实际取消、重启、后台会话与完整导出端到端另行验证。Browser、图片/多容器、完整编辑、生成/Admin/计费及全部最终验收目标保持不变。
