# 原生文本框不自动换行

2026-09-30。状态：共享排版与原生/WASM 局部验收完成，尚未升级产品固定交付物。真实 Agent 创作曾因
模型设置 `wrap=false` 而在来源渲染阶段失败，问题属于内核的文字流计算。

依据 DrawingML [TextWrappingValues](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.textwrappingvalues?view=openxml-3.0.1)
中的 `none` 和 `square`，原生文本属性保持原值，由来源编译器显式映射到共享
`LineWrapping::NoWrap` / `Wrap`。不修改源文件，不把不换行解释为放大文本框、缩小字号
或把宽度设成无穷大。已有 authored `wrap=false` 经同一 PPTX 声明/来源渲染路径生效。

共享文本流在 NoWrap 下只选择 Unicode 必须换行的位置；忽略软断行机会，也不执行
紧急字素换行。显式换行、最后空行、空段落、所有字形和原文字仍保留。仍使用真实的
首行/后续行宽度进行溢出测量与左/中/右对齐，允许超宽文字得到负的居中/右对齐位移。
原来的悬挂标点策略、字体验证、取消和计算预算仍适用，不用不换行掩盖其他未实现内容。

公开段落请求新增可选 `wrapping` 字段，缺省为 Wrap；历史请求行为保持。源文本框的
最终 wrap 属性进入实际 frame 计划，原生、WASM、SDK、静态绘制和保留播放共用计算。
每个强制行在 NoWrap 下只塑形一个完整候选，避免为不会采用的软断行重复测量；这不是
整份文稿的性能结论。没有新增依赖、字体发现、存储或产品 UI。

## 验证

相关 mo-text、mo-presentation-compile、mo-kernel-api、mo-pptx 的 635 项测试通过。
真实 HarfBuzz 的 frame、capacity、hanging 测试 13 项通过；新增六种左/中/右对齐和
RTL 组合验证原生 `a:br`、空末行、三行内容、真实横向溢出、原文件不变及宽框同字形。
窄框启用自动换行时产生更多行，关闭自动换行时保持同一内容与字宽。

严格 scoped Clippy 和生成 Schema/TypeScript 检查通过。公开的两份 Schema 与两份
TS 类型从 Rust 同步生成。首次误用旧 Skia 构建目录被摘要校验拒绝；随后使用现有匹配
源码的固定构建输入通过，没有跳过组件校验。

匹配 SDK/Worker/播放包的产品激活与 Office/WPS 实际观察仍需分别记录；本实现
不解决文字对象间重叠、原生图表或完整模板验收。

128 个真实 Native/WASM 段落请求的完整响应逐字节一致，其中 NoWrap 仍如实报告
40 个横向超宽行。六个原生来源文本框分别经过文本页和资源页两条路径，共 12 对
整页容量报告和 RGBA 像素完全一致，包含实际强制换行和不同对齐/RTL 设置。

```sh
node tools/verification/no-wrap-parity.mjs .codex-work/nowrap-20260930 < /dev/null
```

该检查使用固定自有字体、相同原生与 WASM HarfBuzz/Skia 输入。脚本的有限输入使用
空标准输入，避免 Node 在计算完成后因开放的控制台输入继续驻留；没有强制退出掩盖
计算或断言失败。绘制验证使用支持 PPTX 页绘制协议的 mo-raster-worker。
构建摘要与局部门禁见[冻结记录](../reviews/evidence/2026-09-30-no-wrap-verification.json)。
