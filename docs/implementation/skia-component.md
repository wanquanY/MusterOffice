# 固定 Skia CPU 路径绘制组件

后续[渐变画笔](gradient-raster.md)已将组件升级为 ABI 4，并统一 Native/WASM 浮点着色。下面 ABI 3 的格式与体积是历史记录；当前入口以 ABI 4 及其阶段证据为准。
2026-09-24 · 已完成独立 Native/WASM 组件、批量 C ABI、TS 实例边界和真实像素验证。它接收已经计算好的设备空间路径，不读取字体，不重新排版。本节记录独立组件阶段；后续已接入 [Rust 路径渲染与隔离 worker](path-raster.md)。生产进程管理与完整 Draw IR 仍未完成，不是 E0/E3 验收完成。

## 采用和构建

源码固定为 Skia `8d6d37b063afe87fd361de55359fb4cb6b6f443c`，从官方 `google/skia` 镜像取得。归档为 79787709 字节，SHA-256 `c26bd622455aa77208f224263891b74195df7f6efd4bb7e182d7a4902ff08f89`。采用依据是已有设计指定的 Skia CPU 优先验证路线，见[算法与组件](../design/implementation/algorithms-and-components.md)；当前提交是本次固定实验基线，不宣称它是长期稳定版本。

[锁文件](../../components/skia/lock.json)、[裁剪参数](../../components/skia/cpu-profile.json)和[构建脚本](../../tools/components/build-skia.py)共同定义构建。不开启 Ganesh/Graphite、PDF、Skottie、SVG、字体管理器、HarfBuzz、FreeType、ICU、外部图片编解码、媒体及 PartitionAlloc。Skia 源码中自带的 skcms 仍参与色彩计算。没有下载整个 DEPS 闭包，也没有链接系统 Skia。

这只是本组件的能力范围。图片、效果、GPU、彩色字形、播放等一期能力仍须实现和验收，不能用裁剪配置把它们从产品承诺中删除。静态库中仍有未被最终路径程序引用的平台/编解码代码；最终链接使用 dead-strip。当前 macOS 原生程序只动态链接系统 libc++/libSystem，没有 CoreText、CoreGraphics 或 GPU 框架依赖。以后增加入口要重新检查链接闭包。

上游 GN 即使设置外部 `skia_emsdk_dir`，仍默认执行安装/激活 Emscripten 4.0.7 的操作，并开启 pthread。项目维护[小范围 GN 修改](../../components/skia/explicit-emsdk.patch)，为自动激活和 pthread 分别增加显式开关，保留上游默认行为；本构建关闭两者。后续为截断尖角增加了[私有描边回调接口](../../components/skia/custom-stroke-join.patch)，原有连接算法不变；具体语义及闭包验证见[截断尖角](miter-clip.md)。源码每次从核对后的归档重新提取，应用修改时不允许上下文模糊匹配。

GN 使用上游指定提交 `b2afae122eeb6ce09c52d63f67dc53fc517dbdc8`（2175）。官方二进制站不可达，本次从 GN Git 镜像取同一提交、核对 Git 对象后用 Apple Clang 构建。Ninja 1.13.2 的 macOS 归档与上游 release SHA-256 核对。工具只在忽略目录内使用，没有修改全局安装。

Skia/内置 skcms 的 BSD 文本及编译头文件中的 Khronos 许可声明保留在 [notices](../../components/skia/THIRD-PARTY-NOTICES.txt)。这不代表整个发行许可闭包已经完成：Emscripten、LLVM/libc++/libc 和最终各平台运行库仍须随发行归档。既有 SDK 安装器未保留二进制归档摘要的限制继续存在；锁文件核对 release ID、版本及实际 emcc/Clang/archiver 文件摘要，不冒充完整工具链可复现证明。

## 设备空间批次 ABI 3

[C 入口](../../components/skia/mo_skia.h)借用对齐的 `uint32_t` 请求，返回组件所有的 RGBA 字节；释放必须调用 `mo_skia_free`。调用者提供有效指针及对应长度，一次实例内串行调用。它不是公开 Agent 协议，也不是文档模型。

请求头共 10 words：`[0x4d4f534b, 3, width, height, backgroundRGBA, pathCount, drawCount, commandCount, strokeCount, 0]`。每条路径由 `[fillRule, count]` 和 `count` 条固定 7-word 指令组成；0 为 nonzero，1 为 evenodd。指令分别为 move=1、line=2、quadratic=3、cubic=4、close=5，后接 0/2/4/6 个 IEEE float32 坐标，未用槽必须全零。非 move 指令需要当前 contour；close 后需要新的 move。填充开放 contour 时由 Skia 隐式闭合。

路径后是 `strokeCount` 条 `[widthBits, cap, join, miterLimitBits]`。宽度为设备像素，0 为显式设备细线；cap：butt=0、round=1、square=2；join：miter=0、round=1、bevel=2、miterClip=3。miterLimit 是无量纲比率，miter 范围 0–1024，miterClip 范围 1–1024；round/bevel 的该槽必须为零。最多 4096 个样式。miterClip 的 0 宽度是一像素轮廓，其余连接沿用 Skia hairline。

最后有 `drawCount` 条 `[pathIndex, translateXBits, translateYBits, colorRGBA, paintIndex]`，按顺序绘制，可复用路径。paintIndex=0 为填充；1..strokeCount 指向描边样式。开放描边不隐式闭合；每次 draw 重设画笔模式，描边与填充不会互相污染。ABI 1/2 输入明确拒绝；Rust、C++、TS 必须配套升级。详见[描边实现](stroke-raster.md)。颜色 word 的低 8 位为 R，随后 G/B/A，输入为未预乘的 sRGB 编码颜色。输出为明确通道顺序的 RGBA8、premultiplied alpha、sRGB；当前固定抗锯齿和 source-over，按该 sRGB 表面执行合成，不宣称线性光或 Office 特定混合 profile。

