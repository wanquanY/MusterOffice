# 原生线条颜色计算

2026-09-24：已把真实 PPTX 的[线条继承](line-styles.md)接入共享的[原生颜色计算](source-colors.md)。同一次来源读取、同一组显式规则与预算产生有效线条、颜色依赖和工作精度通道，Native/WASM 返回完全相同的结果。原始声明与文件均不被补值或改写。

这一步提供来源页面编译所需的颜色数据。完整画笔、来源几何/页面绘制及目标应用保真仍未完成，不能视为 Musterwork 替换候选。代码、输入/输出和回归绑定见[封存证据](../reviews/evidence/2026-09-24-line-color-verification.json)。

## 共享计算与接口

`source::color::Session` 复用一个表面的有效颜色映射、颜色方案、宿主上下文和整批预算。既有主题色查询与新的对象线条查询使用同一个解析器和数学实现；没有另建线条专用的色彩公式。

`mo-pptx::source::line::colors::query` 先取得带完整来源的线条样式，再按需计算被选中的纯色表达式。Native CLI 是 `pptx-line-colors <query.json> <source.pptx>`，WASM 是 `resolve_pptx_line_colors(request, source)`。公开字节入口先检查真实包和来源摘要；纯 Rust 查询可复用已经检查的不可变索引。

请求包含 `expectedSourceSha256`、`surface`、`objects`、`lineProfile`、`colorProfile` 和 `context`。必须显式选择 `ms-oi29500-lines-2024-draft-v1` 与 `ecma376-2016-draft-v1`；两者依然是 draft，不表示某个 Office/WPS 版本已通过验收。

新合同为 `pptx-line-color-query` / `pptx-line-color-response`。每个对象同时返回原有 `style` 与单独的 `paint`：

| 结果 | 含义 |
| --- | --- |
| `paint.kind=none` | 有效声明为 noFill；不求值未使用的引用颜色 |
| `paint.kind=unresolvedStyle` | 线条继承本身未解析，具体原因保留在 style |
| `paint.kind=solid`，outcome resolved | 纯色已按显式数值规则求值，附通道、采样、依赖与提示 |
| `paint.kind=solid`，outcome unresolved | 线条已解析，但缺颜色上下文、存在循环或数值不可表示 |

`resolved` 颜色提供非预乘的 `srgb`、`linear` 四通道及 `rgba8` / `rgba16` 诊断采样。工作通道保留浮点精度和超出 sRGB 显示色域的值，采样才执行显示范围裁剪与量化；`clippedForSrgb` 单独报告这种裁剪。不是所有原生线型或效果都因此变成可绘制。

## 占位色组合不丢精度

原先主题色入口接受调用方给定的 RGBA8 占位上下文，不能用它承接另一条原生颜色表达式的中间结果。新的共享会话直接接收借用的原生颜色值与变换序列，递归返回完整 `mo_color::Color` 工作状态。主题引用、样式引用和外层变换之间不会先采样再重建颜色。

实际用例验证了两个会被提前转换破坏的情况：

- 引用色红通道为 1/255，引用内和外层分别调制 50%，最终是 0.25/255，RGBA8 为 0。若引用边界先量化，结果会变成 1。
- 引用为蓝色色相、饱和度 0 的 HSL，外层再把饱和度设为 100%，结果恢复蓝色。若中间转成灰色 RGB 再求 HSL，原色相将丢失。

占位表达式按需执行。对象直接颜色不使用 `phClr` 时，引用中的缺失系统色或循环不会触发无关错误。外层 `phClr` 优先使用线条继承提供的原生占位表达式；若该表达式自身仍引用 `phClr`，则使用显式 `context.placeholder`。没有提供所需上下文时返回 `missingPlaceholder`，不会自引用递归或猜测黑色。

该上下文优先级是当前 draft 的明确解释，仍需目标应用验证。`schemeClr` 的原生表示和有序变换见 [Microsoft SchemeColor](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.schemecolor?view=openxml-3.0.1)；数值操作选择及已知差异继续遵循[来源颜色规则](source-colors.md)，没有因新增入口改变旧结果。

## 来源、错误与资源

线条来源继续记录对象/主题部件、原生 ID、物理元素序号和 style index。颜色输出另记录主题色节点、系统色来自显式宿主还是文件 `lastClr`，以及实际触发的占位引用顺序。主题循环检测贯穿占位表达式，不在样式边界清空活动引用栈。

没有对应数值的颜色返回逐对象诊断；颜色失败不把整条原始线条声明丢弃。取消、源摘要冲突、非法请求及预算超限仍使整批失败，不发布已算出的半批结果。工作通道在序列化前验证有限性，不产生非有限 JSON 值。

