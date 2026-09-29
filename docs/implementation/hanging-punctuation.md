# 原生段落行尾标点悬挂

2026-09-30。状态：共享文本计算与原生段落接线已实现，目标应用版式一致性继续验收。
本阶段解决真实模板启用 `hangingPunct` 后，渲染层遇到任意标点就拒绝整段文字的问题。
原文件、属性、字形和可编辑文字保持；不移除模板声明，也不缩小文字以通过排版。

## 语义与职责

[ECMA-376 Part 1](https://ecma-international.org/publications-and-standards/standards/ecma-376/)
的段落属性定义要求启用时允许应换到下一行的行尾标点留在当前行。
标准正文隐含默认 false；[Microsoft Office 实现说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/e05f6e35-a35d-4f00-924f-8172fe339bf5)
说明 Office 默认 true。已有来源级联遵循 Office 默认，本次没有改成标准默认来避开问题。

`mo-text` 使用明确的 `HangingPunctuation::End` 策略，来源段落负责从最终级联值选择策略。
旧独立段落请求没有新字段时仍为 None。当前策略允许一个逻辑行尾的标点扩展字素悬挂：
Unicode 固定版本的 CL、CP、EX、IS、SY、QU 或 Final_Punctuation，排除 Initial_Punctuation。
组合标记跟随所属字素；开标点、货币符号、连字符、空格和其他类别不由此策略悬挂。
这是一项明确的草案策略，尚不宣称覆盖 Office/WPS 的全部语言习惯、连续标点或禁则例外。

行搜索仍在每个候选的完整上下文中塑形，复用已选字形，不单独测量一个字符来猜测宽度。
只有常规候选超宽、标点位于实际视觉边缘、其余文字的 Q32 笔位置范围能放入该行时，
才接受悬挂。未解析字体、混合层级字簇、取消和预算错误仍阻止完整结果。
包含正文的连字不能被当作标点扣除，正文超宽也不能借悬挂变为通过。

`LineDecision.hanging` 返回实际来源范围和正文的精确笔位置范围。
原始字形、总 advance、轮廓和墨迹范围仍包含标点；对齐使用正文范围，
因此 RTL 视觉左侧的标点也有真实位置，居中和右对齐不再把允许悬挂的标点算入正文宽度。
容量统计在实际对齐后报告左右伸出量，同时保留行搜索的 overflow 判断。
允许的标点伸出仍是可观察的事实，不被伪造为无墨迹越界。

普通段落不额外调用塑形组件；只有候选含可悬挂行尾字素时才额外累计正文笔位置范围。
未新增线程、缓存服务、字体发现或产品依赖。Rust 原生和 WASM 使用同一算法，
SDK、页面绘制和保留播放沿用现有共享文本管线。

## 验证边界

相关四个 Rust 包的 629 项测试通过。真实 HarfBuzz 的段落框、容量与新增标点测试共 12 项通过，
包含已有取消、组件失败和原生属性拒绝回归。新增的合成字体只扩展项目自有字体的字符映射，
不引入系统字体或第三方字形；原始字体及其历史摘要保持。

14 份自有 PPTX 由真实来源编译器处理，其中 12 份发生悬挂，覆盖默认值、显式启用、
三种对齐、中文和希伯来文字。独立 Python 校验读取 PPTX 原生 XML 及字体 hmtx，
用 Fraction 推导字宽、正文边界和位移，不以 Rust 输出作为预期数值。
这验证计算和接线，不能替代 Office/WPS 的版式与编辑往返验收。

新的段落请求和结果字段由 Rust 生成 4 份 Schema 与 4 份 TS 类型；缺省输入保持原义。
本机旧 Corepack 的下载签名校验失败时，使用已经安装的锁定依赖直接执行生成器和 TypeScript，
没有关闭签名检查或改变依赖版本。

```sh
cargo test --locked -p mo-text -p mo-presentation-compile -p mo-kernel-api -p mo-pptx
MO_HANGING_EVIDENCE_DIR="$PWD/.codex-work/hanging-punctuation-20260930/fixtures-final" \
  cargo test --locked -p mo-harfbuzz-sys --test source_hanging --test source_frame --test source_capacity
python tools/verification/hanging-punctuation-reference.py .codex-work/hanging-punctuation-20260930/fixtures-final
```

真实模板的 OPC 修复、目录新版本发布、文字脚本识别、autofit、完整段落能力、
Office/WPS 实际观察和全部 PPT 替换门禁仍分别推进。
拉丁文字与某些东亚标点组合的 AmbiguousScript 来源诊断尚未解除；本轮没有忽略该诊断。
详细构建、跨端与产品激活状态以本轮证据和后续进度为准。

本轮 112 个真实 Native／WASM 段落请求的完整响应逐字节一致，覆盖 24 个悬挂行与
400 个字形；两份真实平台模板在新 Worker 的失败位置均从第 1 页推进到第 7 页的
`graphicFrame`，其中一份已确认是原生 chart 引用。整份预览仍失败，没有发布部分结果。
本轮候选尚未替换正在运行的产品固定交付物，见
[冻结证据](../reviews/evidence/2026-09-30-hanging-punctuation-verification.json)。
