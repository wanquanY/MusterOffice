# 浏览器导出的共享时钟与编码画面

2026-10-08。状态：原生/浏览器共享导出调度、独立规划 WASM、GPU I420 编码输入已实现并验证。浏览器编码器、MP4 封装、持久任务及产品下载尚未接通，不能以本记录作为浏览器导出或整个视频方案验收。

## 计算与宿主边界

`mo-video-export::ExportPlan` 继续固定不可变工程、视频时钟、有限帧数、音频采样数和输出策略。新增 `ExportSchedule` 提取原生会话原有的整数音画交错逻辑；`ExportSession` 已实际使用它，浏览器独立 `mo-video-export-wasm` 也从同一个计划构造调度。没有在产品或 JS 中增加另一套按秒累加的时间线。

`next()` 返回下一帧、下一块音频或完成。等待素材/编码额度时重复读取不前进；编码器接受原始输入后才能 `accept(work)`。确认必须与当前完整工作项相等，伪造时长、跳步、重复、乱序、额外字段及确认完成状态均拒绝。调度拥有计算游标，不是幂等业务任务或已发布文件；持久请求、恢复、取消、资源权限及候选发布仍由产品 Host 负责。

独立 WASM 入口 `ProjectExport.open(document, expectedDigest, options)` 先编译并校验工程摘要。选项明确声明码率、GOP、mono/stereo/omit、音频分块、背景及可选 HDR 策略；非法字段、预算、空工程和不能表示的结束时间在创建编码器前失败。所有 i64 时间/采样数通过十进制字符串传输，不能隐式转换为有精度损失的 JS number。

新增 `h264WebCodecsAacMp4` 是显式浏览器目标档案，不能冒充实际能力检测成功。Native 编码器既不公布该档案，也不将它替换为 VideoToolbox；收到该档案返回 unsupported，未写候选字节。未来 Browser 适配器必须对实际 codec 配置完成平台探测。

## 精确时间与 codec 时间不同

