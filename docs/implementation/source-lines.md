# 原生线条声明与缺省值保留

后续实现已将线条渐变/图案升级为[共享填充声明](source-fills.md)，并继续明确区分声明读取和画笔计算。本文下述 `retained` 表示及验证数字保留当时阶段的历史基线，当前实现以共享填充与最新进度为准。

2026-09-24：真实 PPTX 中的对象线条、主题线条条目和对象 `lnRef` 已进入共享来源索引，并由 Native/WASM 的读取及叶文本编辑入口实际执行。作者模型同时保留缺省尖角限值；没有引入 HTML/CSS 文档表示或改变既定技术路线。

这一步提供完整样式解析所需的原始声明。线条继承、默认值求解、导入页面渲染及 Office/WPS 的尖角效果尚未验收。实际结果与产物绑定见[封存证据](../reviews/evidence/2026-09-24-source-lines-verification.json)。

## 声明与有效值分离

`SourceObject.line` 读取形状、图片和连接线的直接 `p:spPr/a:ln`；`SourceObject.lineReference` 读取直接 `p:style/a:lnRef`。主题格式方案的 `SourceStyleEntry.line` 使用相同的有界解析器。对象组和 graphicFrame 不错误套用这些位置；保留子树中的同名节点也不会成为有效声明。

| 原生内容 | 来源记录 |
| --- | --- |
| 线宽、端点、复合线、对齐 | 可选整数 EMU；三个 cap、五个 compound、两个 alignment 原生枚举 |
| 填充 | noFill、可选颜色的 solidFill；渐变/图案为明确的 retained 来源，不假装已经求值 |
| 颜色 | 与主题共享六种颜色表示、28 种有序变换和精确百分比记录 |
| 虚线 | 11 种预设；保留空预设声明、空自定义列表及有序自定义 dash/space 百分比对 |
| 连接 | round、bevel、miter；miter 限值可缺失，不在读取时补值 |
| 箭头 | head/tail 分别保留可选类型、宽度、长度；六个类型、三个尺寸 |
| 样式引用 | 必需的原生 uint32 索引、可选颜色；此阶段不将索引解释成最终样式 |
| 未建模内容 | 原始部件字节及物理元素绑定；未知属性/扩展拥有显式保留序号 |

属性类型、子元素顺序和可选项依据官方 ECMA-376 Transitional XSD，声明结构也见 [Microsoft LinePropertiesType](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.linepropertiestype?view=openxml-3.0.1)。读取区分没有 `a:ln`、空 `a:ln`、没有 `a:miter`、空 `a:miter` 和 `lim="0"`。`lnRef` 的颜色和 solidFill 的颜色本身均可缺省，不能按常见样本擅自变成必需字段。

来源百分比保持原生词法：`+000400000` 和 `400.123456789%` 不经浮点转换，也不重写成另一种格式。线宽验证 `[0,20116800]`；原生整数百分比及小数百分比采用共享的精确范围验证。每项声明带有源元素序号，所属部件由索引及其摘要绑定；MCE 逻辑投影不改变物理序号。

颜色类型与原生枚举已从 `source/theme/` 移到 `source/drawingml/`，主题和线条共用一份实现。旧主题公共类型导出继续保留，不为线条复制第二份颜色规则。

## 作者缺省尖角限值

作者 `LineJoin::Miter.limit` 改为可选。已有显式值的 JSON 和 PPTX 表示不变；`{"kind":"miter"}` 导出为 `<a:miter/>`，`{"kind":"miter","limit":0}` 导出为 `<a:miter lim="0"/>`。二者经过真实原生文件导出和重新读取后仍能区分。

页面编译遇到缺省限值返回既有 `unresolvedStrokeParameters` 诊断，不擅自补为 0、斜角或某个应用的默认值。显式限值继续使用上一阶段的作者映射；新增的[截断尖角原语](miter-clip.md)没有被未经验证地套入 Office/WPS profile。原有 131 组作者页面的像素、请求、计划和响应均保持原摘要；新增一组明确的缺省限值拒绝验证。

## 有界读取与来源编辑

