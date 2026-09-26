# 原生矩形渐变

2026-09-25：DrawingML `a:path path="rect"` 已接入原生页面编译、共享路径/场景绘制、Native/WASM 和 CLI。焦点可以是点、线或区域，支持非对称边距、零边距、外扩和极窄正边距。组合继承、旋转、背景及背景区域恢复复用已有页面链路。原始 PPTX 声明与来源索引不变；绘制不生成渐变位图。

这是开发 profile 的矩形几何实现。圆形、形状路径渐变、对象固定方向及目标 Office/WPS 验收继续开放，完整一期和 Musterwork 替换门槛尚未通过。

## 几何与数值

[微软 Open Specifications 说明](https://learn.microsoft.com/en-us/answers/questions/2248059/non-preset-a-tilerect-behaves-strange-in-case-of-g)将矩形渐变描述为焦点矩形与外部矩形之间的插值；有面积的焦点区域使用第一个色标。本实现将其表示为四个向内距离场的最大值，再交给已有颜色曲线。

独立平面中的坐标 u、v 先分别执行平铺。对正的左、上、右、下边距 l、t、r、b，标量为：

```text
max(0, 1-u/l, 1-v/t, 1-(1-u)/r, 1-(1-v)/b)
```

非正边距不参与对应项；全零边距得到完整焦点区域。运行时存储四个倒数，零倒数表示禁用该边。共享字段是 `GradientField::Rectangular { edgeRates, uncertainty }`。来源百分比使用精确十进制比值判断焦点是否倒置，允许宽/高为零，不让 `1/3 + 2/3` 的中间舍入改变几何分类。倒置焦点仍给出明确诊断，尚未定义其目标应用兼容行为。

矩形场的接收平面使用当前形状逻辑尺寸和原生放置，色标/属性仍先经过继承求值。来源的缺省焦点沿用已有填充解析 profile；“覆盖整个区域”的验证样本显式声明四条零边距，避免把未声明边误当成零。现阶段不把该逻辑矩形规则外推到其他路径模式或不同于逻辑边界的自定义形状。

倒数是无量纲参数，不受像素坐标 ±32768 的上限约束。新增整数 Q32→float32 一次舍入，遵循 ties-to-even，支持到 Q32 正范围内可表示的 float32；舍入至 `2^95` 被拒绝。边距 3% 和 10⁻¹² 等输入能进入真实渲染。误差检查利用边场仅在 `distance <= 1/rate` 时生效的性质，计算相对误差；四场取最大误差，仍受既有 `2^-20` 数值预算约束。禁用边的不确定范围跨过零时拒绝。坐标误差另由共享平面检查；这些界限不能当作所有色标/像素的统一误差证明。

## 共享组件

固定 CPU 向量阶段直接计算四个边场，复用现有颜色、alpha、裁剪、合成与资源预算。旧线性场的浮点运算顺序和误差计算保留。无新增第三方依赖、像素缓存或运行时编译器。

V11 保持 14 字帧头和 8 字绘制项；新渐变 kind=3 使用 17 字头，依次为共用 5 字、平面矩阵 6 字、双轴平铺 2 字及边场倒数 4 字。最多 4096 个独立渐变，因此总上限从 V9/V10 的 2,883,662 增至 **2,887,758 个 u32**。其他几何及旧版本帧保持原格式。

Native/TS 通过 `mo_skia_rect_gradients_abi()==1` 协商。缺少能力的旧组件在调用前拒绝 V11，两种入口拒绝后均可继续执行旧帧。当前原生静态链接须使用包含新增查询符号的组件。

6 份 JSON Schema 只增加矩形字段变体；生成 TS 同步更新，其中一份较大的合同按既有生成器规则拆成两个文件。独立校验删除新增变体后重新生成，恢复此前 Schema 与 TS 的精确摘要。其他合同保持不变。

## 验证与大小

[阶段证据](../reviews/evidence/2026-09-25-rectangular-gradient-verification.json)绑定源码、构建、原始输出和以下检查：

- 601 项 Rust 测试，严格 Clippy、格式与 80 份合同检查通过。两种倒置焦点在图片解码前拒绝。
- 121 对通用 Native/WASM 调用覆盖三种颜色插值、变换/反射、窄边距、禁用边、alpha、裁剪和快照；7 项无效输入不调用后端。独立 binary64 参考核对 568,998 个像素，最大通道差 1 色阶。
- 18 份自有 PPTX 的 126 个部件通过官方 XSD。139 对来源调用中 117 对成功；之前全部 121 对来源调用的元数据、帧和像素保持不变。15 份轴对齐页面另由原始 XML 独立计算 1,800,000 个像素，最大通道差 1 色阶；该参考不覆盖旋转/组合抗锯齿。
- 533 组直接 Native/WASM/ASan/UBSan 检查含 376 个旧帧、18 个新来源帧和 25 个无效帧；无诊断。未运行 LeakSanitizer。
- 1,002 对既有绘图及 307 对既有页面/文字请求保持结果；CLI 创建、拒绝覆盖和失败不发布通过。
- LibreOffice 26.2.0.3 打开 17 份非对照 PPTX 并输出两页 PDF。全部存在像素差异，部分通道差达到 255；硬色标样本最大差 45。全部观察保留，不能用数学参考通过来消除外部兼容差异。

未压缩 Rust WASM 为 **6,280,243 字节**，增加 19,174；固定 C++ WASM 为 **2,327,852 字节**，增加 316。没有新增锁定依赖；这里只报告模块大小，尚无本阶段整页性能或 Musterwork 安装包测量。

## 复现

先用锁定输入，在新的 `.codex-work/rect-gradient/component` 目录分别完成 Native、WASM 和 native-asan 构建。命令见[开发说明](development.md)，ASan 使用对应 `deps-asan`；三个构建结束后再运行工作区检查。此前封存目录不得覆盖。

```sh
python3 tools/verification/rect-gradient-checks.py
node tools/verification/rect-gradient-parity.mjs
node tools/verification/rect-gradient-source-parity.mjs
node tools/verification/rect-gradient-components.mjs
node tools/verification/rect-gradient-regressions.mjs
node tools/verification/text-page-runtime-regressions.mjs .codex-work/rect-gradient .codex-work/rect-gradient/component .codex-work/rect-gradient/ts-raster/index.js
node tools/verification/rect-gradient-contract-compat.mjs
python3 tools/verification/rect-gradient-reference.py
python3 tools/verification/contracts.py
python3 tools/verification/rect-gradient-observe.py --soffice /path/to/soffice
python3 tools/verification/rect-gradient-evidence.py
```

应用观察使用含 PyMuPDF 的 Python。证据脚本还校对实际构建清单、历史产物、全部报告和本地文档链接；封存后用 `--check` 重验。

后续继续实现 circle/shape 及固定方向。微软后续纠正了径向中心使用实际路径边界的说明，并确认非预设 tileRect 组合有应用实现问题；这些需要各自的几何和应用证据，不能套用当前矩形场或普通圆心/半径模型。
