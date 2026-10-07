# 精确索引画面的合成与导出

2026-10-07 · 计算实现 `107ce8a`，独立消费者 `76a1621d4ccb963785bf6f81c77fae7465e257df`。已连通真实原生素材、工程帧计划、CPU 预览与候选导出；产品播放器、Browser 媒体设备及完整视频验收尚未完成。

## 统一时间与资源合同

`mo-time::ExactInterval` 和 `SampleSide` 统一容器索引、领域采样和合成器的区间语义：After 使用 `[start,end)`，Before 使用 `(start,end]`，倒放翻转侧而不引入 epsilon。端点保持任意有界精确分数，不能压回 `RationalTime` 的边界也不丢失。原领域 `SampleSide` 和容器 `Side/Interval` 通过重导出复用；已有公共 JSON schema 逐文件检查未改变。

`ResolvedMedia` 现在包含明确的 `ResolvedVisual`：

- `VideoFrame`：精确呈现区间、线性 BT.2020 浮点图像及显示几何。
- `VideoGap`：真实素材时间线的空白区间，没有伪造图像；合成时透出下层内容。
- `Image`：静态图片与显示几何，没有视频时间区间。

所有后端共用的资源准入会验证完整 binding、请求时间预算、素材可用区间、返回区间覆盖与边界、媒体类型和尺寸。空白同样经过这些检查，不能借“无像素”跳过身份校验。缺帧、素材不可达、解码失败和超出源范围不能转为空白。CPU 与 GPU 共用该执行器；本轮新真实媒体合成/导出检查使用 CPU，不新增 GPU 执行或性能结论。

## 共用资源适配器

`mo-video-runtime::visual_source::IndexedVisualSource` 将 `IndexedVideoSampler` 接入 `VisualResources`。预览和 `ExportSession` 消费同一接口；宿主只提供已授权、绑定同一不可变内容的 decoder 和 timeline，不复制时间映射、色彩转换或组透明度算法。

适配器保留 sampler 构造时已核验的流元数据，依据原始 PTS 缓存标准化后的画面。两棵有界有序索引提供 `O(log n)` 的查询、更新及 LRU 淘汰，没有无限增长的过期队列。定格、重复片段及 seek 后可复用相同图像；每次命中仍查询并验证时间能力，不能掩盖绑定改变或索引失败。显示几何保留像素宽高比与容器变换。

色彩策略在整个资源实例中固定：使用每帧明确声明，或使用宿主显式提供的解释。新 `InputColor::from_frame` 从实际像素布局判断 alpha；未知声明、无法解释的透明度、未知布局或交错画面仍明确失败。不会按扩展名或分辨率猜色彩。

浮点画面转换前预留字节与条目预算。原始像素缓存、标准化缓存、合成面、调用方保留的 Arc、codec/传输/元数据分别计量，不能把其中一个数字冒充进程 RSS。取消信号在适配器调用内保持已触发状态，使不透明时间/解码错误仍能保留取消类别。失败、取消或回调 panic 清空缓存并释放两个媒体能力；调用方此前持有的不可变图像仍有效。

## 实际验证

五组输入经过保留索引、原始时钟 decoder、标准化、工程帧计划、CPU 合成和真实编码。Python `Fraction` 直接扫描原始 sample/edit 表独立选择目标；原始像素 SHA-256 由独立 FFmpeg 7.1.1 解码核对。每个工程先反向拖动全部输出帧，再按顺序导出，逐帧比较预览与送入编码器的 I420，最后独立解码 FFV1 文件逐字节比较。

| 素材 | 源画面 | 正放成片 | 倒放成片 | 两个方向中的空白帧 |
| --- | ---: | ---: | ---: | ---: |
| B 帧/VFR | 77 | 73 | 73 | 0 |
| 既有 SDK 输出 | 75 | 72 | 72 | 0 |
| 开放 GOP H.264 | 120 | 96 | 96 | 0 |
| signed CTS 变体 | 77 | 73 | 73 | 0 |
| 空白/分数速率/重复/定格变体 | 77 | 41 | 41 | 24 |

