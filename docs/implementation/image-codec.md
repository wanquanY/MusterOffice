# 编码图片解码

2026-09-25。状态：静态 PNG/JPEG 的显式字节解码已有实现，覆盖 Rust 库、隔离 Native Worker、实际 WASM 和共享场景绘制。此阶段不代表真实 PPTX 图片填充已经接入页面，也不代表完整图片格式、动画图片或 Office/WPS 验收完成。

## 职责与输入输出

`mo-image` 持有独立 `ImageDecoder` 接口、源摘要校验、取消、结果验证和已归一化资源。它不依赖绘制器、文件路径、网络或系统解码服务。`mo-skia-sys::NativeRaster` 和 TS `RasterComponent` 实现解码扩展，共享同一组件实例及失效状态；解码与绘制仍为不同计算接口。

`image-decode-request` 只传 `sourceSha256`，编码字节通过独立二进制通道传递。格式取自实际字节，不信任 MIME 名称。成功元数据记录编码/归一化尺寸、EXIF 方向、格式、编码位深、来源颜色分类和编码/像素双摘要。输出固定为紧密排列的 premultiplied sRGB RGBA8，16 位 PNG 转换到该显式输出精度；原始编码资源须由宿主继续保留，不能用解码像素取代原稿资源。

成功对象只能经完整验证产生；无部分像素输出。归一化图片可直接创建现有 `ImageResource`，通过 `PreparedImages` 进入[共享场景绘制](image-scene.md)，没有第二套布局/绘制路径。PPTX 资源选择与提取继续由[来源图片模块](source-images.md)负责。

## 固定组件

- Skia/skcms 延续既有 `8d6d37b063afe87fd361de55359fb4cb6b6f443c`。
- libpng 固定 `d5515b5b8be3901aac04e5bd8bd5c89f287bcd33`（该来源标识 1.6.56）；libjpeg-turbo 固定官方 3.2.0 源码发行包；zlib 固定官方 1.3.2 源码包。
- 来源 URL、字节数、下载摘要和完整许可文件摘要见[锁文件](../../components/image-codec/lock.json)。这些是独立锁定的上游构建输入，不能声称与 Skia DEPS 中的 Chromium JPEG/zlib 分支完全相同。
- 依赖由 CMake 离线构建静态库；Skia 的三个依赖构建描述通过明确路径桥接，禁止隐式查找系统 JPEG/PNG/zlib。构建记录保存锁、命令、头文件、配置、库摘要和 GN 目标图。完整发行工具链可复现性尚未验收。
- Native 使用目标平台 JPEG SIMD；当前 WASM profile 不启用 JPEG SIMD，使用 Wasm EH 的 setjmp/longjmp，避免引入 JS `invoke_*` 调用跳板。已运行的宿主为 Node，目标浏览器兼容性仍待专项验收。

许可证分别涉及 libpng-2.0、IJG/BSD/Zlib。未改变仓库整体公开许可证或完成发行许可闭包。本软件基于 Independent JPEG Group 的部分工作：**This software is based in part on the work of the Independent JPEG Group.** 完整通知保存在 `components/image-codec/licenses/`。

## 完整性、色彩与资源边界

PNG 验证完整 chunk 边界、CRC、IDAT 连续性、终止位置及 ICC 压缩数据；JPEG 验证 marker/scan 边界、完整终止、ICC 分段编号/重组和单图约束。SkCodec 继续负责实际像素解码，skcms 负责向 sRGB 转换；解码结果再次由 Rust 校验尺寸、方向、格式、长度、premultiplied 通道和摘要。

EXIF IFD0 方向支持 1–8，PNG `eXIf` 与 JPEG APP1 共用有界解析。方向只应用一次，5–8 交换尺寸。无颜色声明时使用明确的 sRGB 假设；支持有效 ICC 和 PNG gAMA/chromaticity 转换。无效 ICC、无效色度及上游恢复性解码警告均失败，避免静默丢弃色彩声明或返回修补像素。全部输出统一按本阶段的 SDR RGBA8 profile 验证，HDR/高级色彩目标尚未实现。

输入上限 32 MiB；维度每轴最多 8192；输出最多 64 MiB；容器元数据最多 4 MiB；ICC 解压后最多 1 MiB；最多 4096 个 chunk/marker。libpng 另限制单个解压 ancillary chunk 1 MiB、缓存 32 项。上限不是峰值 RSS 承诺：方向旋转可能同时持有两个像素缓冲，解码器和宿主跨 WASM 传输也有额外分配。

组件同步调用期间由宿主终止隔离进程/Worker 实现抢占取消；Rust 在调用前后与摘要/像素验证过程中检查取消。调用后取消、组件异常、非法响应或分配失败会使实例失效，不发布结果。WASM 禁止共享输入缓冲及组件内存别名；文件、环境和退出调用由宿主限制或拒绝。

APNG、MPO/带 MPF 的 JPEG、HDR PNG、未列入当前 JPEG profile 的编码过程及其他格式明确返回 `UNSUPPORTED`。这只是当前静态解码入口的能力边界，完整一期中的 GIF/WebP/SVG/动画资源等并未从目标中删除。来源分辨率已在后续[分辨率与物理尺寸阶段](image-resolution.md)实现。完整颜色意图、CMYK/复杂 ICC 的参考语料与目标应用差异仍需继续验证。

## 验证与继续工作

自有合成语料包含 41 个成功样例：八种方向、两种 EXIF 字节序、透明度、调色板、2/16 位灰度、Adam7、基线/渐进 JPEG、PNG gamma、RGB/灰度线性 ICC 和乱序 JPEG ICC 分段。像素期望由独立数学/样本值计算；PNG 无颜色转换及灰度 JPEG 块要求精确相等，颜色转换每通道容差最多 1。它们还经过解码→已归一化资源→共享场景的 41 对实际绘制。

错误语料覆盖截断、CRC/Adler、DEFLATE、非法过滤器、多余像素/尾部、空 JPEG scan、ICC 缺段/重复/膨胀、方向越界、像素预算以及不支持的序列。原生与实际 Rust/WASM 比较完整元数据和像素；伪造宿主响应单独检查隔离。150 个解码语料（41 成功、109 明确失败）、4 个 JSON/摘要拒绝及 41 个后续场景构成 195 对新调用；4 个伪造 WASM 宿主响应另验。150 个语料通过完整 C++/依赖 ASan 与 UBSan（未启用 LeakSanitizer），既有 716 对调用重跑保持响应/像素；Rust 测试 527 项、生成合同 78 份。当前单独绘制＋解码 WASM 为 2,313,369 字节，相对先前绘制组件增加 502,378 字节，不包括 Rust 核心、字体、媒体、宿主或安装包。完整计数和已完成检查以[阶段证据](../reviews/evidence/2026-09-25-image-codec-verification.json)为准，不能将合成小图一致性外推为完整视觉质量、性能或产品验收。

下一步在现有来源页面循环中连接已提取图片、decode 资源、`srcRect`、stretch/tile、旋转与效果诊断；继续使用原有场景/画笔/资源缓存。不得把未解析/外链 catalog 当成完整页面成功，不应整页截图或静默忽略图片来通过验收。之后继续完整编辑/导出、播放与高级内容、Agent/Artifact/Musterwork 接入。
