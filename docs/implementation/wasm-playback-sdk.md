# TS/WASM 播放 SDK

2026-09-27 · [K2](kernel-boundary-roadmap.md) 已把原生 SDK 对应的已有作者/来源播放能力接入类型化 TS 接口和可搬迁的离线开发包。独立 Node Worker 使用真实 Rust、Skia 和 HarfBuzz WASM 完成采样；本阶段没有修改计算核心、组件、合同或 Musterwork。完整 PPT、浏览器实机与产品替换验收仍未完成。

后续进展单独记录于[浏览器 Worker 接入](browser-playback-sdk.md)，包含 Chrome/Edge 实测及延迟强制终止缺口；本页保留此前 Node 阶段的固定验证范围和体积记录。

## 接入与职责

[公共 facade](../../packages/playback-client/src/index.ts) 的 `WasmPlayback.prepareAuthor` / `prepareSource` 消费同源生成的准备请求，返回有明确所有权的作者/来源 sampler。它提供 `sample`、`timing`、`advance`、`info`、`dispose` 和 `close`。`info` 是独立副本；generation/ticks 保持精确字符串，不经过浮点整数。源码、字体和准备组件不被 TS owner 长期持有。

[完整示例与调用约束](../../packages/playback-client/README.md)说明：宿主显式提供已验证的代码、文稿、字体和事件历史，负责 Worker、播放时钟、显示、媒体设备、结果提交及持久化。连续帧直接调用 SDK，不逐帧进入 MCP/模型。SDK 不提供页面、账号、文稿库、持久业务 Job 或后台帧队列。

请求、资源、绘图 profile、颜色/alpha、像素摘要和时间计算仍由同一 Rust 核心及组件接口校验。TS 绑定可信、匹配的 WASM 导出，处理对象所有权、请求/响应关联和帧尺寸；它不是另外一套不可信网络 JSON 校验器，也没有复制内核算法。来源与作者仍各有已实现的内容范围，不把来源展平为图片或静默降级。

## 生命周期与失败

TS 接口同步执行，应运行在接入方的计算 Worker。没有用 AbortSignal 或计时器伪装成可在同一 JS 线程中抢占同步 WASM；需要硬取消/期限时，宿主终止实际 Worker，等待终止并拒绝晚结果，再在新 Worker 中重新准备。包内的 [Node 宿主示例](../../tools/playback-sdk/example.mjs)演示单调用背压、二进制 transfer 和终止后才拒绝等待者，它的消息封套只是宿主示例，不是新增办公协议。

同一 owner 在调用中拒绝回调重入，包括 sample、控制和 close。代码与资源输入只读取一次，避免宿主 accessor 在校验之后更换模块或源数组；模块初始化在读取 accessor 前领取所有权，不允许递归初始化。一个导入模块只初始化一份 Rust 运行时，失败后也需要新的 Worker。runtime 工厂可以创建额外的健康组件实例；实际 RasterComponent/ShapingComponent 失效则沿用其隔离合同，终止整个 Worker 后重新准备，宿主示例明确标记此失败为 fatal。

正常核心语义错误保留结构化诊断和不可变计划；桥接错误、异常返回或 trap 使 owner 失效并释放。dispose 即使验证失败也释放；close 幂等。每个返回帧的 Rust 分配只消费一次，元数据失败会释放尚未消费的帧；原错误与清理错误同时保留。没有依赖 GC 最终析构来保证释放，也不在 SDK 保存帧历史。

32 MiB 请求、各 128 MiB 来源/字体、各 64 MiB 元数据/像素是协议限制，不是 RSS 承诺。文本预算按 UTF-8 字节；正常小元数据无需额外编码缓冲。来源数组禁止 SharedArrayBuffer。底层 wasm-bindgen 与组件仍有必要复制，TS facade 不额外复制像素帧；不能由此宣称全链路零复制或保证 60 FPS。

## 离线开发包