同一批次同时受线条继承预算与颜色预算约束。默认最多 256 个对象请求；颜色阶段共享 65536 步和 64 KiB 百分比词法计算量，主题引用和内外变换均计入。线条阶段及底层 OPC/XML/MCE 的已有预算继续生效；没有为了颜色接通而取消来源验证、取消检查或资源限制。

生产实例池、跨任务缓存、浏览器 Worker 和完整资源生命周期仍需继续实现。本阶段没有测量端到端性能或峰值内存收益。

## 实际验证

全仓 **277 项 Rust 测试**通过，相对上一阶段新增 8 项。严格 Clippy、rustfmt、53 份 Schema 与 TS 类型检查通过；此前 51 份 Schema 和生成 TS 文件保持不变。既有主题色查询的完整响应、依赖顺序和诊断均保持原结果。

新增 90 份自有实际 PPTX：74 份将已有主题颜色语料连接到真实对象线条，16 份专门覆盖占位表达式组合、工作状态、按需求值、显式宿主上下文和跨样式循环。另复用上一阶段 28 份继承样本。新增 247 批 Native/WASM 检查，包含 116 批真实叶文本编辑后的重读，响应和候选文件字节一致。

独立参考从 ZIP/XML 关系及原始声明重建线条、有效映射和颜色方案，采用 64 位有效数字的 Decimal 与不同的 HSL 算法进行颜色组合。235 批成功查询共核对 589 个对象，其中 541 个颜色数值可解析、16 个颜色明确未解析，其余为 noFill 或未解析线条。覆盖六种颜色表示、28 种变换及 190 个预设色。

预先设定 RGBA8 精确一致、RGBA16 最多 1 个码值差；本次两种采样实际差值均为 0。工作颜色通道预设绝对/相对容差均为 2×10⁻¹²，实际最大绝对差约 5.2736×10⁻¹⁶。该精度检查支持当前数值规则，不证明目标应用颜色相同。

90 份新增文件中 718 个部件通过官方 ECMA XSD。另有两个沿用的缺颜色映射探针故意省略母版必需的 `clrMap`；XSD 确认这两个部件非法，单独列出，不混入正向合规统计。它们检查计算诊断及无需映射的直接色槽行为，来源读取成功仍不代表整个文件符合 XSD。

116 份编辑候选除授权的文字内容之外，其余 XML 和部件保持不变。此前 2969 批主内核用例在当前二进制上全部重新执行且结果不变，累计 **3216 批**。旧几何参考按真实帧摘要复用；197 批 Skia 组件/sanitizer 与 61445 个参考像素继续复用未变的组件源码/二进制证据，本轮未重新执行组件测试。

本轮没有新增 WPS、PowerPoint、LibreOffice 运行、应用编辑往返或来源页面像素对照。上一阶段的占位色、空虚线、默认值及继承引用差异继续保持开放，不因数值参考通过而关闭。

没有新增外部依赖或锁文件变化。未压缩 CLI 为 4514496 字节（增加 35552），Rust WASM 为 4011406 字节（增加 21186），JS glue 增加 768 字节。文字 worker、两个 C/C++ WASM 组件和薄适配器字节未变；raster worker 大小仍为 3634992 字节但摘要变化，已重跑新产物上的渲染回归。这些是未完成内核的开发产物，不能外推完整内核或 Musterwork 安装包大小。

## 复现和下一步

按[开发说明](development.md)构建 Native/WASM，并准备已有主题颜色、线条继承语料、标准预设色事实表和官方 XSD。Python 的 lxml、Decimal 与 jsonschema 仅用于开发验证。

```sh
python3 tools/verification/line-color-fixtures.py
node tools/verification/line-color-parity.mjs
python3 tools/verification/line-color-independent.py
python3 tools/verification/contracts.py --line-color-report .codex-work/line-colors/parity.json
```

`line-color-evidence.py` 核对完整回归、真实产物、输入/输出和历史摘要；默认只验证，`--seal` 独占创建新证据。全量复现需要执行历史各入口，不能把旧报告替换产物摘要后当作重新运行。

后续已补齐[来源几何声明](source-geometry.md)，继续推进几何求值与填充，并把有效样式和颜色连接到页面编译。完整页面文字/图片、复杂画笔与效果、可编辑高级对象、播放及 SDK/MCP/Skill/Plugin/Musterwork E0–E3 仍在原目标内，未缩减为当前可解析的线条范围。
