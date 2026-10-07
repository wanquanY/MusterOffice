# 视频预览像素网格与工程坐标

2026-10-07。已实现共用计算接口并通过下述计算检查；尚未完成产品播放器接入。对应 [PR-06/13/20](../design/video-editing.md)、[技术合同 §5.2/6](../design/video-editing/technical-spec.md) 与 [AV-05/07、SYS-06](../design/video-editing/acceptance.md)。完整工作台仍以[实施总表](video-editing-progress.md)为准。

## 问题与实现

右侧面板的预览通常小于工程画布。直接把 `VideoFramePlan.canvas` 改成预览尺寸，会改变像素单位的位移、旋转中心、素材 contain 和嵌套效果；先渲染全尺寸再缩小又让面板承担不需要的合成开销。

`mo-video-render` 新增 `RasterExtent`、`FrameRenderOptions`、`render_frame_with_options` 和 `render_with_options`。`mo-video-gpu::GpuCompositor` 新增 `encode_with_options`，仍使用同一计划执行器。旧 `render_frame`、`render_with_backend` 和 GPU `encode` 以工程原尺寸委托，导出接口保持原行为。

设工程坐标到目标像素网格为 `V = diag(rasterWidth / canvasWidth, rasterHeight / canvasHeight)`：

- 色块：`V × authorEffect`，色块源范围仍为工程画布。
- 媒体：`V × authorEffect × sourcePlacement`，不修改源时刻、流、资源身份或像素宽高比/显示旋转。
- 嵌套：子组合先在目标网格合成，父组合使用 `V × authorEffect × V⁻¹`。子组合裁切在它自己的画布边界完成，不能把本已裁掉的画面重新移回来。

每层组合缓冲和 `pixel_visits` 根据目标网格计量，源解码、标准化缓存继续有各自预算。相同尺寸走原仿射矩阵路径，避免增加无意义的浮点乘法。非法画布或网格尺寸、资源不匹配、预算不足和取消均不交付部分画面。

非等比整数网格也可请求；它描述采样密度，不修改工程比例。Host 展示时仍按工程宽高比布局，面板 letterbox 由 UI 完成。缓存身份必须包含固定工程/帧、资源与解释、目标网格、采样方式、合成/后端版本及色彩输出档案。

## 精度与质量边界

空间定位沿用共用的 Q32.32 逆仿射系数、nearest-ties-even 量化与 Q30.33 像素中心求值。nearest 正好落在理想数学边界时，按量化后的坐标决定像素。真实视频测试的空间 oracle 使用整数分式独立计算该约定；时间 oracle 仍使用精确分数，两个概念没有混用。

不同网格表示不同采样结果，不承诺低分辨率图与原图逐像素相同。当前 nearest/bilinear 档案不等于高阶抗混叠或代理生成。源图标准化尚未随输出网格缩小，不能把减少的合成缓冲宣传为完整解码/RSS 节省。

GPU 新用例暴露了原 `mix` 的边界问题：接近 1 的分数经 f32 舍入后，`1-f` 会丢掉非零的微小权重，透明邻点处的非关联输出颜色因此可能与 CPU 明显不同。现在从 Q33 整数分子分别计算两个权重并加权求和，保留微小 premultiplied RGB/alpha。没有降低对照阈值或清零输出隐藏差异。GPU 计算档案提升为 `musteroffice.video.linear2020-premul-gpu-f32-v2-draft`，旧缓存不能冒充该结果；依赖版本、codec ABI 和 shader 整数覆盖规则保持原合同。

## 验证

