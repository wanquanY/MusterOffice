# PPTX 来源读取与保留式文本编辑

2026-09-24 · P03 的实际基础实现；不是完整导入、布局或兼容验收。

本页保留 P03 格式工具的阶段边界。2026-09-27 新增的版本化导入、统一事务与直接编译见[统一来源文档](source-backed-document.md)；以下“尚未完成”描述不替代新实现记录。

## 当前路径

`mo-pptx::source` 从不可变 OPC 包建立原生来源索引。它先验证唯一的 presentation 入口，再按 `sldIdLst` 和关系读取页面、母版及版式。对象使用部件作用域内的原生数字 ID，不能把它当作文档全局 ID；名字只作显示，不能作修改凭据。索引记录源包/部件摘要、页顺序、显式尺寸、对象树、局部变换、普通文本 run、换行和动态字段，并返回未解析能力提示。

文本绑定由同一次命名空间 XML 扫描建立，定位普通 run 中的 `a:t`。用户请求只包含源包摘要、部件/对象/段落/run 目标、旧文本和新文本。内核必须重新读取真实来源建立绑定，不能接受调用方伪造的 XML 偏移、对象名称或旧索引作为修改依据。源包摘要、旧文本或目标变化即产生冲突。

修改在内存候选中进行：全部请求通过前置条件后，结构化 XML 覆盖只更换目标叶文本，保持其他原始字节；OPC 重写复用其他部件的压缩 payload。没有实际修改时返回与输入逐字节相同的文件。候选必须重新打开真实 OPC 字节，重新建立来源索引，并与预期的已解析投影比较后才返回。CLI 再对暂存文件执行包级重读并无覆盖发布。宿主授权、事务持久化、CAS、崩溃恢复仍未完成。

## 实现边界

| 内容 | 当前行为 |
| --- | --- |
| Transitional 原生文件 | 按关系和内容类型识别文稿/放映/模板及相应宏容器；不按文件名猜测页面 |
| 页/母版/版式 | 有界遍历关系，校验根类型、ID、部件类型；记录对象层级与显式变换；建立占位符匹配、逐项位置/尺寸来源和主题部件栈 |
| 主题 | 共享目录读取颜色/字体声明与格式条目来源，按颜色/字体/格式分别应用版式和页面覆盖；保留角色颜色映射及有效来源 |
| 文本 | 普通形状文本 run 可查询/替换；动态字段缓存可查询但不提供普通文本修改绑定；换行与段落顺序独立记录 |
| 源字节 | 保留同部件未知属性、节点、注释、编码；未改动包与其他条目保留；UTF-16 已有实际用例 |
| 兼容处理 | 主部件/页面/母版/版式/主题使用固定 MCE profile，执行 Choice/Fallback、Ignorable、ProcessContent、MustUnderstand 和扩展区暂停处理；逻辑选支不改写源字节 |
| 编辑依赖 | 分支外普通叶文本可修改；分支内文本、带物理子节点的文本和动态字段有明确约束；原文件任意分支中发现 timing 时，暂阻止该部件文本修改，等待引用 retarget |
| 签名 | 可索引/无修改复制；实际修改被拒绝，等待移除/重签策略；不是签名有效性验证 |
| 宏部件 | 不执行，保持原内容类型、关系与字节；本轮只用合成 opaque bytes 验证保留，尚未验证真实宏文稿语义 |
| 错误 | 来源冲突、保留冲突、未实现映射、预算、取消和读取失败分别报告；嵌套 XML/OPC 错误保留类别 |

当前索引是原生格式的部分投影，已增加占位符声明、父匹配和位置/尺寸继承，但尚无完整样式继承、字体塑形、世界坐标、时间求值或完整高级对象语义。`graphicFrame` 仅作为原生对象识别，内部表格/图表等不能算已实现；未解析的 shape-tree 节点保留在源包并返回提示。Strict、完整 MCE、表格/图表/SmartArt/公式、动画文本 retarget 等必须继续实现，临时拒绝不是一期最终边界。逐项来源及主题部件栈的边界见[来源继承](source-inheritance.md)。

这组底层编辑操作保持文本 run 结构，尚不是最终 Document/SourceBinding 全局身份、跨 run 编辑、撤销重做及 OperationService。`editable` 和 `editConstraint` 说明当前修改条件；它们不承诺目标应用字体、视觉、动画或未知扩展的语义等价。兼容处理的算法、固定 profile、物理定位及剩余限制见 [MCE 实现](mce.md)。