[WebCodecs](https://www.w3.org/TR/webcodecs/) 的输入使用整数微秒；工程和 MP4 轨道仍需保留精确有理时间。

- 共享核心分别把每个**绝对**起止端点以 ties-to-even 量化为微秒，得到 transport timestamp/duration，并返回起止残差。
- 不重复累加舍入后的帧时长；音频分块方式不改变视频位置或工程终点。
- 调度工作项保留原始 timescale、PTS、duration；编码输出关联回原工作项以后，封装与校验使用精确时间。
- 不足一微秒且无法保持正时长的末帧明确拒绝；不会为满足浏览器 API 而擅自补长。Native 的精确时间能力不受这项浏览器限制影响。
- 音频仍以完整采样向上取整；原 `audioTail` 保留相对工程末端的正残差，不能用它掩盖 AAC 编码延迟或 padding。

规划组件没有编码器、存储、GPU 或业务依赖。交付工具记录 Cargo 依赖闭包、源码摘要、WASM、声明及完整清单，要求显式编译模块和新 Worker；本地开发交付保持 `releaseCleared: false`。

## GPU I420 输出

`mo-video-gpu-plan` 新增共享 WGSL 和 176 字节参数 ABI，参数来自已有 `SdrTransform`、原生色域矩阵及 `i420_chroma_weights`。`video-visual` 公共组件公开 shader/参数；浏览器执行器只核验 ABI、分配缓冲和执行程序，不重写颜色公式。

三个计算 pass 依次完成线性背景合成与电视信号转换、水平色度低通、垂直色度低通及字节封装。遵循既有 BT.709 / 理想 BT.1886、limited range、水平 left-cosited/垂直居中及 Lanczos2 规则；HDR 必须提供明确的 tone-map。它不经过 canvas/sRGB 隐式转换。

每个输出 u32 只有一个写入者，三个平面每行按 4 字节对齐，padding 为零。`readI420` 返回调用方拥有的字节和明确 plane layout，可以直接构造 I420 VideoFrame。输入能力关闭后，正在执行的转换仍持有自己的引用；成功、错误、部分分配失败和取消均释放临时 GPU 资源。

本实现有 GPU readback，不宣称零拷贝。除输入线性图外，电视信号缓冲为 16 bytes/pixel、水平色度为 4 bytes/pixel，还有输出与 staging 两份按行对齐的 I420 及 176 字节 uniform；全部计入执行器额度。返回的 CPU 字节、驱动/codec 内存另由宿主计账。未测长片性能，不以正确性测试作为吞吐或最终体积承诺。

## 实际证据

| 检查 | 已观察结果 | 边界 |
| --- | --- | --- |
| Rust 核心回归 | 导出、GPU 计划与渲染共 41 项通过；1 项既有真实媒体用例默认 ignored 不计入 | 包级验证 |
| 新调度测试 | 5 项，包括一小时 30000/1001 fps、44.1 kHz、不同音频分块、精确尾帧、确认拒绝和残差重建 | 没有声称真实一小时视频编码 |
| 公共 WASM | Node 实际独立 Worker，4 个工程、239 个步骤及微秒残差与原生完全一致；摘要、参数、未知确认字段、重放及提前拒绝检查通过 | 规划计算，不是浏览器编码 |
| 真实 Chrome GPU | Chrome 154.0.8037.98，Apple / metal-3 非软件设备；25 组 SDR/HDR/alpha/背景/高频边界/不同尺寸，共 109080 个有效样本与 CPU 逐字节一致，360 个 padding 字节为零 | 合成测试输入，不代表所有素材与色彩档案 |
| VideoFrame | 实际 I420 VideoFrame 构造与原格式 copyTo 后有效样本不变 | 尚未进入 VideoEncoder |
| 既有浏览器颜色 | 同一视觉交付的 8 组裁切/方向/alpha/sRGB 显示和 44 组原始平面/位深/HDR 回归通过 | 原始颜色门禁，未扩大为整播放器验收 |
| 资源/协议错误 | 伪造尺寸/stride/offset/NaN 参数、HDR 缺策略、奇数尺寸拒绝；部分分配失败、输入提前关闭及取消后资源账本正确回收 | 不是浏览器进程硬内存上限或完整设备恢复 |
| Native 编码 | 6 项默认测试通过；另实际导出 6 个 MP4，包含 VideoToolbox H.264/AAC、FFV1/PCM、48/44.1 kHz 与无音轨，经独立 ffprobe/FFmpeg 校验通过 | 图形与程序 PCM 工程，沿用该门禁原有范围 |
| 静态检查 | 相关 5 个 Rust 包 all-targets strict Clippy、WASM、TS strict 构建通过 | 不代表全工作区与产品验收 |

开发过程证据位于 `.codex-work/video-export-core-regression-fixed.log`、`video-export-wasm-verified-initial/report.json`、`video-browser-i420-shader-fixed/report.json`、`video-export-shared-schedule-native/report.json`。GPU 向量 SHA-256 为 `2233072d22938aabecb56f03e0d8764dad73f2425235434561fb111b86f60e5c`。构建中发现并修正 WGSL 保留词后，使用新交付目录重跑实际 GPU；保留失败记录，没有替换安装包或放宽像素阈值。

```sh
cargo test --offline --locked -p mo-video-export -p mo-video-gpu-plan -p mo-video-render
cargo run --offline --locked -p mo-video-export --example schedule_vectors > NATIVE_SCHEDULE.json
cargo run --offline --locked -p mo-video-gpu-plan --example i420_conformance > NATIVE_I420.json
python3 tools/video-export-sdk/build.py --output NEW_EXPORT_COMPONENT --bindgen PINNED_BINDGEN
node tools/verification/video-export-sdk-check.mjs NEW_EXPORT_COMPONENT NATIVE_SCHEDULE.json NEW_SCHEDULE_REPORT_DIR
python3 tools/video-visual-sdk/build.py --output NEW_VISUAL_COMPONENT --bindgen PINNED_BINDGEN
node tools/verification/video-browser-i420-check.mjs NEW_VISUAL_COMPONENT NATIVE_I420.json NEW_GPU_REPORT_DIR
python3 tools/verification/media-encode-check.py --component VERIFIED_NATIVE_COMPONENT --project --hardware --output NEW_NATIVE_REPORT_DIR
```

## 继续实现的编码与发布边界

1. 编码适配器消费共享调度、相同不可变视觉/音频快照；限制在途原始帧、PCM、packet 和候选写入，取消时真正关闭编码器及 Worker。末尾才 flush，不能用反复 flush 规避 codec 反压。
2. 审核固定 Mediabunny 封装能力。当前固定源码 `9d36fc5779b254904bc609e66f2cd80ed8390b4b` 的 ISO BMFF muxer 根据帧率倒数近似选择 video timescale，默认 movie clock 为 57600；尚无已验证的显式精确时钟入口。必须在组件责任层解决精确轨道/电影时钟、最后短帧和 AAC 延迟/padding，禁止伪造帧率元数据、修改已安装产物或以浮点秒替代作者时间。该项尚未选定和实现具体扩展，不把它写作已解决。
3. 独立回读实际输出，核对帧/样本范围、时间、颜色、track metadata、codec 延迟以及完整解码；通过前保留未发布候选。
4. 产品 Host 接入固定组件、持久导出意图和任务、OPFS 候选/配额/恢复及正式文件提交。右侧只展示对用户有用的进度、播放和下载；不能用临时 Blob URL 代替正式文件交付。

稳定 `v0.1.0` 不变。本批未修改产品源码或提升产品组件 pin；产品现有本地导入/预览继续使用先前固定组件。