| 检查 | 实际结果 | 边界 |
| --- | --- | --- |
| 相关六 crate 常规回归 | 54 passed，4 默认 ignored | ignored 不计为通过；下述专用矩阵另行执行 |
| 新坐标用例 | 5 passed，包含两层旋转、裁切后移动、组合透明度、旋转素材/精确时刻、不等比网格、预算及逐检查点取消 | 这 5 项已包含在 54 项中 |
| 合成预算 | 16,384² 工程在 16² 网格、两层组合时峰值临时图像 8,192 bytes、512 pixel visits | 内核逻辑计量，非 RSS/解码预算或整机性能 |
| Native/WASM | 新 6 查询（4 rendered/2 rejected）及原 9 查询逐 f32 bits、RGBA8、诊断一致 | Node WASM，非浏览器媒体验收 |
| 实际 Metal GPU | Apple M4 Max，96 个网格/变换/采样组合，RGBA8 最大差 0；22 个取消检查点通过 | 仅该设备后端，无实时帧率或零拷贝结论 |
| 原 GPU 档案回归 | 29 个组合，其中 4 个真实视频线性画面；既定误差标准通过，RGBA8 最大差 1；26 个取消检查点通过 | HDR/复杂采样继续允许原有限误差 |
| 原生媒体与导出 | 五输入、两个方向，共 1,420 张缩小预览符合独立量化空间 oracle；710 帧原尺寸预览与编码输入一致，10 个 FFV1 输出独立解码逐字节一致 | square-pixel、identity-display、显式 BT.709 limited 语料；不代表所有 codec |
| 原行为保持 | 上述 710 帧 YUV 与前次固定证据摘要相同 | 未重跑整个产品/PPT 验收 |
| 工具链 | 四相关 crate（含 conformance）Native/GPU all-targets strict Clippy、WASM 编译通过 | 不推论到产品未改动代码 |

真实输入包括 VFR/B 帧、长/open GOP、负 CTS/高 tkhd identity，以及 empty/repeat/fractional-rate/dwell edit list。两个方向各有 12 个空白帧，缩小预览保留下层背景，不能用空白掩盖缺帧。

本地证据保存在忽略目录：`video-viewport-tests-final.log`、`video-viewport-clippy-01.log`、`video-viewport-parity.json`、`video-viewport-render-parity.json`、`video-viewport-gpu-04.json`、`video-viewport-gpu-regression.json`、`video-viewport-real-02/report.json`。失败尝试保留：测试最初使用不存在的 `TimeSample::before`；GPU 插值回归最初失败并据此修正；一个报告路径使用相对路径导致落盘失败；原始空间 oracle 最初按无限精度理想坐标而非已声明 Q32 档案取样，现已更正；Node 绑定目录补全 CommonJS 包声明后实际运行。没有将这些失败计作通过。

## Host 接入边界

### 固定开发组件

核心提交 `dd64f86b2866eb1e8179e58044a7355ec1d7909b` 已构建独立 20 包 SDK `0.1.0-video.20261007.3`，manifest `dab130f7cba1df231acb39603465479bb3476e1a54759ad9af90bad265f99c14`，`sourceDirty: false`、`releaseCleared: false`。同提交重新构建的 profile 5 codec worker 字节保持相同，manifest 仍为 `4a4b194f09fdf2893b749dd5d3c4ba8ad54a1891a3648945341192522ab66fb3`，8,424,952 bytes；这不包含应用/GPU 框架、素材、字体和模型。

只消费 registry archives 的独立程序实际使用新 API 生成 64×48 / 12,288 bytes 预览，并完成前后定位、修改工程和 75 帧 FFV1 MP4 导出。独立解码的 1,382,400 bytes YUV 与输入一致；另解码 144,000 个 AAC 样本但输出仍无音轨。消费者报告在 `video-viewport-consumer-dd64f86/report.json`，没有用仓库私有源码填补缺失 API。

Musterwork worktree 提交 `bedef797d` 已安装并固定该版本。27 项组件安装检查、6 项实际素材权限/导入、7 项工程命令、1 项重开、2 项后台配置共 43 项通过，0 ignored；最终 `--locked` 图上的生产库 strict Clippy 通过。原 PPT 与完整非视频依赖图不变，原 192 项工作区状态（210 项展开记录）保留。产品证据由其 `video-viewport-product-*.log` 与汇总报告登记，不将这些检查当成真实播放器 UI 验收。

该增量不引入宿主权限、项目存储、业务队列或路径。产品预览仍需绑定当前作用域、工程版本、素材授权、有限会话/缓存、取消和二进制帧传输；UI 必须丢弃过期 generation，并使用同源音频设备时钟。真实右侧播放器、音频、导出 UI 和 Browser 媒体路径仍待连接，不能据此宣布用户可以验收完整方案。
