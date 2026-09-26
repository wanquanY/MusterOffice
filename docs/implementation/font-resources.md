# 字体资源检查与字符覆盖

2026-09-24 · E0 字体/文本基础。`mo-font` 已能处理宿主显式提供的真实字体字节，Native/WASM 共用实现；它尚不是字体选择器、塑形器、排版器或字体嵌入实现。

## 已实现边界

| 输入/计算 | 行为 |
| --- | --- |
| 资源身份 | 请求必须携带 SHA-256、face index；按块核对真实字节，不按家族名假定字体相同 |
| 容器 | SFNT TrueType、OTTO/CFF 和 TTC 1/2；检查所选 face 的目录、表范围/重叠、表 checksum；独立 SFNT 再查全文件 checksum adjustment |
| TTC | 检查集合目录、各 face 目录范围及 TTC 2 DSIG 描述符范围，允许 face 共享表；只检查所选 face 的表与语义，不宣称整集合已验证或签名可信 |
| 名称 | 保留记录顺序和 platform/encoding/language/name ID；严格解码 UTF-16BE 和 MacRoman；未知编码返回未解析文本；format 1 的 language tag 独立保留 |
| 度量 | 原始 UPEM、字形数量、16.16 字体修订号、hhea/vhea 以及 OS/2 度量、weight/width/fsSelection/fsType 声明 |
| 可变字体 | 原始 fvar 轴顺序、tag、名称 ID、最小/默认/最大值、实例坐标；16.16 用精确整数传输，检查重复轴、范围和实例大小 |
| 字符覆盖 | 显式 Unicode cmap 选择；支持格式 0/4/6/10/12/13 与补充格式 14，校验被消费映射的长度、排序、范围和字形引用后查询 |
| 变体 | 分别返回 mapped、missing、unsupportedVariation、noUnicodeCmap；未声明的变体不退回基础字形；覆盖不是字簇塑形成功的证明 |
| 预算/取消 | 字节、face、table、name、名称字节、axis、instance、query 预算；摘要、checksum、记录与映射循环检查取消；JSON 入口另受请求大小限制 |

Unicode cmap 选择优先级固定为 `(3,10), (0,6), (0,4), (3,1), (0,3), (0,2), (0,1), (0,0)`，返回实际记录索引与格式。Symbol/Macintosh 编码不作为 Unicode 隐式替补。已选记录不支持的格式明确报错，不通过跳过损坏/未实现记录伪造缺字结果。未选中的 cmap 不做全表语义验收。

所选 cmap 的结构校验在查询前执行，避免底层解析器把异常映射作为普通 `None` 返回。恶意共享 variation 数据还受工作量预算限制。超大偏移/长度先按实际字节范围判断，避免 32 位 WASM 与 64 位 Native 的溢出路径分叉。字体表非重叠检查先于 checksum，避免反复校验重叠大表。

`fsType` 是文件内声明，不能替代字体授权；当前不作“可合法嵌入/分发”的自动决定。`fvar` 数据不包含 avar 归一化、gvar/HVAR/MVAR 度量求值或实例化结果。检查成功不代表 glyph、GSUB/GPOS、CFF 程序、COLR/SVG 内容或整个字体已通过安全净化，也不表示字形可绘制。

## 计算入口

Rust：`mo_font::inspect(request, bytes, limits, cancellation)`；Native 开发 CLI：

```sh
target/release/mo-cli font-inspect request.json font.ttf
```

请求示意（摘要必须替换为输入文件的真实摘要）：

```json
{
  "expectedSha256": "0000000000000000000000000000000000000000000000000000000000000000",
  "faceIndex": 0,
  "characters": [
    { "codepoint": 65, "variationSelector": null },
    { "codepoint": 65, "variationSelector": 65024 }
  ]
}
```

WASM：`inspect_font(requestJson, Uint8Array)`。成功返回 `status: inspected` 与字体记录；错误区分输入无效、字体无效、未实现、摘要冲突、预算和取消。JSON 拒绝重复键/未知字段；二进制不经 base64 传输。相关 Schema/TS 来自同一 Rust 类型。

默认字体字节上限 128 MiB、face 256、所选 face 的 table 256、名称记录与 language tag 总计 4096、原始/解码名称字节各 1 MiB、axis 64、instance 4096、字符查询 100000。WASM 同步开发入口没有中途宿主消息取消；原生调用可传取消回调，生产 Worker 隔离、资源句柄与硬取消仍待落实。

