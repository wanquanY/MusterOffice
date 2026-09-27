# TS/WASM 分步播放接入

2026-09-27 · 将 [Rust 待完成帧](prepared-playback.md)接入同源 WASM/TS 播放 SDK。自建文稿与导入 PPTX 共用 `beginSample → step → take/close`；同步 sample 保留并继续回归。宿主掌握调度、取消、Worker 和结果提交，不增加业务任务、存储、权限或产品界面。

## 实现

WASM 的待完成帧直接持有 Rust 私有计算对象，准备失败携带原类型化诊断。`begin` 只允许调用一次，在同步组件回调中借用 Rust 指令和图片；组件返回前已完成自己的输入快照，不跨让出点借用 Rust 内存，也不为了分步执行增加完整图片的 JS 中间副本。这里返回的是可继续执行的组件对象，Rust 调用栈此时已经结束，没有把 Promise 放进同步回调。

`complete_render` 消费待完成帧，共用原有有界 JS 像素读取和 Rust 完成校验。外部返回的字段读取失败、错误类型、状态、长度、预乘通道与组件故障均保留诊断及失效处理标记。Rust 实例、计划、generation 和 dispose 校验继续生效；公开 session 字符串不能代替实例身份。

TS `PlaybackExecution` 独占其播放 owner，直至 take 或 close。活跃时 sample、advance、dispose、close 和第二个 beginSample 均拒绝；step/take 中的回调也不能重入。提前 take 或非法 workUnits 不会丢失句柄，正常取消允许健康 owner 再采样。完成、提前关闭和错误路径分别释放组件任务、Rust 帧和 owner 租借；清理错误与原错误一起报告。组件失效时释放 owner 并标记 raster 失效，由宿主销毁隔离单元，不能在同一个失败 Worker 中更换组件继续工作。

Node/浏览器开发宿主增加 begin/step/take/cancelSample 消息，便于独立接入与检查。cancelSample 在步骤之间返回清理确认，dispose 先释放活跃帧。产品可直接在自己的 Worker 中驱动同一 SDK，自行选择让出粒度，避免每步 IPC。这个示例消息封装不作为 MCP 的替代协议，也不是持久任务系统。使用方式见 [SDK README](../../packages/playback-client/README.md)。

## 验证范围

- Node 真实 Worker 分步完成 10 个 owner、71 帧；完整像素与原参考逐字节相同，完整元数据另外与原生 SDK 参考核对。
- Chrome 隔离环境分步完成同一 71 帧及完整元数据对照，另外在 10 个 owner 的三个边界完成 30 次协作取消，再继续真实采样；没有将逻辑关闭当作物理内存回收。
- 26 个独立真实 WASM Worker 检查作者/来源两种路径的提前关闭、准备失败、异主完成、generation/dispose 变化，以及注入的状态、字段读取、类型、长度、通道、失败载荷错误。每个故障用例使用新的 Worker，验证后由父宿主终止；不是 26 组新增绘图语料。
- 19 项 TS 生命周期测试、15 项 Rust 播放 owner 测试、15 步仓库门禁及独立包类型消费分别验证。范围和原有忽略项以[阶段证据](../reviews/evidence/2026-09-27-wasm-stepped-playback-verification.json)为准。

`mo-wasm` 增加对已有固定版本 serde_json 的直接依赖，用于已有诊断的序列化；没有新增第三方包或更新版本。Skia/HarfBuzz 复用已固定的组件，开发包仍为 releaseCleared=false。

## 未完成项

`step` 的工作单元没有时间上界；准备、单个 Skia primitive、快照、完成校验和清理仍可能同步执行。当前取消确认说明该显式帧已结束，不能据此承诺 200 ms P95、2 s 强停、60 FPS 或操作系统 RSS 回落。跨引擎性能、Musterwork 实际 WebView、完整高级内容和 Office/WPS 互操作继续按原目标实施，尚不能宣布完整替换。