共 **710 帧、10 个实际导出文件**，预览/编码输入/独立解码一致。五组测试均显式执行，0 ignored。正常结束及缓存已填充后的取消，均检查两个原生进程被物理回收。三帧标准化缓存峰值 **589,824 active bytes**，仅对应 128×96 RGBA32F 语料；不是 4K、FPS 或总内存基准。

真实矩阵使用显式 BT.709/BT.1886 limited、left chroma 解释，素材均为不透明、方形像素、单位显示变换。带 alpha 的线性叠加、非普通时基边界、空白组透明度、坏 binding/类型/区间、预算、缓存淘汰、定格复用、所有观测到的取消点及回调 panic 由额外单测覆盖。不能把这个真实语料矩阵扩大为全部格式、色彩管理或音画同步验收。

相关 9 个 crate 的默认测试共 **254 passed、6 ignored**；ignored 的专门原生/GPU用例不混入此数。另有 strict Clippy（native+GPU 编译）、默认 runtime 的 WASM 编译、全部已生成 schema 检查、SDK 包闭包检查及 6 项驱动测试通过。WASM 编译不代表浏览器执行；本轮没有运行整个顶层 media group 或远程 CI。

本地证据：`.codex-work/video-visual-time-verified/`、`video-visual-real-verified/`，以及 `video-visual-{tests-verified,clippy-03,wasm-verified,schemas-verified-02,driver-verified,sdk-verified}.log`。新版驱动 media 分组为 11 步，在索引源验证后增加实际合成/导出检查，复用同一次已核验时间证据。

## 不可变开发包

- SDK 版本 `0.1.0-video.20261007.2`，干净源码 `76a1621d4ccb963785bf6f81c77fae7465e257df`，20 个自有 registry crate；manifest `d0345e52d66a399ef47b541ef72e6f636acaffe41beace7a1e7a1fc3724d50bd`。
- 同源 macOS arm64 release worker 保持 wire profile 5，manifest `4a4b194f09fdf2893b749dd5d3c4ba8ad54a1891a3648945341192522ab66fb3`，7 个二进制合计 **8,424,952 bytes**。纯 Rust 区间归属变化不改变消息字段；C decode ABI 2 / encode ABI 1 与固定 FFmpeg 9.0.2 组件不变。
- `tools/video-sdk/check-runtime.py` 通过独立拷贝的 registry archives 创建/编辑工程，使用新 indexed 适配器预览/定位/导出 75 帧。FFV1 候选 408,469 bytes，独立解码的 1,382,400 bytes I420 全相同。另解码 144,000 AAC 样本，导出仍无音轨；不宣称音画混合验收。
- 包与 worker 路径分别为 `.codex-work/video-runtime-sdk-76a1621-02/`、`video-visual-worker-76a1621/`，消费者报告为 `video-runtime-consumer-76a1621/report.json`。构建为未公开的开发档案，`releaseCleared: false`；稳定标签 v0.1.0 保持原指向。

Musterwork worktree 提交 `dd5e1c7b6` 已固定并安装上述 20 包与同源 profile 5 worker。27 项安装测试与最终锁图下的 7 项真实素材授权/工程编辑/重开集成通过，生产库 strict Clippy 和两个发行身份核验通过；非视频 registry 的包身份及依赖边保持原值。扩大到产品 all-targets Clippy 时被既有 PPT/Office 测试中的 unwrap lint 阻止，不计作通过。产品原有 192 项未提交状态保留，未宣称干净 HEAD 的全应用验收。

接下来完善素材导入/区间描述、预览/导出 RPC、设备播放和右侧播放器。完整目标仍以[实施记录](video-editing-progress.md)和[验收矩阵](../design/video-editing/acceptance.md)为准。