## 接口与复现

新增 Rust → Schema → TS 合同：`pptx-source-response.schema.json`、`pptx-text-edits.schema.json`。Native CLI 为 `pptx-inspect` / `pptx-edit-text`，WASM 为 `inspect_pptx(bytes)` / `edit_pptx_text(requestJson, bytes)`。索引响应中显式携带部分解析提示，不能冒充完整导入成功。

```sh
target/release/mo-cli pptx-inspect .codex-work/pptx-output/native.pptx
target/release/mo-cli pptx-edit-text edits.json .codex-work/pptx-output/native.pptx .codex-work/pptx-output/edited.pptx
```

`edits.json` 从读取结果选取明确对象和旧文本，不使用机器路径作为内核资源：

```json
{
  "expectedSourceSha256": "<从读取结果取得的64位SHA-256>",
  "edits": [{
    "target": {"part": "/ppt/slides/slide1.xml", "objectId": 6, "paragraph": 0, "run": 0},
    "expectedText": "<读取到的原始文本>",
    "replacement": "修改后的文字"
  }]
}
```

上例含占位值，不能直接执行。对象 ID、段落/run 序号只在绑定的源包版本内有效，不能硬编码为所有文件的标题位置。Native/WASM JSON 请求上限 32 MiB，开发内联二进制桥接上限 128 MiB；核心也支持宿主提供范围读取器，生产资源句柄与 Worker 服务待接入。

自有语料和独立检查：

```sh
python3 tools/verification/pptx-source-fixtures.py .codex-work/pptx-output/native.pptx .codex-work/pptx-source-fixtures
node tools/verification/pptx-source-parity.mjs target/release/mo-cli .codex-work/wasm-node/mo_wasm.js .codex-work/pptx-source-fixtures/manifest.json > .codex-work/pptx-source-parity.json
python3 tools/verification/pptx-source-independent.py .codex-work/pptx-source-parity.json
python3 tools/verification/mce_reference.py .codex-work/pptx-source-parity.json
python3 tools/verification/contracts.py --source-report .codex-work/pptx-source-parity.json
```

可在 parity 命令末尾提供本轮由 LibreOffice 重存的自有测试文件，用于复现该固定样本的 17 对象、转场 fallback 选择、分支外标题修改和无修改复制；不是任意用户文稿接口。未提供时不计该外部文件的 3 个用例。

## 本轮证据

颜色映射阶段全仓 82 项 Rust 测试通过（当前计数见[实现进度](progress.md)）；168 组来源读取/编辑 Native/WASM 对比包含 73 个读取结果、95 个编辑请求。独立 lxml 树处理器对 42 个成功输入核对兼容分支、原始节点序号、对象及文本；独立 ZIP/XML 检查 82 份实际输出，核对 1706 个未修改压缩条目和同部件非目标字节。包含 UTF-16、选支/忽略/包装节点、分支内部拒绝与外部修改、隐藏动画引用和无修改复制。12 个继承专项输入另有独立关系/主题栈与 346 项几何属性比较。

此前 LibreOffice 重存文件有 17 个对象（原 15 个，新增两个占位对象），并把 `rule:layout` 从版式移入母版；独立 lxml 清点确认。MCE 实现现在能够在保留全部转场分支字节的同时修改分支外标题。修改后的该文件再次由 LibreOffice 26.2.0.3 打开并导出两页 PDF，提取到“外部文件原生修改”；这是外部打开证据，不是内核渲染或视觉等价证明。较早记录的 60 处局部变换差异及新增结构差异仍未解决。

主题声明、分项覆盖及独立验证见[来源主题](source-themes.md)，角色映射和外部纯色采样见[颜色映射](source-color-mapping.md)。当前源码、Native/WASM、读取响应及实际输出摘要见[颜色映射验证证据](../reviews/evidence/2026-09-24-pptx-color-map-verification.json)；[主题证据](../reviews/evidence/2026-09-24-pptx-theme-verification.json)保留此前 79 项测试的历史状态，[继承证据](../reviews/evidence/2026-09-24-pptx-inheritance-verification.json)保留此前 75 项测试的历史状态。较早的 [MCE 验证证据](../reviews/evidence/2026-09-24-pptx-mce-verification.json)及[来源编辑证据](../reviews/evidence/2026-09-24-pptx-source-verification.json)保留当时 70/61 项测试的历史状态。结构依据参照 [ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/) 及 [Microsoft 的 PresentationML 结构说明](https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document)。
