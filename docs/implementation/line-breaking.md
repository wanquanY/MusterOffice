# Unicode 断行机会

2026-09-24 · `mo-unicode::line_break` 已实现固定 Unicode 18.0.0 / UAX #14 revision 57 默认断行机会，Native CLI 与 WASM 共用 Rust 计算。它给出可选与强制边界、三个坐标系和需词典分析的位置；实际行宽选择、目标应用定制、连字符绘制、行边界重塑和页面布局继续由文字排版层实现。

## 固定数据与算法

[数据清单](../../crates/mo-unicode/data/line-break-manifest.json)锁定官方 LineBreak、EastAsianWidth、DerivedGeneralCategory 及 LineBreakTest 的 URL、长度和 SHA-256，沿用仓库已有 Unicode-3.0 许可。运行表 `line-break.bin` 为 31752 字节、3174 条非默认范围，包含 49 种原始分类与必要的辅助属性；Extended_Pictographic 复用已固定的 Unicode 18 表。不会读取浏览器、操作系统或系统字体的 Unicode 版本。

实现依据[固定 UAX #14 revision 57](https://www.unicode.org/reports/tr14/tr14-57.html)，按规则优先级求值。原始标量序列保留；只为上下文判断建立忽略附加符后的计算索引。前方非空格位置、数字后缀状态和 RI 奇偶状态预先计算，避免在长空格、组合符或数字标点链中反复回扫。扫描次数随输入长度线性增长，属性查询使用有界二分表。

换行控制、CRLF、零宽空格、连接符、不换行空格、开闭标点、引号、数字、Hangul、Brahmic 和 Emoji 相关规则均进入同一默认算法；没有按测试样本返回预存答案。非空输入的结束位置为强制边界，空输入没有断行机会，另外返回零结束坐标，供上层决定空段落布局。

版本变化具有实际语义：Unicode 18 将软连字符归入 HH、部分破折号归入 BA；revision 57 同时改变 BA 与 GL 之间的默认处理。版本不能只写进报告、实际仍使用较旧依赖的数据。

## 开发入口与合同

`mo-cli line-break-analyze request.json` 和 WASM `analyze_line_breaks` 接收：

```json
{"texts":["中文排版","a b","ไทย"],"characters":[173,8211]}
```

`line-break-request/response` 是 Rust 生成的开发合同。成功响应含 `unicodeVersion`、每段结果及显式字符属性查询。每段 profile 为 `unicode18.0.0-uax14-r57-default-v1`，包含：

- `end`：scalar、UTF-8、UTF-16 的原始结束坐标。
- `opportunities`：递增的边界和 `allowed` / `mandatory`；不返回禁止位置，也不据可选机会修改作者文字。
- `complexContextScalars`：原始 SA 分类的位置。默认算法将其映射为 AL/CM；调用方不能把默认结果误当作已经完成泰文等语言的词典分词。

字符查询返回原始 Line_Break 分类和规则所需的辅助属性。`eastAsian` 表示 EAW 的 F/W/H 集合，不是字体测得的字宽；`combiningMark` 仅表示 Mn/Mc。无效 Unicode scalar、重复键、孤立 surrogate 和未知字段明确拒绝。

单文本最多 65536 scalars / 262144 UTF-8 bytes；一批最多 1024 个文本、262144 个累计 scalars、65536 个属性查询；JSON 请求沿用 32 MiB 上限。核心检查取消，失败不返回部分结果。批次预算在分析前预检；核心无文件、网络或主机语言查询。浏览器 Worker、硬取消延迟和生产宿主内存峰值仍需独立验收。

## 与后续排版的衔接

默认断行机会不是最终行。默认规则可能在“空格＋组合符”的字簇内部允许断点，不能直接当作可裁剪字形的位置；目标排版 profile 需要显式处理字簇、不可分域、脚本连接、CJK 禁则与作者规则。若改变默认规则，必须记录具体定制和适用范围。

后续以实际字体度量和有效字号选择行尾，必要时重新塑形边界，再执行行特定的 bidi L1/L2、基线与对齐。软连字符的可见结果取决于所选断点与语言；当前没有通过插入普通短横线宣称完成连字符支持。词典分词、Office/WPS 行为、自动断词、紧排/两端对齐和紧急换行都尚未由这个默认入口解决。

## 验证与复现

在[开发说明](development.md)中的 Native/WASM 构建基础上运行：

```sh
python3 tools/unicode/fetch.py --manifest crates/mo-unicode/data/line-break-manifest.json --directory .codex-work/line-break
python3 tools/unicode/generate-line-break.py --check
cargo run -p mo-unicode --example line_break_conformance --release --locked --offline -- .codex-work/line-break/LineBreakTest-18.0.0.txt
cargo run -p mo-unicode --example line_break_properties --release --locked --offline > .codex-work/line-break/native-properties.txt
python3 tools/verification/line-break-properties.py
node tools/verification/line-break-parity.mjs
python3 tools/verification/contracts.py --line-break-report .codex-work/line-break/parity.json
```

[本轮证据](../reviews/evidence/2026-09-24-line-break-verification.json)绑定如下实际结果：

- 官方 19346 条用例全部通过，0 跳过；除了直接 Rust 执行，再分为 19 批通过真实 Native/WASM API，并核对全部边界。
- 独立密集参考逐点比较 1114112 个码点；真实 Rust 属性查询核对 1112064 个有效 scalar；6254 个边界属性探针在两端一致。
- 22 批补充测试覆盖空文本、强制边界、三个坐标系、SA、长空格/附加符/数字/RI 上下文、65536 个可选/结束边界、合法预算上限与错误请求。强制/可选类型另由明确的硬换行字符集和结束位置检查。
- 8 项新增 Rust 测试验证具体文字行为、Unicode 18 变化、预算与每个取消检查点；全仓 160 项 Rust、严格 Clippy/TS、28 份 Schema 通过。新接口增加两份 Schema，原 26 份及既有 1623 批响应保持不变；总计 1664 批跨端对比。

新增功能使当前开发 Rust WASM 从 3134895 字节变为 3184405 字节，增加 49510 字节，其中包含新数据和 API/算法代码；Native CLI 从 3760528 字节变为 3812144 字节。它们不包括完整绘制、媒体、字体包及宿主，不能据此估算完整内核或 Musterwork 安装包。

官方语料并非所有字符串的穷举，属性一致也不代表目标应用排版或视觉验收。该实现推进了文字流水线，E0–E3 及 Musterwork 替换门禁仍未完成。