[打包器](../../tools/playback-sdk/README.md)接收已生成的 wasm-bindgen web 输出及固定图形/文字组件，提供 ESM、`.d.mts`、同源合同声明、三个 WASM 模块、组件 JS、宿主示例及已有许可记录。普通接入方无需构建 C++、下载运行时 npm 依赖或创建持久库。调用方在加载前验证外部固定的清单摘要，显式编译三个代码模块；运行时不自动 fetch 或查找字体。

公共声明支持 ES2022；直接使用内部 wasm-bindgen 声明时，其生成器要求 `ESNext.Disposable` 类型库，这不外泄为普通 SDK 的要求。独立 consumer 同时检查了公共接口与原始绑定的结构类型兼容，没有使用 any 或忽略声明错误来通过。

固定模块与组件属于开发输入，清单明确 `releaseCleared: false`。本地归档不是公开发行或最终许可审计。真实包更新/宿主热重载策略、各浏览器、签名与完整分发验收继续推进。

## 验证与体积

[阶段证据](../reviews/evidence/2026-09-27-wasm-playback-sdk-verification.json)记录最终源码、模块、归档、外部消费、测试日志和历史产物身份。

- 最终 15 步门禁通过：全仓严格 Clippy、119 份 Schema、TS 类型与客户端回归、实际 Native/WASM 文稿/计算 Schema 对照等。13 项新生命周期测试和 3 项包装测试分别覆盖重入、精确时间/代次、单次读取、异常清理、重复释放、UTF-8、完整清单及拒绝覆盖。没有把旧协议客户端的 17 项回归算作本轮新能力。
- 归档解压到含空格的独立目录，ES2022 公共 consumer 和显式原始绑定 consumer 均编译通过。真实 Worker 从该包准备 4 个作者 owner、6 个来源 owner，34＋37 个旧动画帧逐字节一致；另核对上轮原生 SDK 的保留像素产物。来源和字体在传给 Worker 后、首帧前再次主动 detach，采样仍成功；逐帧塑形调用、字体上传与 gather 复制保持零。
- 真实 WASM 已进入栅格回调的 Worker 可以被实际终止，终止后没有晚帧；另一个 Worker 在无效外部组件回复后使用健康组件仍保持像素，更换另一健康实例也相同。实际图形组件主动失效后确认诊断并终止 Worker，没有把核心计划可复用解释为可以继续使用已失效的运行环境。初始化重入、重复运行时、显式释放与宿主期限失效均验证。`fetch` 在测试 Worker 内明确禁止，没有隐式代码下载。
- 两次独立构建归档完全相同。最终 53 份清单文件合计 **14,152,324 字节**（不含清单自身），gzip 归档 **4,089,028 字节**。其中 Rust WASM 10,630,592、Skia WASM 2,370,691、HarfBuzz WASM 841,468 字节；新增 facade 和初始化 JS 合计 15,530 字节，未压缩、未最小化。该记录属于固定开发包，不是 Musterwork 桌面包增量或未来完整功能的体积预测。

首次外部类型检查暴露内部生成声明的 Symbol.dispose 要求；移除公共声明对整个内部绑定的不必要依赖后，分别验证普通 ES2022 接入与带正确标准库的原始绑定接入。失败日志和中间包保留，未通过 skipLibCheck 或改写生成声明绕过检查。本阶段没有新增第三方依赖或更换组件版本。

## 剩余目标

本阶段只接通已有播放 profile。完整效果/转场/Morph、音视频同步、SmartArt/公式、其余高级可编辑内容，以及全部高层创建/导入/编辑/渲染/导出接口仍按完整一期推进。Node Worker 通过不代表浏览器 Web Worker 已验收，旧自有样本相同也不代表 Office/WPS 或商业字体语料已验收。

Musterwork 自己的 Viewer/Player、Artifact、历史迁移与最终替换链路，完整能力矩阵和同条件启动/采样/RSS/发行体积仍独立验证。本轮没有声称速度收益；持续目标保持 active，不因 SDK 包装完成而结束。
