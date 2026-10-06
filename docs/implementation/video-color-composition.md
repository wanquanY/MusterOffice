# 视频色彩与 CPU 合成实施记录

2026-10-06。当前是实际执行的参考后端，未完成实时 GPU 档案或产品接入。上游为[精确编译](video-editing-projection.md)、[真实源帧采样](video-source-presentation.md)及[隔离媒体 worker](video-media-worker.md)。最终要求仍以[技术方案](../design/video-editing/technical-spec.md)和[验收矩阵](../design/video-editing/acceptance.md)为准。

## 所有权与模块

- `mo-color::rgb` 负责通用 D65 RGB 原色矩阵与显示 transfer 数学。沿用固定 `libm`，没有增加色彩处理第三方代码。
- `mo-video-render` 持有视频像素解释、几何放置、线性光合成和输出变换。正常库依赖不包含 codec、原生进程或 GPU；测试才接入 `mo-video-source/native`。
- `VisualResources` 由调用方提供，按内容/流绑定和精确时间返回拥有像素的引用计数图像。内核不打开文件、URL，不修改工程，不保存宿主任务。
- 输入错误、取消和预算失败不返回部分成功图像。未知格式、缺失必要解释、文字未接入等返回明确错误，不替换为透明画面。

## 明确的颜色档案

草案档案标识为 `musteroffice.video.linear2020-d65-203nits-premul-cpu-v1-draft`。内部为 **BT.2020 / D65 / 线性显示光 / RGBA32F / 预乘 alpha**，线性值 1 对齐 203 nit；在输出前保留可表示的负 RGB、广色域和高光。

`InputColor` 显式携带原色、transfer、YUV matrix、full/limited range、色度位置和 alpha 方式。由原始 codec 标签构造解释时：

| 类别 | 当前接受 | 未知或不支持 |
| --- | --- | --- |
| 原色 | BT.709、BT.2020、Display P3、SMPTE C、BT.470 BG | 拒绝；不能从分辨率猜测 |
| SDR transfer | sRGB、Gamma 2.2/2.8、理想黑位 BT.1886 | CICP 1/6/14/15 按本档案的 BT.1886 **显示**解释处理；保留原标签 |
| HDR transfer | PQ；带显式峰值和 system gamma 的 HLG | HLG 缺参考显示参数拒绝；不把 inverse OETF 当完整 EOTF |
| matrix | RGB、BT.601、BT.709、BT.2020 NCL | BT.2020 CL、ICtCp 等尚不支持 |
| range / chroma | full/limited；六种明确的色度位置 | 子采样 YUV 无位置解释拒绝 |
| alpha | opaque、straight、encoded-premultiplied | 有 alpha 且含义未知拒绝 |

转换顺序：验证平面 → 按位深展开范围 → 按位置插值色度 → YUV 矩阵 → 若源 encoded-premultiplied 则先除 alpha → 显示 transfer → 原色变换 → 线性光乘 alpha。RGB 路径拒绝再次使用 YUV 矩阵。浏览器已经转换的 RGB 须描述**转换后的**编码。

