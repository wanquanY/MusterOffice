# 产品原生预览接入

本文按阶段记录；最新声音输出状态见末尾“真实原生联合检查”。早期“尚未播放声音”仅代表当时阶段，不能作为当前状态。

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

## 工程声音传输增量

2026-10-07，产品提交 `576398432` 在同一个 pinned preview 上接入声音二进制流，消费既有 `.20261007.5` SDK；没有发布新内核、改动 worker 字节或稳定版标签。本地协商 minor 13 返回工程采样格点和双声道布局，音频块为有限 f32le，1～4096 帧、最多 32 KiB。

音频与画面各有一个最新请求、一个执行中计算、一个未确认结果及各自超时。renderer 绘制等待不能阻挡声音，音频 ACK 不能清除画面期限。暂停/定位可独立 flush 音频 generation，取消旧计算并拒绝旧结果；正常画面刷新不自动切断声音。关闭流时先取消并等待两条计算实际返回，再调用原生会话 close；音频权限与版本仍由原产品 Store 前后检查。

48 项 transport、12 项 Electron 预览/命令、42 项本地客户端/真实 UDS fixture、4 项 renderer 接收、14 项协议与 1 项 Rust/Python gRPC 检查通过，类型/协议锁/生产 strict Clippy 通过。产品 `.codex-work/video-preview-wire-report.json` 记录源和日志摘要；210 项旧 WIP 保留，协议全局描述仅提交 local_video 的增量，其余 89 个描述完全保留。分层测试不代表真实登录产品播放已验收。

**当前只接通声音接收，AudioWorklet、声卡播放和音画同步仍待实现**。UI 继续明确为画面预览。平台输出方案进一步核实：Web Audio 允许工程同采样率的单一 AudioContext，并规定整体输出连续转换到设备采样率；该系统声音输出不另建产品 DSP，不削弱 Office 的工程时间/源重采样/混音职责。不得逐块独立转换或假定 currentTime 就是已听到的位置；仍需实际设备时钟、缓冲/缺料与切换验收。[Web Audio 构造规则](https://www.w3.org/TR/webaudio/#AudioContext-constructors)


## 设备播放增量

2026-10-07，产品提交 `c0508c9ae` 接入共享 Vue 的实际 AudioWorklet 声音输出。Office SDK、worker、PPT 和 v0.1.0 稳定基线未修改。产品只负责混音后的平台输出和画面调度，素材重采样、容器映射与混音仍由固定 SDK 持有。

单 revision 输出为一个工程采样率 AudioContext、一个同源静态 worklet 和监听静音节点。双声道 ring 为 16,384 帧/128 KiB，单次请求至多 4,096 帧/32 KiB，只有 worklet 接收/丢弃后才确认原传输块。缺料不推进工程时间；定位、暂停和版本替换清除旧播放 epoch。画面按设备 getOutputTimestamp 与有界渲染区间调度，不使用 render-ahead currentTime 充当已听位置，不因画面等待阻挡音频。

播放画面采用半开时间区间的 floor，用户选定画面开始采样采用 ceil，保持分数帧率下的精确对应。Chrome 初始时间戳可能 performanceTime 为零，需等待有效设备估计；普通时间戳向后修正不能被当作设备断开。

40 项共享编辑器、4 项 desktop renderer adapter 测试、两侧类型和前端边界检查通过。真实 Chrome 154 的 dev 48 kHz 与 built 24 kHz 版本均验证播放/暂停/静音/seek/缺料/EOF/版本切换/失败重试/卸载；worklet 正式静态资源在 CSP 下实际加载。另通过导入面板浏览器回归并去除过时的“当前预览仅播放画面”提示。

产品证据为 `.codex-work/video-playback-report.json`；原 210 文件 WIP 的字节、完整 tracked diff 与展开状态逐项保持。浏览器声音图真实，但 PCM transport 是明确的测试源：尚不能声称已登录 native Runtime→Electron→UI 整链路、物理扬声器回采或长时音画同步达标。下一步继续真实素材联合验收及总方案全部剩余项，目标仍执行中。


## 真实原生联合检查

2026-10-07，产品提交 `f7e585669` 通过本地已登录 Backend、独立已签名 Runtime、原文件权限、SQLite、固定 AAC/VFR worker、双向 gRPC、生产 Electron preload/renderer adapter、共享 Vue 和 AudioWorklet 的一次联合实跑。工程实际导入原画面/原声/关联组，解码画面非空，48 kHz 声音图峰值约 0.124，暂停/定位/第 75 帧结束/重播/切换 revision 3/卸载成立；测试窗口、身份和会话清理检查通过。

联合检查发现产品导入器同时携带会话和工作区上下文，违反原 FileQuery 的单来源合同。修复由产品宿主持有：每个 FileReference 只选择命令已经拥有的原上下文，计算后按同一上下文复查；Agent sealed Run 权限不变。1 项选择回归、7 项产品事务、4 项执行器及生产 strict Clippy 通过。Office 计算/SDK/worker/PPT 与稳定标签无需修改。

证据入口为产品 `apps/desktop/quality/video-native/check.mjs`，说明 `docs/architecture/video-native-playback-acceptance.md`，本地报告 `.codex-work/video-native-joint-report.json`。这是生产组件和真实本地服务的隔离联合窗口，尚未替代完整会话页面、Agent/人工历史、物理扬声器回采、长时同步与 4K 性能验收。产品原 210 文件 WIP 保持，总目标继续执行。
