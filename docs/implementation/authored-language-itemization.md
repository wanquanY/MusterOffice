# 文稿语言参与脚本消歧

2026-09-30。状态：源码、局部原生/WASM 和真实模型产物回放验证完成；产品激活单独记录。

明确标注 `lang="zh-CN"` 的 `PPT。` 在旧来源编译器中仍被 `AmbiguousScript` 拦截。
句号的 Unicode Script_Extensions 有多个候选，而 Latin 上下文无法决定它；原实现没有
把文稿已经提供的语言信息交给分段器。保留内容、字体和语言的完整导出回放复现了失败。

现在 `itemize_with_languages` 接受显式语言的标量范围，来源编译及实际段落塑形共用它。
只有上下文尚未确定、且存在多个兼容脚本时才使用语言提示；提示必须属于候选集合。
文字本身确定的脚本、组合序列、隔离域和配对符号处理优先。提示冲突不会拆分字素，
没有依据或候选不兼容时仍保留原诊断；不采用系统语言、字体名称或私有扩展猜测。

这是有版本的计算策略，不是 Unicode 规定的唯一算法。依据
[UAX #24 的实现建议](https://www.unicode.org/reports/tr24/tr24-41.html#Implementation_Notes)，
语言作为额外文稿上下文用于处理尚有歧义的片段。按
[BCP 47 语法](https://www.rfc-editor.org/rfc/rfc5646.html#section-2.1) 读取开头的 script
子标签，不扫描 extension/private-use；无显式 script 时，仅为已声明的 zh/ja/ko 使用
本策略固定的 Hani/Kana/Hang 提示。Hans/Hant、Jpan、Kore 分别映射到上述 Unicode
脚本。提示仅在候选集合允许时生效，例如中文不能决定只属于 Hira/Kana 的长音符。
没有引入通用语言识别、likely-subtags 数据库或完整 IANA 注册表校验。

每个语言标签最多 255 字节，语言范围数量受现有 itemization 输出预算约束。范围须
完整覆盖文本；允许 XML run 边界位于字素内部，但冲突提示不能改变该字素的原诊断。
只读主语言；缺失语言和无法由主语言决定的内容仍可能需要后续明确策略。altLang 的
字体选择行为保持，未把它冒充主语言传给塑形器。没有新增依赖、权限、持久状态或页面。

已有 JSON 请求中的 language 字段直接生效，无新必填字段；底层无语言 itemize 入口
保持 v1。带语言的分段结果使用 `unicode18-script-bidi-grapheme-language-items-v2`，
段落塑形响应使用 `unicode18-auto-items-hb14.5-language-reshape-v3`。这些仍是未发行的
开发合同；组件摘要必须整体更新，不能将旧 WASM/Worker 作为本策略的验证结果。

## 验证范围

- 108 项文字、91 项编译单元测试、11 项来源文字、3 项实际原生悬挂/混排测试及 5 项
  kernel-api 单元测试通过；严格 scoped Clippy、格式与差异检查通过。
- 正向覆盖 zh/ja/ko、显式 script、上下文优先、多语言片段、字素内冲突、双向隔离域；
  反例覆盖缺失/无效/不兼容语言、私有扩展伪装 script、预算和逐检查点取消。
- 真实 HarfBuzz Native/WASM 的 54 份完整塑形/行布局响应一致，包括保留歧义的反例。
- 真实模型 V7 两版文稿重算：六个 PNG 与两个 PPTX 均逐字节相同。另通过公开原子编辑
  仅将封面标题改为 `PPT。`，旧 Worker 复现歧义，新 Worker 导出成功；未改语言或字体。
  三份交付检查和九页完整元数据、容量、RGBA 在 Native/WASM 一致。
- 当前 macOS arm64 Worker 12,489,648 字节，比字体复用版增加 17,520 字节。该数字
  不代表完整桌面包、内存或跨平台性能；没有降低字号或图像化文稿。

证据：[文稿语言验证](../reviews/evidence/2026-09-30-authored-language-verification.json)。
Office/WPS 的语言相关字体选择及字形互操作仍需独立验收；含图表模板仍不因此通过。
