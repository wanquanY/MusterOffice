# 渐变画笔与浮点着色

2026-09-25：在既有[填充颜色计算](fill-colors.md)之后，将线性和圆形径向渐变接入路径与场景绘制层，并完成 Native/WASM 实际像素与独立解析验证。证据见[本阶段记录](../reviews/evidence/2026-09-25-gradient-raster-verification.json)。这不代表来源 PPTX 的完整渐变、整页绘制或 Musterwork 替换已经验收。

## 分层与接口

`mo-raster::Brush` 区分 `solid { rgba }` 和 `gradient { gradient }`，替换开发版 `PathDraw.color` / `PathInstance.color` 字段；现在统一为 `brush`。已有 author 页面编译器将纯色降低为 solid 画笔。路径和场景请求、Schema、TS 类型同步迁移，不同时接受互相矛盾的颜色与渐变字段。这是尚未发行的开发接口升级。

渐变的 `geometry` 为世界坐标 Q32 EMU 的 `linear { start, end }` 或 `radial { center, radius }`；`stops` 为递增或相等的位置及非预乘 sRGB 工作通道。显式选择 `tile`（clamp/repeat/mirror/decal）、`interpolation`（srgb/linearSrgb）与 `alpha`（straight/premultiplied）。字段均为必填，不采用浏览器或环境默认值。

此处输入是已求值的空间画笔。它不随局部路径的重复放置而移动，也不再次应用路径仿射变换；场景编译器只与视口一起精确重定位。文稿编译器负责把作者的相对渐变、旋转、组变换、fillToRect 等转换为世界画笔。圆形径向原语不能冒充椭圆、矩形或形状路径渐变；这些空间语义仍需连接。

颜色不经过 RGBA8 中间采样。每个停靠点的 f64 工作通道直接转换为后端 float32，随后在选定颜色空间插值。Alpha 的预乘选择也显式传入；没有把两种插值规则当成等价效果。

## 资源、数值与失败

每批最多 4096 个独立渐变、65536 个存储停靠点；每条渐变为 2–4096 个停靠点。重复请求声明累计最多处理 262144 个停靠点，先检查再复制。相同设备参数共享画笔资源；颜色空间、透明度、延展方式都属于资源键。

位置必须位于 [0,1] 且不递减。重复位置作为硬停靠点保留，不排序或去重。缺省端点延展首末颜色。不同位置在 float32 边界坍缩，或内部位置坍缩到 0/1 时，返回精度错误，不悄悄改变渐变。RGB 必须有限、绝对值不超过 65504；alpha 必须在 [0,1]。f64→float32 的最大绝对误差不超过 2⁻²⁰，实测误差独立报告。

世界坐标先精确减去视口原点，再通过有理缩放进入 float32；几何输入误差受请求的坐标容差约束。报告 `gradientCoordinateErrorBound` 与 `gradientValueErrorBound`，两者分别描述几何输入和停靠点转换，不宣称是整个 shader 或最终像素的误差界。

设备空间线段长度或径向半径小于 2⁻¹⁴ 像素时返回精度错误，避免触发固定 Skia 中的退化渐变替代规则。PPTX 退化画笔的实际应用语义仍由文稿层定义。所有非法输入、预算超限和取消都在发布像素前失败。

## 组件 ABI 4

头部仍为 10 words：`[magic, 4, width, height, backgroundRGBA, pathCount, drawCount, commandCount, strokeCount, gradientCount]`。路径与描边样式格式保持原定义；其后是渐变资源，最后是绘制项。

每个渐变为 9 words：`[geometry, tile, interpolation, premul, stopCount, geometry0, geometry1, geometry2, geometry3]`，其后每个停靠点为 `[position, r, g, b, a]` 的 float32 位模式。线性几何为两点；径向为中心、半径和零。绘制项为 `[path, dx, dy, solidRGBA, strokeIndex, gradientIndex]`；gradientIndex 为零表示纯色，否则为 1-based 引用且 solidRGBA 必须为零。

整个批次最多 2617354 words。C++ 独立验证全部语法、引用、数值及预算，再创建 Skia 对象；它不依赖 Rust 已经检查。Shader 的逆平移抵消路径局部放置，纯色与渐变切换时清除旧 shader，描边仍使用同一画笔和已有截断尖角算法。失败不转交部分输出，分配失败使实例失效。

## 跨端浮点规则

最初实测发现相同指令在原生与 WASM 上存在 1 个色阶的像素差。固定 Skia 在原生启用低精度着色流水线，而标量 WASM 使用浮点流水线。组件采用上游已有的 `SK_DISABLE_LOWP_RASTER_PIPELINE`，统一浮点着色并保留原生 SIMD。

ARM SIMD 与标量路径还存在融合乘加、倒数和整数转换差异。显式构建开关 `MO_SKIA_DETERMINISTIC_RASTER` 配合[受控修改](../../components/skia/deterministic-raster.patch)，统一分步浮点乘加、精确除法与最终转换；`-ffp-contract=off` 属于构建合同。没有通过修改输出像素或放宽跨端比较来消除差异。实际跨端结果、性能影响和适用平台必须分别验证。