BT.709 摄像机 OETF 的逆不等于本档案要求的成片显示线性光。独立对照曾暴露这一问题；现按 [BT.1886](https://www.itu.int/rec/R-REC-BT.1886/en) 的理想黑位参考 EOTF 使用 `max(signal,0)^2.4`。负信号截断会计数，超白保留。这是明确选择的显示档案，不声称还原拍摄场景光。

PQ 按绝对亮度换算，HLG 使用逆 OETF 后的亮度相关 OOTF；非物理 HDR 信号截断有诊断。矩阵从原色 xy 和 D65 推导，没有从 zimg 或 W3C 示例复制实现。[ITU BT.2100](https://www.itu.int/rec/R-REC-BT.2100)与 [W3C Color 4](https://www.w3.org/TR/css-color-4/#color-conversion-code)为核查依据；[CSS HDR 工作草案](https://www.w3.org/TR/2026/WD-css-color-hdr-1-20260907/)仅作参考，不当作最终标准。

## 输入平面和输出

当前支持 packed RGB24/BGR24、RGBA/BGRA/ARGB/ABGR、planar GBR/GBRA、YUV 420/422/444、YUVA 420/444，以及相应支持的 9/10/12/14/16 位 LE/BE 平面；另支持 NV12 和 P010。只接受代码明确列出的格式，严格检查平面数量、行字节数、行数、有效位深与 padding。奇数尺寸按色度平面的向上取整尺寸计算。

非交错软件帧是当前输入边界。硬件帧导入、交错消除、ICC 图片解释和未列出的格式仍待实现。

输出可选 straight 或 encoded-premultiplied sRGB RGBA8。HDR → SDR 必须传入明确 tone-map；现有 `ReinhardLuminanceV1` 使用显式源白点，是版本化的全局亮度曲线，**不是** BT.2390、内容自适应或最终 HDR 质量档案。最终色域截断、tone-map 像素有计数；没有默认静默 tone-map 或 HDR 成片导出。

## 几何与合成

- 先应用像素比例，再应用媒体显示矩阵，按变换后边界等比例 contain 并居中，最后应用作者变换。缺 SAR 需调用方明确解释；不把旋转直接改写成源像素。
- 读取 FFmpeg display matrix 的 16.16 / 2.30 字段；支持仿射旋转、镜像和位移。透视/奇异矩阵明确拒绝。
- 作者变换绕画布中心，按计划顺序合并矩阵，只在合成时重采样一次。最近邻和双线性是当前 CPU 参考档案；双线性在线性预乘 alpha 中执行，避免透明像素彩边。
- 自底向上执行 source-over。嵌套组先生成独立表面，再整体应用透明度/变换；不能把组透明度分摊给子项。
- 视频返回的呈现区间必须覆盖请求的精确 before/after 采样，内容/流绑定、尺寸和可用区间必须匹配。静态图片不能伪造视频时间区间。
- 有理透明度在渲染边界以 Q32、nearest-ties-even 量化，每个因子误差不超过 `2^-33`；作者时间与工程不改变。

当前 quad 边界采用中心覆盖判定，未加入 MSAA/解析边缘覆盖；双线性也不等价于高质量大比例缩小过滤。文本、裁剪、模糊、遮罩、完整转场/关键帧、GPU 和更高质量过滤未在本模块中完成，不关闭相关最终验收项。

## 预算与取消

默认单图 16,777,216 像素/256 MiB；硬上限单图 1 GiB、边长 16384。合成另外限制节点、深度、效果数、像素访问和同时存活表面。预检包含嵌套节点，组表面分配计入峰值。

`peak_temporary_bytes` 仅是当前合成表面的 active bytes。解析器持有的源帧、allocator 容量、worker、GPU 或进程 RSS 不在该数值内；调用方必须分别设资源预算。逐行/主要边界检查取消，不声称它能抢占阻塞的资源回调。

## 实际验证

| 验证 | 本地结果 |
| --- | --- |
| 色彩与像素/合成单测 | 18 项通过（包含原有 mo-color 4 项），真实 worker 用例默认 ignored，另行显式执行 |
| 真实素材全帧颜色 | 同一固定 worker 解码 77 帧、2,838,528 个线性 RGB 通道值；与独立 FFmpeg CLI/zscale 比较，最大误差 `4.76837158203125e-7`，RMS `6.966979884664961e-8` |
| 独立参考配置 | BT.709 limited、left chroma → 线性 BT.2020；bilinear，`agamma=0`，比较实际 FP32，门限 max `2e-6`、RMS `2e-7` |
| 工程合成 | 从真实工程计划采样输出第 0/17/40/89 帧，另输出旋转 15°、缩放 75%、透明度 85% 的合成 PNG；摘要逐个记录 |
| Native/WASM | 实际 macOS arm64 程序与 Node WASM 执行 9 查询，7 渲染/2 拒绝；linear f32 bits、RGBA8 和诊断完全一致 |
| 原合同回归 | 25 步编辑和 15 查询编译跨端结果与既有摘要一致 |

独立 FFmpeg CLI 仅作测试参考，不进入产品依赖。前序 worker 的原始 YUV SHA 与本次 normalization 输入再次核对，避免两个测试不使用同一素材。原生包摘要、素材摘要、CLI 版本和所有命令均进入报告。

跨端 fixture SHA：`fafdd2a9943f9da326c6c79945e0d06022bed8b1492bab4a14bee3ac5b70e98f`；结果 SHA：`66e1ae3ac94977bdac02d616fbe2a8cb9cbac8f81f043820b1938549c1d4db83`。这些小向量不证明所有 GPU/设备或 HDR 素材一致。

## 复现与证据

```sh
python3 tools/verification/current.py --groups media \
  --ffmpeg-dir .codex-work/ffmpeg-native-final \
  --output .codex-work/new-video-media-check
```

media 分组依次执行 C 回调、Rust 所有权、worker 构建、真实 worker、工程源采样和色彩/合成六步。第六步脚本为 `tools/verification/video-render-check.py`。跨端由 `current.py --groups wasm` 的独立 `video-render-native-wasm-parity` 步骤执行，报告与原编辑/编译结果分别保存，不能互相覆盖。

本次本地证据在 `.codex-work/video-render-gate-final/`、`video-render-parity.json`、`video-render-domain-regression.json`、`video-render-projection-regression.json` 和 `video-render-workspace-clippy.log`。既有五步媒体检查也在同次通过。报告绑定当时源文件摘要，后续文档和入口报告路径修正不冒充媒体代码重测。

这批结果证明有真实解码像素进入同源 CPU 合成。没有证明浏览器媒体接入、GPU 实时性能、音频 DSP、生产视频导出、完整效果或用户工作台已完成。