组件限制：每边 1–8192 像素，总计最多 16777216 像素（64 MiB 输出），4096 路径、65536 draw、262144 指令；draw 引用的指令工作量总计不超过 1048576。坐标和位移都必须有限且在 ±32768 像素内，平移后的控制点范围及描边的保守扩张也不得超出该边界；方形端点和尖角限值参与扩张检查。整个语法和预算在调用 Skia/分配帧缓冲前检查。复杂度门槛不等于完整 CPU 时间或原生内存上限。

文档仍使用 Q32 EMU。当前验证工具先以整数减去 viewport 原点，再以 Fraction 缩放到设备空间，最后进入 float32 边界；人为增加 `2^110` 的共同原点不会改变批次或像素。它证明当前诊断映射避免过早浮点化，**不是已经完成的 Rust Draw IR 编译器或全域误差承诺**。后续 [Rust 编译器](path-raster.md)已实现有理缩放、局部锚点及控制坐标误差预算；完整仿射变换和裁剪仍需实现，不能直接把全局 EMU 转为 float32。

## 失败与宿主

状态码：0 完整成功，1 非法输入，2 分配失败，3 预算/支持范围超限，4 已失效。返回失败时输出指针/长度为零；只有整帧完成才转交像素。状态 2 永久标记当前库实例失效。

Skia 的部分分配失败会 abort，C++ `new` 也可能终止；不允许捕获后继续使用同一个内存状态。Native 必须置于可回收工作进程，WASM 必须置于独立模块/Worker。验证程序每个请求启动独立原生进程，并有 15 秒超时；生产进程池、超时/取消、RSS 限制和浏览器 Worker 尚未接入。

[TS 薄层](../../packages/raster-component/src/index.ts)只接收宿主核实的模块和工厂，没有下载逻辑；核对 import 白名单和 ABI，禁止 I/O/时区访问，为 libc 初始化提供显式空环境。它防止同一个模块被第二个包装器重新认领。真实返回码 2/4、trap、分配失败或不可信输出会使包装器永久失效；陷阱后连清理函数也不再调用。宿主必须销毁对应 Worker/实例。正常成功返回独立像素副本，不暴露可变 WASM 内存。

WASM 模块最大线性内存 256 MiB，栈 1 MiB，禁用文件系统、动态执行和 pthread。这个限制不包含 JS 像素副本、输入和宿主其他组件，也不代表完整内核的内存上限。Native Skia 内部分配尚未纳入生产预算系统。

## 初始 ABI 1 阶段的历史验证

当前 ABI 3 的配套构建和新增/回归证据见[截断尖角](miter-clip.md)；以下数值绑定原来的 ABI 1 产物，不表示当前字节摘要。

[本轮证据](../reviews/evidence/2026-09-24-skia-component-verification.json)绑定锁文件、构建参数、工具、组件和测试输入。148 组批次包含 29 份已有段落路径、64 份确定性随机曲线、填充/透明度/裁剪、无效语法与资源边界。Native、WASM 和 Native ASan/UBSan 的像素输出一致；独立矩形/镂空/source-over/半像素计算另核对 61444 个像素，容许 1/255 的舍入差异。

真实堆耗尽有两条路径：一条使帧缓冲分配返回失败，另一条在预留传输缓冲后耗尽内存，允许 1 像素输出分配进入 Skia，再触发其实际 abort。两者均不发布部分结果，拒绝实例复用；新实例可以继续绘制。它不是原生全部分配位置的穷举，也不表示恶意内容的完整隔离验收。

7 份 PNG 直接由本组件的像素编码，未借用浏览器、Quick Look 或办公软件进行栅格化。已查看双向文字、中文和原创三次曲线样本；视觉观察不替代 Office/WPS 对照。此独立组件验证不与已有 1883 批 Rust 主内核测试混算。

当前开发产物：Native 验证程序约 2.10 MB，WASM 模块约 1.80 MB，另有 JS glue 与 TS 薄层；静态库归档约 5.34/5.50 MB。它们是本 profile 的真实未压缩产物，未包含字体、图片/媒体/GPU、完整内核和宿主，不能外推到 Musterwork 安装包，也没有宣称性能门禁通过。

## 复现与下一步

先按锁文件取得源归档、构建工具以及既有 Emscripten，放在忽略目录。构建脚本自身离线，不安装工具。命令在仓库根目录执行：

```sh
python3 tools/components/build-skia.py --target native
python3 tools/components/build-skia.py --target wasm
python3 tools/components/build-skia.py --target native --sanitize
pnpm build:raster-component
python3 tools/verification/skia-fixtures.py
node tools/verification/skia-parity.mjs
python3 tools/verification/skia-previews.py
pnpm check:types
```

段落样本依赖[段落路径验证](paragraph-paths.md)已生成的缓存；缺少输入会明确失败。当前构建/原生探针已验证 macOS arm64，WASM 在 Node 上执行，尚未验证 Windows/Linux/浏览器。

Rust 受精度/资源预算控制的路径批次及开发 worker/WASM 入口已在[后续实现](path-raster.md)完成。接下来将完整画笔/图片/clip/group/effect 连接到同一个 Draw IR，再推进整页渲染、目标软件对照和性能测量。高级可编辑对象、动画/媒体、Agent OperationService 与 Musterwork 替换门禁继续保留完整目标。