## 实际验证

[本轮证据](../reviews/evidence/2026-09-24-font-resources-verification.json)绑定 39 个 Native/WASM 字节一致用例，包含 14 个成功 face 输入和 25 个错误/预算/请求用例。独立 FontTools 4.61.1 比较目录、名称、度量、轴、实例和字符映射；格式 10 因该版本 FontTools 不支持，使用独立 Python 标准二进制观察器核对。共 37805 项基础字符/变体查询，涵盖所用真实字体已声明的 Unicode 字符和 variation 序列以及缺字/不支持序列探针。

五个第三方字体来自固定 Google Fonts commit：Noto Sans、Noto Sans Arabic、Noto Sans Devanagari、Noto Sans SC、Noto Emoji；最后一个为单色字体。原始文件、许可证、SHA-256 和 Git blob ID 由[固定清单](../../fixtures/fonts/upstream.json)核对。字体文件仅在忽略的开发缓存，不进入内核包或桌面包体积。该语料不能代表所有语言、字体或颜色 Emoji。

仓库另有原创合成的 TTF/CFF/TTC 微型字体，由[生成器](../../tools/verification/owned-font-fixtures.py)创建。它们用于格式、异常和引用测试，不代表视觉质量；fvar 合成实例只测试元数据，没有构造可用的轮廓变形。Rust 本轮新增 14 项测试，包括全部 1920 个 TTF 前缀截断、所选 cmap 损坏、跨 face 目录重叠、各预算/取消、严格名称解码与 format 1 语言标签。

复现（FontTools 仅为开发依赖，不进入核心）：

```sh
python3 tools/verification/fetch-font-corpus.py
python3 tools/verification/font-fixtures.py .codex-work/font-fixtures
node tools/verification/font-parity.mjs target/release/mo-cli .codex-work/wasm-node/mo_wasm.js .codex-work/font-fixtures/manifest.json > .codex-work/font-parity.json
python3 tools/verification/font-independent.py .codex-work/font-parity.json
python3 tools/verification/contracts.py --font-report .codex-work/font-parity.json
```

Python 环境需安装 `fonttools==4.61.1`；本轮使用仓库忽略目录内的独立环境，并核对下载 wheel 与官方 PyPI SHA-256。大字体通过 GitHub 的固定 blob API 获取，证书验证保持开启；离线重复验证用 `--verify-only`。

## 后端选型发现与后续闭环

本轮核对 HarfRust 0.13.3 的发行源码：内部 buffer 有 `successful` 状态与工作限制，而公开 `Shaper::shape` 返回 `GlyphBuffer`，后者没有公开该状态。这个接口缺口目前无法证明“返回的字形序列一定完整”；本轮没有声称已经复现特定错误文稿，也没有把库的公开返回当成完整成功。版本、源码摘要和位置保存在证据中。HarfRust 尚未加入实现依赖，不能把该候选当成生产塑形选型已经通过。

后续已经落地并单独验证[HarfBuzz Native/WASM 组件](harfbuzz-component.md)，包括上游分配失败后的错误成功复现及实例隔离约束。字体资源与组件已通过[类型化 Rust 文本层](text-shaping.md)连接。下一步完成显式 FontManifest/主题别名、按 script/language/方向分段；随后落实字簇与行断点、双向文字、字体缺失诊断、轮廓/彩色字形、字号/度量、布局和导出中的字体闭环。完整字体嵌入与可编辑性、目标 Office/WPS、资源成本和 E0 均保持未完成。

标准依据：[OpenType 文件结构](https://learn.microsoft.com/en-us/typography/opentype/spec/otff)、[name](https://learn.microsoft.com/en-us/typography/opentype/spec/name)、[cmap](https://learn.microsoft.com/en-us/typography/opentype/spec/cmap)、[fvar](https://learn.microsoft.com/en-us/typography/opentype/spec/fvar)、[OS/2](https://learn.microsoft.com/en-us/typography/opentype/spec/os2)、[Unicode 17.0 属性](https://www.unicode.org/Public/17.0.0/ucd/PropList.txt)。字段事实和边界实现独立编写，未复制第三方办公内核实现。
