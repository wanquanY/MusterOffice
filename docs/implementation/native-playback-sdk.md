# Native SDK 播放采样

2026-09-27 · [K2](kernel-boundary-roadmap.md) 的原生播放接入层。把已有作者/来源播放计划接入独立 Rust SDK，沿用同一时间求值、页面编译和栅格 worker；没有增加产品页面、账号、文稿库或持久任务。当前仍是有限能力的开发版，完整 PPT 与 Musterwork 替换验收继续实施。

## 接入方式

`mo_embedded_sdk::playback::NativePlayback` 接收宿主明确提供的绝对 worker 路径、SHA-256 和每次调用的超时。配置时与新进程启动前检查实际程序；程序必须位于宿主管理、运行期间不被改写的目录。SDK 不查找系统字体或下载组件。

| 入口 | 内容与生命周期 |
| --- | --- |
| `Presentation::prepare_author_playback` | 从当前作者快照自动绑定 revision，指定页面、viewport、默认画笔、session 和 generation |
| `NativePlayback::prepare_author` | 显式的既有 `PlaybackPrepareRequest`，由核心校验快照和 binding |
| `NativePlayback::prepare_source` | 显式的既有 `PptxPlaybackPrepareRequest`、PPTX 与字体 `Content`；借用 reader 仅持续到准备返回 |
| `info` / `timing` | 返回准备信息或查询实际时间计划的工作计数 |
| `sample(at, history)` | 精确有理数时刻及显式事件历史，返回实际 RGBA8 帧及原核心元数据 |
| `advance(generation)` | 严格递增代次，清除旧事件历史复用状态，不重建不变页面资源 |
| `dispose` / Drop | 显式验证清洁退出，或终止并回收未释放进程 |

一次准备建立一个由调用方持有的 worker 和播放计划。通过 `&mut self` 串行采样，不在内部堆积帧队列。来源图片解码与文字塑形沿用[已有保留式计划](source-playback-sessions.md)，原始 PPTX、字体及非 Send reader 可以在第一帧之前释放；采样仍须重算本帧时间状态、放置、精度和实际绘制。文稿、viewport、字体或资源变化需要重新准备。

宿主负责播放时钟、事件采集、显示、音视频设备和自身任务生命周期。Agent 使用工具进行创作/编辑及请求结果；连续帧由产品直接调用 SDK，不需要每帧经过 MCP 或模型。这里的 session/generation 仅用于计算状态隔离，不表示用户会话、账号或持久业务任务。

SDK 的生产闭包保持纯 Rust，不链接 Skia/HarfBuzz，不引入 HTTP、SQLite 或旧持久宿主。精选 C/C++ 图形/字体组件继续留在配套 raster worker。完整源码打包增加同源 `mo-native-render` 和 `mo-kernel-api`，没有另写一套渲染算法。

## 传输、校验与失败

[保留式传输](../../crates/mo-native-worker/src/session.rs)共用现有 worker 帧格式，一个进程、一条传输线程、容量为一的输入和响应通道。调用方生产最多 64 KiB 的分块，不能先把所有资源复制进请求队列。32 MiB 请求元数据、分别 128 MiB 的来源/字体、64 MiB 响应元数据与 64 MiB 像素是协议上限，不等于整个进程的 RSS 承诺。

SDK 校验响应种类、文稿与 session/revision/generation 身份、规范化后的精确采样时间、像素尺寸/长度、实际 SHA-256、预乘 alpha 及共享栅格 profile。沿用核心的 profile 登记与有理数规范化规则，避免接入层另设一套不一致的含义。控制响应不得夹带像素，JSON 重复键、超限帧及无效 UTF-8 会终止 owner。

取消、超时、reader 错误、传输或响应不完整使 owner 永久失效，并终止、等待实际子进程退出及回收传输线程；后续调用返回 Stopped。正常语义拒绝保留结构化核心诊断，核心允许继续时可复用 owner。显式 dispose 另检查退出码和尾随输出。输入 reader 和取消回调在调用线程执行，必须及时返回；可信 worker 不得启动持有传输管道的后代进程。这些要求没有转移文件授权和进程调度的产品责任。

