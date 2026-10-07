# 产品原生画面预览接入

2026-10-07 · 产品提交 `4b93c4fac50c268bd168b9fd90487f13a63f4565`。该增量接通真实原生计算与现有桌面右侧工程面板；分层验证通过，完整产品登录端到端与音频播放尚未验收。

## 依赖与职责

继续消费视频 SDK `0.1.0-video.20261007.3`，源码 `dd64f86b2866eb1e8179e58044a7355ec1d7909b`、manifest `dab130f7cba1df231acb39603465479bb3476e1a54759ad9af90bad265f99c14`；worker profile 5，manifest `4a4b194f09fdf2893b749dd5d3c4ba8ad54a1891a3648945341192522ab66fb3`。本轮没有修改内核算法、公开组件发行或稳定版标签。

Office 的工程编译、精确索引、源采样及共享 CPU 合成生成画面；产品负责原工程权限、当前版本校验、解码进程所有权、调度、双向流及 UI。没有假造 Agent Invocation、另一份工程存储或产品私有媒体时间算法。

## 实际行为

- 原 SQLite reader fence、Actor/workspace、head pin、authority epoch 和未删除资源身份在打开、计算及发布时复核；空闲每 15 秒复核。历史回执不能重新授予已删除素材访问权。
- 一个 resident blocking actor 持有固定 SDK 工程和当前源，独立于原编辑命令额度；取消后等待真实 worker 回收才释放 permit。连续帧复用采样器，seek 仍由 SDK 处理。
- 本地协议 minor 12。Runtime → Electron → renderer 采用二进制 RGBA、递增 generation、最新定位覆盖及一帧 ACK 背压；不宣称零拷贝。输出最大 4 MiB，1,048,576 pixels。
- 原右侧工程面板显示画面、上一帧、画面播放/暂停和下一帧；保持工程比例，旧定位结果不得覆盖新画面。切换工程版本立即清屏并重开。当前明确是无音频的画面预览。

## 验证

| 证据 | 实跑结果与边界 |
| --- | --- |
| 真实固定 SDK + SQLite 工程预览 | 4 项；当前版本、权限、缓存素材删除、取消、额度与关闭 |
| 实际 VFR/B-frame 素材 + 固定 worker | 1 项显式 opt-in 执行；7 帧、连续及随机定位、同位置字节相等、中途取消和重开；AAC 存在不代表播放音频 |
| 本地 transport | 43 项全库测试；包含预览认证、1000 次定位合并、ACK 及物理 drain |
| SQLite 工程、Electron、共享编辑器 | 分别 10、48、15 项；Electron 通过真实 UDS，但服务器是明确的受控 fixture |
| 完整协议门禁 | 14 项协议与 1 项 Rust/Python gRPC 跨语言测试；新消息锁及生成描述更新 |
| 实际 Chrome 154.0.8037.98 | 共享 Vue 组件，960×720 raster 保持 4:3，旧帧丢弃、播放背压、三次打开/关闭、重试与清屏；transport 是受控 fixture |
| 静态门禁 | Desktop/共享 Vue 类型检查、前端边界、四个生产库 `--lib` strict Clippy 通过 |

合计 136 项测试，日志和源摘要记录在产品忽略目录 `.codex-work/video-preview-report.json`。不将分层测试等同真实登录产品整链路。产品保留原有 210 条展开文件状态；仅提交本次增量。二进制协议描述从 HEAD 协议加本次 local_video 生成并验证只有该文件改变，工作树原有其他协议变化仍保留。

## 尚未完成

本轮不解除完整验收的以下欠项：真实登录桌面面板整链路、手动素材导入 UI、图片/音频精确资源、音频设备时钟/音画同步、Agent 原调用的图像结果、冻结版本导出和成片卡片/下载、浏览器媒体、完整多轨/GPU/4K 与长时段性能。单驻留源能保证资源有界，但多轨交错需进一步缓存调度，不能据此宣称最终性能已达标。

总体状态继续为目标执行中，见[实施总表](video-editing-progress.md)与[完整验收](../design/video-editing/acceptance.md)。
