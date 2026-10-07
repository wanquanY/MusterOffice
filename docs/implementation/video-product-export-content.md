# 产品候选持久保存与恢复

2026-10-08。产品提交 `aaaf79aec02c3b738e3b6df9ea6ce2b522e69519` 接通原生真实成片候选、原 Runtime 内容账户与跨 owner 重开恢复。继[导出生命周期](video-product-export-tasks.md)继续实现，Office 计算核心仍不持有产品任务或数据库；SDK/worker/PPT pins 与 v0.1.0 稳定基线未改变。

## 本次实现

Native adapter 保存有界规范导出配方，固定 codec、码率/GOP、声音模式、背景/采样/tone map 和计算预算。恢复需匹配原 SDK/worker、完整配方与输出额度，不能换当前默认值；不持久化本机目录。

真正经过容器、时钟、流和抽样解码检查的 Native candidate 才通过桥接生成验证记录。记录绑定原意图、实际编码统计、检查结果与成片长度/SHA-256；写入前后再次检查取消、固定来源和 writer。

SQLite schema 202 在原内容 owner 下按导出尝试预留唯一输出，长度/hash/验证记录在消费字节前固定。同尝试改变编码字节或验证记录会被拒绝；重新编码必须取得新尝试和输出身份。实际字节复用原 material lease、流式校验与 ready 元数据事务，不新建文件系统或整片内存缓存。每块输入/输出前后复核原任务、来源和取消；普通上传去重及其他输出账户不能借用保留身份。

启动恢复结束旧尝试后，新尝试可以取回同一意图的旧完整候选，并保留旧 source-attempt/ContentId；不会恢复旧执行令牌或续写半个 MP4。隐私删除、候选删除、取消或物理文件损坏均阻止读取成功。部分 sink 必须整体丢弃，候选 ready 不代表 Artifact/消息已经发布。

## 实际验证

69 项不同测试通过：57 项内容存储（含 7 项新增候选边界）、7 项原任务、1 项新迁移、4 项真实视频导出。去除了任务过滤器重复命中的 7 项候选测试。3 项既有特殊 fixture 的 ignored 不计通过；两个 crash helper 由父测试启动，另一个需要外部旧版数据库的直接验收未运行。生产库 strict Clippy 和整个 Device 装配编译通过。

真实测试从固定 VFR/AAC 原素材产生硬件 H.264/AAC，保存候选后释放匿名文件与旧 DB owners，经正式 product startup 重新取得 process lease，然后以新的导出尝试读回相同 SHA-256/长度的成片，不再编码。恢复后删除原素材会阻止再次读取。

独立 FFmpeg/ffprobe 7.1.1 全片解码恢复文件：75 帧、128×96、3 秒、48 kHz 双声道，逐帧 PTS 为 i/25，原声对照 SNR 约 41.7/42.0 dB；144,000 authored samples 与 144,384 含尾 padding 的解码 samples 分开记录。不是物理 A/V、长片/4K 或完整 UI 验收。

产品设计 `docs/architecture/native-video-export-content.md`，证据 `.codex-work/video-export-content-report.json`；原 210 个 WIP 文件、完整 diff 和展开状态保持不变。

## 仍需接通

resident 调度与原计算 watch 进度、原 Agent Invocation 归属、原子 Artifact/文件引用/Completed、一次性文件卡片、RPC/新工具版本及 UI 尚未交付。当前沿用原内容系统 `264 MiB + 12 bytes` 单对象上限；长片需在原系统分离流式与内存读取额度，不能直接增大所有 Vec 上限。生成/Admin/计费、Browser、完整编辑和全部本地验收目标保持不变。