## 独立消费与验证

[SDK 打包说明](../../tools/sdk/README.md)与[独立示例](../../tools/sdk/playback-example/src/main.rs)提供构建/调用路径。接入方校验固定 SDK 清单，将示例复制到自己的独立目录，调整唯一 SDK 路径依赖，然后传入 worker、准备请求和采样数组。示例显式保存帧文件并标记 `productCommitted: false`；示例文件 I/O 不是内核文稿存储。

最终源码、实际 worker、外部 consumer、历史像素与回归记录见[阶段证据](../reviews/evidence/2026-09-27-native-playback-sdk-verification.json)。验证范围包含多帧复用、来源输入释放、精确时间、代次、取消/超时、阻塞读写、异常帧和子进程释放。独立消费从已打包生产源码编译，不跨仓库引用 Musterwork 私有代码。

- macOS arm64 的独立 consumer 从固定 SDK 构建，在空 PATH、独立目录运行。4 个作者 owner 的 34 帧和 6 个来源 owner 的 37 帧与历史成功样本逐字节一致；来源逐帧塑形调用、字体上传和 gather 复制为零，原静态页面元数据与播放结果保持一致。历史语料有明确的自有字体/图片范围，不代表商业文稿验收。
- 3 项真实 worker 测试覆盖作者 9 次、来源 6 次采样与旧单次入口比较、来源 reader 提前释放、超出 JavaScript 安全整数的 generation、语义拒绝后继续使用及取消失效。4 项传输测试和 4 项帧校验测试覆盖阻塞 I/O、线程/子进程释放、损坏摘要、预乘 alpha、重复键、长度/尺寸溢出及共享 profile；这些已包含在相应库回归中，不重复累计。
- 首次 25 步检查通过，含 938 项 Rust 测试、合同/TS、原生导出与三种 MCP feature 回归。随后外部历史语料发现非约分时间被 SDK 误拒绝；修正两处响应时间校验后，最终重跑受影响三库 22 项回归、严格 Clippy 和原生集成门禁。MCP 默认 17、旧宿主 18、HTTP 22 项有重叠，不作为 57 项独立测试计算。普通回归忽略的外部组件测试与实际显式执行范围在证据中分别列出。
- 最终两份 SDK 归档完全相同，538 文件、25 个同源库；与上个 SDK 的 54 个 registry 依赖及许可声明保持一致。独立 consumer 的 macOS 动态库仅系统 libiconv/libSystem；Linux musl 与 Windows GNU 通过交叉编译检查，尚无两平台实机播放证据。

开发中保留了三类失败：空输入 reader 类型错误、来源测试 viewport 选取错误、SDK 对合法资源 profile 和约分时间的误拒绝。分别修正类型/测试输入，复用核心 profile 表及时间规范化规则后完成重验；没有降低像素校验或改写旧基准。前两次 SDK 包保留为中间产物，最终固定输入为第三次构建。

## 尚未完成

本阶段暴露已有作者几何和来源资源播放 profile，没有增加完整效果、转场/Morph、音视频同步、SmartArt/公式等高级计算能力。来源采样针对显式原生 PPTX，不能声称已覆盖所有导入编辑覆盖层；不支持的内容仍按核心诊断返回，不展平为图片冒充可编辑。

Node/WASM 的高层 SDK、跨平台实机、完整公开渲染/播放操作、Musterwork 自己的 Viewer/Player 与 Artifact 接入、Office/WPS 往返及同条件性能/体积测量继续推进。保留式调用消除了逐帧重启和重复传入源包/字体，但本阶段没有测量整体速度、RSS、发行包或桌面产品体积，不能把源码归档大小外推为这些收益。用户客户端安装、公开发行和部署均未进行。
