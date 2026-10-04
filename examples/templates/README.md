# Presentation template examples

[English project guide](../../README.md) · [中文项目说明](../../README.zh-CN.md)

These are the two original 14-slide templates from Musterwork's
`native-platform-v3` collection, included with the maintainer's permission under
the project's [Apache-2.0 license](../../LICENSE).

| Template | Download | Preview | Contents |
| --- | --- | --- | --- |
| Blueprint · Business Review / 蓝图 · 商务汇报 | [PPTX](business-blueprint/business-blueprint.pptx) | [All 14 slides](business-blueprint/preview.jpg) | 5 charts, 6 tables, 2 illustrations |
| Perspective · Research Report / 视野 · 内容报告 | [PPTX](editorial-perspective/editorial-perspective.pptx) | [All 14 slides](editorial-perspective/preview.jpg) | 3 charts, 7 tables, 2 illustrations |

## Use a template

1. Download the PPTX. On GitHub's file page, use **Download raw file** if needed.
2. Open it in your presentation editor, or attach it to a conversation in
   [Musterwork](https://app.musterwork.com) and ask the agent to adapt it.
3. Replace the example text, numbers and cases with your own material.

The content is Chinese. The files reference **Noto Sans SC** but do not embed or
bundle it. Install the font in your editor, or explicitly choose a replacement.
MusterOffice renderers require font bytes supplied by the integrating application.

Charts preserve their data series and embedded XLSX workbooks. Tables preserve
native cells. The two illustrations in each deck are images; the deck itself is
not a flattened screenshot. All business numbers, organizations and cases are
fictional. AI-generated scenes do not depict identified real businesses or events.

## Preview and provenance

The previews come from the actual MusterOffice rendering pipeline in Musterwork.
Both previews and PPTX files are byte-for-byte copies of the qualified original
artifacts. [manifest.json](manifest.json) records their sizes, SHA-256 digests and
object counts. [artwork-provenance.json](artwork-provenance.json) preserves the
illustration prompts and original image hashes; the image bytes are embedded in
the PPTX files.

These decks demonstrate the supported import/rendering path. They were authored
with `@oai/artifact-tool` 2.8.59 and subsequently imported, rendered and edited
through MusterOffice in Musterwork. They are not claimed to have been authored
entirely by the MusterOffice engine. The private application's code, account data,
template catalog identifiers and runtime storage are not part of this example.

The source qualification records cover macOS/Linux import and rendering, plus
native/WASM parity for the chart pages. They do not certify every PowerPoint/WPS
editing workflow or arbitrary chart editing. Support still depends on the
operation and document content.

## 中文使用提示

两套模板分别为「蓝图 · 商务汇报」和「视野 · 内容报告」，各 14 页，采用项目的
Apache-2.0 许可证。下载 PPTX 后可在演示文稿编辑器中修改，也可上传到
[Musterwork](https://app.musterwork.com) 让 Agent 使用。

请替换虚构业务数据和案例；外部编辑器需安装 Noto Sans SC 字体或指定替代字体。
图表、工作簿和表格单元格仍可编辑，预览图来自 MusterOffice 实际渲染。
示例不代表全部 Office/WPS 互操作或任意图表编辑能力已验收。
