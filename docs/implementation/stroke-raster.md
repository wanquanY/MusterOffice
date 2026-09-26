# 纯色描边与设备批次 ABI 2

2026-09-24 · 在原有 Rust/TS/C++ 路径上实现真实纯色描边。本阶段范围是已求值 Draw IR 与路径渲染。后续已接通[显式作者描边与 PPTX](stroke-author.md)，样式继承、虚线、箭头和复合线型仍需继续连接。没有以位图替代原生线条，也没有切换 HTML/CSS 技术路线。

## 语义与职责

`PathDraw.stroke` / `PathInstance.stroke` 是可选的明确画笔参数；省略时保持原有填充行为。描边包含 Q32 世界/页面 EMU 宽度，butt/round/square 端点，round/bevel/miter 连接，及 miter 的无量纲限值。开放 contour 不隐式闭合。填充加描边用顺序明确的两次实例绘制，共用路径资源；后续 fill 会重置画笔模式，不继承前一条描边状态。

当前 Draw IR 的线宽已处于世界空间，几何变换只变换中心线，viewport 的统一比例才缩放线宽。这是求值层合同，不是对所有 Office 对象线宽继承规则的推断。旋转、反射、非等比缩放、剪切及奇异变换均有显式测试；在几何变换后使用世界宽度，不让底层矩阵无意拉伸笔刷。

宽度 0 明确表示 Skia 设备 hairline，不等于无描边；无描边由调用者省略对应 draw。正的极细线仍按真实宽度绘制，不提升为零宽线。所有输入必须显式提供端点/连接，不借用系统默认样式。Serde 内部标签的 unit variant 会忽略多余字段，因此无参数连接采用空结构变体，拒绝在 round/bevel 上夹带 miter 参数；Rust 单测与跨端非法输入均覆盖这个区别。

## 数值、预算和 ABI

线宽直接由精确整数比例舍入到 float32，没有先经 f64；真实中点反例验证双重舍入会产生错误末位。宽度误差以原始 Q32 **像素**报告，必须满足调用者容差。miter 限值单独以原始 Q32 **无量纲比率**报告，不能混入像素误差预算。

这些上界分别约束中心线控制坐标、宽度及连接参数的量化；它们不承诺曲线笔画外边界、尖角阈值两侧的连续性、抗锯齿覆盖或 Office/WPS 像素一致。尖角限值范围为 0–1024。每份设备批次最多 4096 个不同的已量化描边样式；量化后相同的样式共享记录，输入的实际误差仍分别检查并汇总。

设备 ABI 升为 2，头部记录样式数，路径之后保存 4-word 样式表，绘制记录扩为 5 words。0 画笔索引表示填充，其余索引引用样式；完整格式见 [Skia 组件](skia-component.md)。Rust、Native C++、WASM C++、TS 适配器均使用同一版本，拒绝 ABI 1 输入；这不是公开 Agent 协议版本变更。

宽度、端点/连接枚举、样式引用、非有限值、未使用字段和预算在组件绘制前验证。设备控制点加描边保守扩张必须留在 ±32768 像素范围内；方形端点和尖角限值参与扩张。该限制防止数值范围越界，不表示最优可见 ink bounds 或完整 CPU/RSS 预算。错误输入、取消、真实分配失败和组件 trap 继续遵循整帧原子失败与失效实例回收合同。

## 本轮证据

[封存证据](../reviews/evidence/2026-09-24-stroke-raster-verification.json)绑定源码、实际产物、输入和输出。新增 77 组路径用例（65 成功、12 拒绝）及 70 组场景用例；路径共 193 组、场景共 251 组，Native/WASM 元数据及像素逐字节一致，同一设备批次另经独立原生 C++ 探针执行。原来的 116 组路径、181 组场景与 74 组作者页面保留状态和像素；旧填充批次可从 ABI 2 无损还原为旧 ABI 1 字节并核对封存摘要。

独立 Fraction 参考核对描边参数舍入、样式表和世界线宽，同时重验路径的 2109532 个变换后坐标、场景的 2250320 个控制坐标。32 个独立解析像素探针核对端点、连接、开闭路径和画笔切换。12 份样本按实际 premultiplied 像素编码为检查图，放大时使用 nearest-neighbor；预览编码不参与生产渲染。

独立组件共 174 组 Native/WASM/Native ASan/UBSan 比较，像素逐字节相同；61445 个参考像素检查通过。实际组件堆耗尽和 Skia 内部分配 abort 仍无部分输出，拒绝复用，替换实例恢复。主内核累计 247 项 Rust 测试、2492 批 Native/WASM 逻辑用例、49 份 Schema、严格 Clippy 与 TS 检查通过。Schema 有 6 份按描边能力更新，其余 43 份字节不变；未更改外部运行依赖版本。

这些是当前 macOS arm64/Node WASM 的功能和数值证据，不是 Windows/Linux/浏览器、完整页面、Office/WPS、性能或安装包验收。开发产物大小记录在证据中，不能推算完整内核或 Musterwork 安装包。PPTX 既有导出/来源编辑回归通过，但本轮没有新增描边的 Office/WPS 原生编辑往返结论。

## 复现和下一步

当前复现先按 [Skia 构建](skia-component.md)生成配套 ABI 3 Native/WASM/sanitizer 组件（本文封存数值对应历史 ABI 2），按[开发说明](development.md)构建 CLI、worker、Rust WASM 与 TS。既有段落、页面和组合测试语料须已生成。

```sh
python3 tools/verification/skia-fixtures.py
node tools/verification/skia-parity.mjs
node tools/verification/path-raster-parity.mjs
node tools/verification/scene-raster-parity.mjs
node tools/verification/page-render-parity.mjs
python3 tools/verification/path-raster-reference.py
python3 tools/verification/scene-raster-reference.py
python3 tools/verification/page-render-reference.py
python3 tools/verification/stroke_reference.py
python3 tools/verification/stroke-previews.py
python3 tools/verification/contracts.py --path-raster-report .codex-work/path-raster/parity.json --scene-raster-report .codex-work/scene-raster/parity.json --page-render-report .codex-work/page-render/parity.json
```

继续将作者形状、连接线和 OOXML 的描边声明映射到同一条求值/绘制链，核实端点、连接、默认值与组合缩放在目标软件中的行为，再推进虚线、箭头、复合笔画与样式继承。页面文字/图片、完整高级对象、播放、Agent 接入和 Musterwork 替换门禁仍按原计划推进。