对象与主题共享全包线条预算，默认最多 100 万个投影线条元素、32 MiB 属性值。存储前检查预算；底层 XML/MCE/OPC 的物理元素、字节、深度与取消限制仍同时生效，未选择分支不能绕过底层预算。

解析器验证已知属性取值、重复声明和原生子元素次序。未知原生子元素或命名空间返回兼容性错误；渐变、图案和扩展的未建模子树明确保留，不能借其中的 `ln` 或颜色节点污染当前解析状态。读取成功仍不等于整份文件通过完整 XSD 验证。

真实叶文本编辑只修改授权目标，候选包重新建立来源索引。线条声明、扩展内容、未改动部件及原有编码继续保留。本阶段没有开放线条属性编辑，也没有把来源投影当成完整 Document/SourceBinding。

## 实际验证

本阶段全仓 261 项 Rust 测试通过；相对上一阶段新增 8 项。严格 Clippy、rustfmt、49 份 Schema 及 TS 类型检查通过。作者模型与来源响应共影响 8 份 Schema，其余 41 份不变。

77 份实际合成 PPTX 包含 59 份成功输入、18 份非法声明。Native/WASM 运行 77 批读取和 59 批真实文本编辑，来源响应与编辑候选字节一致，共 136 批。独立 ZIP/lxml/MCE 树参考核对 1175 条线条声明、10 个线条引用和物理来源绑定；修改后全部非目标文本 XML 及其他部件保持不变。

471 个投影部件通过官方 XSD；一个刻意包含未知属性的部件明确排除，未混入正向合规统计。18 份非法输入亦由 XSD 独立确认非法。另有 3 份作者 PPTX 验证限值缺省、0、400000 的原生导出与重新读取，30 个部件通过 XSD；这 3 份辅助文稿不重复加入主内核批次。

旧来源回归仅新增 `line`/`lineReference` 字段及更新主题提示；剔除这几个明示的合同新增项后，既有字段、来源身份、变换、文字和编辑输出保持不变。42 份旧主题成功输入经更新后的独立 XML 参考重新校对。其他文档/文件/文字/路径/场景回归保持既有结果。

主内核累计 2825 批 Native/WASM 逻辑用例，包含新增的 136 批来源和 1 批页面拒绝。组件二进制、适配器及源码摘要未变，上一阶段 197 批组件/sanitizer 和 61445 个组件参考像素证据按摘要复用，本轮没有将其记作重新运行。旧解析几何参考同样按实际帧摘要复用。

没有新增外部依赖。未压缩开发产物 CLI 为 4403872 字节（增加 38608），Rust WASM 为 3922028 字节（增加 31058）；文字与 Skia WASM 组件及薄适配器不变。raster worker 为 3634992 字节（减少 16400），仅记录链接产物变化，不据此宣称性能优化。这些数值不包含完整一期能力，也不能推断 Musterwork 安装包体积。

## 复现与后续

按[开发说明](development.md)构建同版本 Native/WASM、生成既有来源/颜色映射语料和官方 XSD 缓存。开发 Python 需要已记录的 lxml、python-pptx 和 jsonschema；它们不是运行内核的依赖。

```sh
python3 tools/verification/source-line-fixtures.py
node tools/verification/source-line-parity.mjs
node tools/verification/source-line-author.mjs
python3 tools/verification/source-line-independent.py
node tools/verification/page-render-parity.mjs
python3 tools/verification/contracts.py --source-line-report .codex-work/source-lines/parity.json --page-render-report .codex-work/page-render/parity.json
```

`source-line-evidence.py` 在全部既有回归、编译与检查日志生成后，核对真实产物、输入/输出、源码、模块迁移和历史摘要；默认只检查，`--seal` 用独占创建写入新的证据文件，拒绝覆盖历史记录。全量复现时须重跑各实现文档中的既有回归，不能把旧 artifact digest 直接复制到新报告。

后续已将声明连接到[带来源的线条继承与显式默认值查询](line-styles.md)；来源页面编译继续推进。页面文字/图片、完整画笔与效果、可编辑高级对象、播放及 Agent/Musterwork 接入仍按完整一期目标推进。本轮没有新增外部应用运行或编辑往返证据，不关闭此前的尖角兼容性缺口。
