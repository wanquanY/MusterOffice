# 来源段落与字符样式

2026-09-25。本阶段新增真实 PPTX 的 Rust 样式计算层，将[来源文本](source-text.md)中的段落、列表层级、字符声明和母版关系连接起来。入口为 [`cascade::resolve`](../../crates/mo-presentation-source/src/source/text/cascade.rs)，供后续来源文字编译器调用；本阶段没有新增 CLI/WASM 导出或 Agent 操作，也没有接通页面文字绘制。

## 数据与职责

输入是实际包读取所得 `SourceIndex`、预期源摘要、对象引用、预算与取消检查。源摘要用于拒绝过期请求；不能用它证明外部伪造的索引可信。宿主应从受控的原包生成索引。

输出保留段落/字符的物理序号、原始 run 下标与类型，以及属性值和逐项来源。正文继续引用 `SourceObject.paragraphs`，不在样式结果中重复保存。段尾 `end_style` 独立计算，供插入文字或空段落的后续计算使用，不覆盖现有 run；也不声称已经实现编辑器的光标格式行为。普通文字、硬换行、字段和空段落保持各自绑定。

段落的八类子声明和字符的十三类子声明按属性族选取：行距、段前后距、项目符号颜色/大小/字体/类型、制表位，以及字体、填充、描边、效果、下划线、超链接等。互斥选择整体覆盖，显式空制表位、`buNone`、`noFill` 和空字体名称不会被当作缺省值。所选结果是带来源的 `TextStyleDeclaration`，通过 `declaration` 取回完整原生节点。**选中声明不等于求值成功**：其子树中的保留内容、主题字体、资源、颜色、效果和交互仍须由相应计算层处理，超链接不会在这里执行。

## 当前继承配置

配置名为 `drawingml-text-cascade-draft-v1`。从局部到回退依次为：当前段落 `pPr`、对象对应级别的 `lstStyle`、已匹配版式占位符的同级段落/列表样式、基础主题 `txDef`/`lnDef`/`spDef` 的同级列表样式、已匹配母版占位符的同级段落/列表样式、母版对应 `titleStyle`/`bodyStyle`/`otherStyle`、演示文稿 `defaultTextStyle`、配置默认值。字符在上述各级的 `defRPr` 前应用自己的 `rPr`；段尾则使用自己的 `endParaRPr`。

段落级别只由当前段落的 `lvl` 决定，未写时为零；`lvlNpPr` 自身的 `lvl` 不重新选择级别。Office 忽略 `defPPr`，所以该节点不参与计算。对应级别缺失时不擅自回退到第一级。参见 [Office pPr 继承说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/3bf4b4b9-d1cd-4610-be56-9427e22c5fb4)、[defPPr 说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/18292376-7677-4792-8422-8024b97ec52f)和[列表级别差异](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/9b34280e-538e-4811-8af9-761d34f88f20)。

母版样式分类复用既有占位符分类函数，不另外维护一份映射。普通文本框选 `otherStyle`，不会从版式的任意形状继承。模板提示文字的 run 和段尾样式不作为子页面正文样式；模板内同级段落多于一个时明确返回歧义，不随意取第一项。

字号回退为 1800，缺省 kerning 保持 `None`（关闭，区别于显式零阈值），smartClean 为 true，baseline 为零。显式 false、零和精确词法均保留。language、alternativeLanguage、bookmark 可以保持未指定；语言补足与 alternate-language 派生尚待字体/编辑层连接。依据 [Office rPr 差异](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/953e072c-f234-4373-b017-3f1f71eac93b)和 ECMA-376 Part 1 的段落、字符与默认文字样式章节。

以上组合仍是明确的 draft 配置，尤其是版式模板段落、主题默认项、母版占位符与演示文稿默认项冲突时的优先级，尚未取得 Office/WPS 实测证明。独立参考核对的是该配置，并不是从目标应用反推得到的完整行为。

## 预算与诊断

每次解析一个对象，默认最多 1024 段、4096 run、100 万步和 4 MiB 累计词法/引用分配记账。该记账不是序列化长度或峰值 RSS 承诺。每个使用到的列表级别只建立一次继承链，最多九条；重复段落复用该链，保持结果的原始来源。

源摘要不符、非法对象/父级绑定、目录结构冲突、超预算或取消会使整个调用失败，不返回半份成功结果。未匹配/歧义占位符、未知的参与级别属性/子节点、字段自己的 `pPr`、尚未解释的 `xml:space` 等返回类型化 `unresolved`。未参与计算的其他级别和被忽略的 `defPPr` 不影响当前结果。该策略不等于通用文件有效性检查。

## 验证与继续接线

13 项新增 Rust 用例覆盖局部/列表/版式/主题/母版/演示文稿覆盖，九级与 180 个重复段落，段尾隔离、空段落、字段、换行、显式清除选择、真实包修改、中途取消、预算和损坏绑定。兼容分支用例另以物理 XML 元素计数核对来源位置。

独立 Python ZIP/XML/MCE 参考检查 24 份实际输入：18 份完成样式计算、6 份未决；核对 199 段、400 个字符样式、9789 个属性值、8194 个属性来源和 9 个子声明选择。固定官方 Transitional XSD 校验 120 个输入部件，其中 116 个通过，四个故意非法探针分别为 `pPr`/`defPPr` 未知属性、缺少图片子元素的项目符号和 `xml:space`。其中忽略 `defPPr` 的计算成功不代表该探针符合 XSD。

在仓库根目录运行：

```sh
MO_TEXT_CASCADE_EVIDENCE_DIR="$PWD/.codex-work/text-cascade/fixtures" cargo test -p mo-pptx --test text_cascade --locked
python3 tools/verification/text-cascade-reference.py
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p mo-pptx --target wasm32-unknown-unknown --locked
```

参考脚本需要此前固定的 ECMA XSD 缓存与 lxml；先校验官方 ZIP 及其中全部 26 份 XSD。测试证据目录必须为绝对路径。该阶段的[库级检查记录](../reviews/evidence/2026-09-25-text-cascade-library-verification.json)单独记录本轮证据，保留此前的[文本框 Native/WASM 冻结基线](../reviews/evidence/2026-09-25-text-body-verification.json)。本轮新增计算尚无真实 Native/WASM 调用对比，不把此前 10064 批检查算作新能力验收；也没有新的发布构建体积或产品性能结论。

后续库级工作已补充对象/版式/主题默认/母版的 `font_reference` 来源，以及[原生字体与显式资源绑定](native-fonts.md)。上述 413 项与 24 份参考记录保留为本阶段历史证据。接下来将字号/基线/语言等映射到已有段落布局与字形轮廓管线。文本框属性、段落属性、字体/画笔必须一起处理，不能仅让文字出现在页面上就声称保真。当前来源页面仍拒绝含文字的可见对象；高级内容、播放、Agent 产品接入和 Musterwork E0–E3 仍未完成。
