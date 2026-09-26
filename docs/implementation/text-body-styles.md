# 来源文本框继承

2026-09-25。本阶段把[原生文本声明](source-text.md)中的文本框属性转换成带来源的有效值，并读取主题 `objectDefaults` 的文本默认项。这是连接真实 PPTX 与段落布局的中间计算层；段落/run 继承、字体绑定和来源页面文字绘制继续推进。

## 计算边界

Rust [文本框计算](../../crates/mo-pptx/src/source/text/body.rs)按属性求 19 个文本框值，包括内边距、旋转、分栏、方向、锚点、换行、溢出与兼容标志。输出 `attributes` 和类型化 `origins`，逐项区分原生对象、主题部件/默认项和配置默认值。缺省、false、零以及精确坐标词法不会在来源索引中改写。

`drawingml-body-inheritance-draft-v1` 明确采用以下计算顺序：当前对象、匹配的版式占位符、当前有效基础主题中的 `txDef` / `lnDef` / `spDef`、匹配的母版占位符、配置默认值。每项只选第一个显式值。直接查询母版对象时，它自身的声明仍是局部声明。普通非占位对象不会从版式上的任意文字对象继承。

主题文本框优先顺序与母版回退依据 [MS-OI29500 bodyPr 说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/b3da041f-d56f-464b-a9b3-2e3274de9caf)，页面/版式覆盖依据 ECMA-376 Part 1 Annex L.3.2。这里将规则组合成显式 draft 配置，**尚未取得 Office/WPS 对所有冲突组合的实测证明**。

三类 autofit 以所选声明整体继承：`noAutofit`、`spAutoFit`、`normAutofit`。局部显式关闭会阻止继承父级自动适配；选中的 `normAutofit` 缺少比例/行距减少时，使用该元素的 100000/0 默认值，并记录 defaulted 标志。整条链均无选择时才采用关闭。Office 会沿样式层次决定 autofit，见 [MS-OI29500 noAutofit](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/56ab6d15-2c9a-4d77-bc6b-3d8a7d98ebf3)。本配置对选择整体覆盖的解释还需要目标应用边界测试；它没有实际缩小字形、改变行距或扩大形状。

未知文本框/默认项内容、未匹配/歧义占位符输出类型化 `unresolved`；缺少文本体有独立结果。源摘要冲突、非法查询和预算/取消是整个请求的错误。请求顺序和重复对象保留，计算不改写原包或作者声明。

## 主题来源与接口

[主题默认读取器](../../crates/mo-pptx/src/source/theme/defaults.rs)只解析主题根下的 `objectDefaults`。`SourceThemePart.textDefaults` 保留对象默认根、shape/line/text 分类与物理序号；每个分类的 `bodyPr` / `lstStyle` 复用已有平坦文本目录和读取器。`spPr` / `style` 属于其他属性族，其内部的同名文本节点不能注入本层。未知属性/扩展保留位置，原包字节仍由来源层持有。

主题内的文字与主文档、页面/版式/母版共用文本声明预算；主题本身仍消耗已有 XML/主题预算。查询有累计操作步数和词法/来源分配预算，默认最多 256 次对象查询、100 万步和 4 MiB 记账预算；这不是最终 JSON 字节长度或生产峰值内存承诺。

开发入口：

```sh
mo-cli pptx-text-bodies query.json source.pptx
```

请求为 `expectedSourceSha256`、`surface`、`objects` 和上述 `profile`。范围读取 API 为 `resolve_pptx_text_bodies`，WASM 导出同名函数；两端共享 Rust 计算。新增 `pptx-text-body-query` / `pptx-text-body-response`，总计 68 份运行合同，继续由 Rust 生成 Schema 和 TS。它们尚不是公开 MCP 服务或 Musterwork Artifact 接口。

## 验证与体积

[冻结证据](../reviews/evidence/2026-09-25-text-body-verification.json)绑定实际源码、构建、输入、查询、编辑候选和日志。

- 累计 400 项 Rust 测试，本阶段新增 8 项；严格 Clippy、rustfmt、Schema 和 TS 检查通过。
- 99 份自有 PPTX、294 批 Native/WASM 检查，包含 92 份已计算输入、四份明确未决输入和三份错误输入。96 份实际改字、Native/WASM 候选字节相同，重读后的文本框结果保持一致。
- 独立 XML/MCE 校验 1748 个属性值、1748 个来源位置和 92 个 autofit 选择；1920 个未修改压缩部件逐字节相同。
- 在固定官方 Transitional XSD 上检查 396 个输入部件，390 个通过、六个按语料意图失败。保留未知属性及形状子树隔离探针不声称格式有效；部件检查不代替整包或应用认证。
- 旧 9770 批重跑，全部响应内容、编辑候选和像素逐项不变。累计主内核检查 10064 批，不代表能力完成百分比。

macOS arm64 / Apple M4 Max / Node 23.5.0 的未压缩开发产物为 CLI 6146896、Rust WASM 5320763、绘制 worker 5576480 字节，相较上一阶段增加 75248/61154/19872 字节。Rust WASM glue 增加 768 字节；文本 worker 大小未变但重新构建，C++ WASM/glue 和 TS 适配器摘要未变。Cargo/pnpm 锁文件及外部依赖版本不变；这些不是完整内核或 Musterwork 安装包，未新增产品延迟/内存测量。

## 复现与后续

在上一阶段语料、固定 ECMA XSD 缓存和当前 Native/WASM 构建就绪后：

```sh
python3 tools/verification/text-body-fixtures.py
node tools/verification/text-body-parity.mjs
python3 tools/verification/text-body-reference.py
python3 tools/verification/contracts.py --text-body-report .codex-work/text-style/parity.json
```

旧报告及其绑定 JSON 必须在重跑前按上一份证据摘要归档。`text-body-regressions.py` / `text-body-regression-audit.py` 重跑并严格比较；`text-body-evidence.py --seal` 要求全部日志与产物匹配，且拒绝覆盖历史记录。

下一步沿同一来源目录完成段落、字符和主题字体选择，接到已有字体资源、段落塑形/布局和路径绘制管线。段落属性不能直接套用文本框规则：Office 忽略 `defPPr`，见[对应说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/18292376-7677-4792-8422-8024b97ec52f)；部分段落缺省与标准描述不同，见[列表级别说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/9b34280e-538e-4811-8af9-761d34f88f20)。这些是后续实现依据，不是本阶段已经完成的能力。

当前来源页面仍拒绝可见文本；没有新增 Office/WPS 打开、编辑、保存或视觉验收。高级对象/播放、完整 Agent 服务和 Musterwork E0–E3 尚未完成。
