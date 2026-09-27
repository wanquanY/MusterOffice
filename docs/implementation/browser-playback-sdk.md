# 浏览器 Worker 播放接入

2026-09-27 · 在 [TS/WASM SDK](wasm-playback-sdk.md)上增加浏览器宿主示例，直接调用相同 Rust、Skia、HarfBuzz 和类型化 owner。它是接入产品可以复用的无界面 Worker 代码，不是新的播放器页面、权限服务或存储系统。完整高级内容、所有浏览器、取消性能与 Musterwork 替换验收仍未完成。

## 交付与所有权

[浏览器宿主](../../tools/playback-sdk/browser-worker.mjs)提供 `openBrowserWorker({kernel, raster, text})`，三个参数都是调用方显式编译的 `WebAssembly.Module`。默认创建包内的 dedicated module Worker，也允许产品提供自己的可信 Worker URL。宿主构建负责验证并固定整个包，浏览器从可信不可变的同源地址加载 JS；不能把“检查了 WASM 摘要”当作全部 JS 模块已经认证。

Worker 初始化完成后才返回可调用对象，单次 `call` 支持 transfer、期限和 AbortSignal；不排队、不创建持久业务任务。正常语义错误保留计划，实际组件失效、脚本错误、不可克隆消息或无法关联的回复会终止该 Worker 并永久关闭宿主对象。活动取消/超时同样关闭；已取消但尚未开始的调用不销毁健康 Worker，已完成调用移除取消监听。新计算在新 Worker 中重新准备。

Node 与浏览器的 prepare/sample/timing/advance/dispose 共用[一份示例分派](../../tools/playback-sdk/dispatch.mjs)，其调用的是 SDK，不复制文稿语义。来源与字体先 transfer 到 Worker，准备后主动 detach，首帧不依赖原数组；帧像素直接 transfer 回接入产品。来源 owner 复用准备结果，已有样本逐帧塑形调用、字体上传和 gather 复制均保持零。界面、播放时钟、媒体设备、文件取得和结果提交仍归产品。

普通浏览器路径不需要 SharedArrayBuffer、COOP/COEP 或运行时 npm 依赖。代码模块与资源显式提供，计算 Worker 禁止隐式 fetch；这不禁止宿主按自己策略加载代码/文件。接入示例与其私有消息封套不是新办公协议，也不改变 MCP/Skill/Plugin 的接入方式。

## 实际验证

[可重现驱动](../../tools/verification/browser_playback/README.md)只使用新建临时浏览器 profile、回环服务器和明确列出的自有语料；不读取现有浏览器会话。服务前校验开发包完整清单及历史输入，内存中冻结允许提供的字节。页面只运行验证，不实现产品 UI。CSP 限制同源模块、Worker 和连接，并仅放行 WASM 编译所需能力。

[本阶段证据](../reviews/evidence/2026-09-27-browser-playback-verification.json)分别记录真实 Chrome、Edge 的普通浏览器路径，以及 Chrome 隔离探针。每次均准备 4 个作者、6 个来源 owner，服务器将 71 帧完整像素及完整元数据与上轮 Native SDK 保留产物比较。不是只比较报告中的摘要；没有将三个环境重复的 213 帧写成 213 个独立语料。原生参考属于已有固定输入，本阶段没有重新执行原生引擎。

生命周期测试在真实 Rust 已进入栅格回调后触发 abort、deadline 和 close；另覆盖背压、组件真实失效、脚本加载/执行异常、消息克隆错误、错误关联、正常诊断和取消监听清理。Node 示例重构后重新消费同一包的全部 71 帧，并重验原有 Worker 生命周期。类型/合同及与变更相符的仓库检查分别记录，不将旧测试数算作新增 PPT 能力。

## 取消实测暴露的缺口

第一轮假设浏览器 terminate 返回后执行立即停止，实际失败；非隔离与隔离两次失败均保留。隔离探针通过共享计数器证实：调用者已收到取消时，500 ms 短回调仍能继续完成。随后用 6 s 长回调验证延迟强制中断，计数变化在约 2 s 附近停止，没有到达回调结尾。共享计数器仅用于测试，不加入文稿资源或公共计算合同。

这与 [HTML 标准的异步终止步骤](https://html.spec.whatwg.org/multipage/workers.html#terminate-a-worker)及 [Chromium WorkerThread 的延迟强制终止实现](https://chromium.googlesource.com/chromium/src/+/c95a6e3d4d2ccc9c493213473016f11f5b94f4d3/third_party/blink/renderer/core/workers/worker_thread.cc)相符。浏览器 API 没有可等待的退出确认；当前回调可以继续片刻，独立探针端口能观察到它的尾部。接入对象会立即屏蔽晚结果和禁止复用，但不能声称 CPU、WASM 堆或进程 RSS 同时归还。

最终测试区分逻辑取消、执行计数停止和物理内存回收，纠正的是验证中“立即停止”的错误假设，没有通过更改计算算法、跳过检查或扩大产品性能指标获得成功。约 100 ms 的探针采样和连续稳定观察不是精确退出时刻，也不是产品取消 SLA。**原设计 P95 ≤ 200 ms、强终止 ≤ 2 s 的整体门禁继续开放。** 有界 step、Rust/组件长临界区取消检查、逐引擎验证及原生受管进程收尾仍要继续实现与测量。

## 包与后续

最终开发包仍是原来三个固定 WASM 模块；本阶段没有更换 Rust/图形/文字组件或新增第三方依赖。56 份清单文件合计 **14,158,985 字节**（不含清单），gzip 归档 **4,091,056 字节**；相比上阶段分别增加 6,661 和 2,028 字节，来自接入示例及说明。两次构建的归档字节一致，开发包保持 `releaseCleared: false`。这些数值不是完整内核最终体积，也不是 Musterwork 桌面包增量。

Chromium 两个浏览器通过不代表 Safari/WebKit、Firefox、Musterwork 实际 WebView 或 Office/WPS 通过。当前阶段只接通已有采样能力；完整动画/转场、音视频、SmartArt/公式、高层跨端接口、实际 Viewer/Player 接入、历史迁移和完整替换验收按原目标继续推进。没有声明 60 FPS、主线程 long task 达标或内存下降。
