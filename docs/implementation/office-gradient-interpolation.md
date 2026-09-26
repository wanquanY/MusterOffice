# Office 渐变插值与 JSON 浮点输入

2026-09-25：原生渐变颜色链路新增明确的 `OfficeGamma1875` 插值模式；不符合条件的色标继续采用已有 sRGB 插值。此阶段修复的是颜色求值，路径渐变几何、固定方向及完整 Office/WPS 互操作仍未完成。

## 已核实的规则与边界

微软 Open Specifications 员工公开说明了特定双色、对称三色渐变的逐通道插值规则：端点为 0/1 的双色，以及端点颜色相等且中间位置严格处于区间内的三色，使用指数 1.875 的 RGB 曲线；alpha 保持线性。其他色标布局保持普通插值。见[微软说明](https://learn.microsoft.com/en-us/answers/questions/2248059/non-preset-a-tilerect-behaves-strange-in-case-of-g)。

本实现显式比较完整的 RGBA 工作颜色。目标应用对端点 alpha 不等、扩展工作颜色等细节仍须实测；公开说明和数学参考不能替代 Office/WPS 接受度。

设相邻颜色为 A/B，局部位置为 t。上升通道使用 `1-(1-t)^(15/8)`，下降通道使用 `t^(15/8)`，再在该通道的 A/B 之间插值。对称三色分成两段，第二段从末端回到中间颜色；三色中点不会改变端点判定。

## 共享绘制实现

`mo-raster` 对明确请求的新插值模式验证精确端点、色标数量、严格中点及直通 alpha；不能满足条件就报错。来源编译器根据已求值色标选择模式。判定在 float32 转换前执行，原始颜色、位置及原生 PPTX 声明继续保留。

底层仍复用 Skia 的直线、径向、独立平面几何、平铺和 decal。一个固定 CPU 阶段把标量位置转换成逐通道颜色，并在最终颜色输出时预乘 alpha；decal 遮罩与颜色 alpha 分开处理。指数 15/8 用三次平方根与整数乘方求值，避免依赖平台 `pow` 的不同实现；同一像素的上升/下降曲线供三个颜色通道共享。没有渐变图片、逐色标细分、运行时编译器或新增 C++ 依赖。

新帧 V10 复用 V9 的 14 字头、8 字绘制项及最大 2,883,662 字预算，在渐变的 interpolation 字增加值 2。可选能力为 `mo_skia_office_gradients_abi()==1`。Native/TS 在调用前检查能力；旧组件拒绝 V10 后仍可执行已支持帧。既有普通 sRGB/linear-sRGB 请求仍生成原先帧。

## 公共 JSON 根因修复

边界公共请求实际暴露了 `serde_json` 默认快速浮点路径把 `0.9999999999999999` 读成 `1.0` 的问题。这会错误改变色标端点和端点颜色的相等关系。

工作区保留固定 `serde_json 1.0.151`，启用其 `float_roundtrip` 功能；无新增包或锁文件版本变化。保留修复前 WASM，针对完全相同的近端点位置和近端点颜色请求重放，用真实组件核对修复前进入绘制、修复后在调用后端前拒绝的区别。十进制来源 PPTX 另在精确比值转 binary64 时拒绝将非端点压到 1.0，避免 XML 路径重现同类问题。

## 验证与复现

[阶段证据](../reviews/evidence/2026-09-25-office-gradient-verification.json)将源码、构建与以下检查绑定：

- 595 项 Rust 测试、严格 Clippy、格式和 80 份 Schema 检查通过。6 份 Schema 及其生成 TS 只扩展插值枚举，其余保持一致；固定依赖版本未变。
- 157 对通用 Native/WASM 请求包含直线、径向、独立平面、四种平铺、透明度、扩展工作色、三色、裁剪与快照。独立 double `pow` 参考核对 314,568 个像素，最大通道差为 1 色阶；9 项无效输入均不调用后端。
- 10 份自有 PPTX、70 个官方 XSD 部件通过；独立原始 XML 参考核对 1,200,000 个像素，最大通道差为 1 色阶。121 对来源公共请求通过，其中 99 对成功；95 个旧样本逐字节不变，16 个旧渐变样本仅更换颜色插值及关联摘要，几何帧保持相同。
- 398 组直接 Native/WASM/ASan/UBSan 检查包含 228 个旧帧及 22 个无效帧。未运行 LeakSanitizer。旧 V9 组件拒绝两种入口的新帧后仍可用。
- 854 对旧绘图请求及 307 对旧页面/文字请求不变；两个近端点 JSON 回放保留修复前/后的真实 WASM。
- LibreOffice 26.2.0.3 能打开全部 10 份 PPTX 并导出两页 PDF；双色/对称三色的 RGB 差异可达 48 色阶，透明度样本也有明显差异。这些是记录的外部差异，不是 Office/WPS 一致性证明。

原始 Rust WASM 为 **6,261,069 字节**，比上一阶段增加 46,623；C++ WASM 为 **2,327,536 字节**，增加 2,273。Rust 增量包含正确浮点解析功能；这些是未压缩模块大小，不是安装包或整页性能测量。

本阶段使用 `.codex-work/office-gradient`；以前阶段的产物不覆盖。复现入口：

```sh
python3 tools/verification/office-gradient-checks.py
node tools/verification/office-gradient-parity.mjs
node tools/verification/office-gradient-source-parity.mjs
node tools/verification/office-gradient-components.mjs
node tools/verification/office-gradient-regressions.mjs
node tools/verification/office-gradient-json.mjs
python3 tools/verification/office-gradient-reference.py
python3 tools/verification/office-gradient-evidence.py
```

完整独立 C++ 构建继续使用 `tools/components/build-skia.py` 与锁定的 Native/WASM/ASan 图像解码依赖。上面的 JSON 回归还需要本地封存的修复前 WASM；它不是从当前源代码重新构建的旧版快照。一般页面/文字回归继续使用共同 `text-page-runtime-regressions.mjs`。文件结构通过不等于应用显示一致，原始应用差异不豁免。

## 后续路径渐变的调查修正

同一微软讨论后续在 2025-04-30 修正了径向渐变中心的边界说明，指出实际几何路径边界参与计算；这与 2025-04-16 早期回答的逻辑矩形说明存在差别。该修正尚未实现为本项目的路径渐变规则，后续需结合自有几何样本验证。不能把既有线性渐变的接收形状矩形直接套到径向模式。

微软亦承认自定义 tileRect 与路径渐变组合存在 Office 实现问题。内核仍需明确几何语义、精度与兼容证据，不能通过截图或普通径向渐变替代完整原生路径功能。完整高级内容、生产 Agent 分发和 Musterwork 替换门槛保持未完成。