着色调用固定提交中的 [SkGradient 接口](https://github.com/google/skia/blob/8d6d37b063afe87fd361de55359fb4cb6b6f443c/include/effects/SkGradient.h)。固定组件的声明、颜色输入与算法实现共同构成当前 draft，不据此声称与 Office/WPS 的插值或混合完全一致。

## 验证边界与后续

新增验证入口：`gradient-raster-fixtures.mjs`、`gradient-raster-parity.mjs`、`gradient-raster-reference.py`。原生、WASM 和 ASan/UBSan 使用真实组件。独立参考从请求中的世界几何与颜色计算像素中心的解析结果，并以 Fraction 检查重定位和 float32 转换；不从组件颜色反推参考答案。

本阶段新增 58 批主内核检查：42 份实际绘制的元数据和像素 Native/WASM 完全一致，原生独立组件与 ASan/UBSan 的像素也一致；16 个非法/不可表示请求在调用组件前失败。另有 14 份直接伪造的组件批次，独立验证 C++ 拒绝错误 ABI、几何、插值、颜色、引用及歧义字段，不发布像素。当前 macOS 消毒器不支持 LeakSanitizer，因此未声称完成泄漏检测。

独立解析参考核对 39 个矩形画笔输入、159744 个像素和 638976 个通道，预设容差为 1 个 RGBA8 码值，实测最大差为 1。该对照使用 Python 的独立空间公式和 sRGB 转换；Native/WASM 之间的要求仍为零像素差，不与参考容差混淆。额外核对 156 个世界几何坐标和 770 个停靠点数值的转换。

既有 7680 批主内核用例在当前产物上重跑，6965 批完整结果不变；715 批仅迁移明确的画笔字段、ABI、profile 和零新增工作计数。620 份成功绘制的像素摘要保持原值，当前批次可重建原 ABI 3 字节并核对原摘要，132 份页面计划逐项核对。路径、场景、作者页面的独立几何检查也重跑通过。累计 360 项 Rust 测试、7738 批主内核跨端检查；严格 Clippy、格式、61 份 Schema 和 TS 类型通过。

底层既有 197 组组件检查在新浮点 profile 上重跑通过，61445 个独立参考像素无差异，包括真实分配失败、失效实例拒绝复用和新实例恢复。构建闭包仍为原生 497 / WASM 496 个上游编译单元，新增规则位于受控头文件修改；实际原生动态链接仍仅为 libc++/libSystem。

## 当前局部性能与体积

`gradient-raster-benchmark.mjs` 在 Apple M4 Max、14 个逻辑核、36 GiB RAM、macOS Darwin 25.2.0、Node 23.5.0 上测量。每种输入预热 5 次、测量 25 次，进程/模块保持热态；每次仍重新解析、编译、绘制、散列和复制结果。输入为单个全幅矩形，没有字体、图片或 PPTX 解析；原生计入工作进程 IPC，WASM 计入 JS/Rust/组件边界，均不计启动。两者执行边界不同，不能用本表推导内核语言性能优劣。

| 输入 | Native 中位数 / p95 ms | WASM 中位数 / p95 ms |
| --- | ---: | ---: |
| 256² 线性渐变 | 0.978 / 1.112 | 1.943 / 2.087 |
| 256² 径向渐变 | 0.933 / 0.999 | 2.261 / 2.392 |
| 1024² 线性渐变 | 13.979 / 14.409 | 30.318 / 31.454 |
| 1024² 径向渐变 | 14.369 / 14.818 | 37.383 / 38.572 |

本次没有对旧着色 profile 做时间对照，没有测量 RSS/完整页面/安装包，不能宣称性能提升或已经满足产品帧率。诊断接收端采用线性分块累计，避免逐块复制整个已有输出所造成的二次复杂度污染测量。

| 未压缩开发产物 | 字节数 | 相对填充颜色阶段 |
| --- | ---: | ---: |
| 原生 CLI | 5113776 | +19680 |
| Rust WASM | 4613815 | +73723 |
| Rust WASM 胶水 | 30802 | 0 |
| 绘制工作进程 | 3703184 | +68288 |
| 绘制 C++ WASM | 1808300 | +3061 |
| 文字工作进程 | 2375360 | +32 |

字体 C++ WASM 及其胶水/薄层保持原摘要；渲染薄层虽然 ABI 核对升级，输出长度恰好不变，不因此声称字节不变。没有新增外部运行依赖或锁文件变更。这些产物仍不是完整内核或 Musterwork 安装包。

完整来源渐变空间计算、图案、图片、效果图、来源对象与文字页面编译、高级对象/播放、真实 Office/WPS 对照和 Musterwork E0–E3 仍需实现及验收。当前改变不削减一期承诺，也不能外推安装包、完整页面性能或资源收益。
