# Unicode 文本边界与固定属性

2026-09-24 · `mo-unicode` 已实现固定 Unicode 18.0.0 / UAX #29 revision 49 的扩展字簇分段，并接入 Native/WASM。它为字体回退、选区和后续文本布局提供统一边界；字体选择、bidi、script itemization、断行和段落排版尚未由本模块完成。

## 为什么先固定这一层

已采用的 HarfBuzz 14.5.0 发行源码包含标记为 Unicode 18.0.0 的 UCD 表。另行核对的 `unicode-segmentation 1.13.3` 仍固定 Unicode 17.0.0；运行该库对照本次官方 853 条测试，实际有 10 条边界不同。新版 GB9c 已改变，不能只替换数据文件便宣称算法版本一致。该候选只用于开发比较，未加入内核依赖。

当前实现直接依据 [UAX #29 rev.49](https://www.unicode.org/reports/tr29/tr29-49.html) 的扩展字簇规则编写前向状态机，使用相同版本的官方属性。没有复制候选库的实现，也不使用 Rust、JS 或操作系统的隐式 Unicode 表。每个 scalar 做一次有界查表和状态推进，不向前重复扫描长组合序列。

## 数据与责任

[固定清单](../../crates/mo-unicode/data/manifest.json)记录官方 GCB、InCB、Extended_Pictographic、Default_Ignorable_Code_Point、Variation_Selector 数据及完整字簇测试的来源、长度和 SHA-256。源码保留 [Unicode-3.0 许可](../../crates/mo-unicode/data/LICENSE-UNICODE)。原始数据在忽略缓存；运行时只带确定性生成的 `properties.bin`：1649 个不重叠范围，16502 字节。该大小不是完整文本内核或字体包大小。

表包含 12 字节头部，以及每条 10 字节的 little-endian start/end/flags；二分查找返回类型化属性。原始输入和生成结果都可重新校验。独立验证将文本数据展开为 1114112 个码点的稠密参考，逐点与生成表比较；Native/WASM 又实际查询了 4838 个范围起止、相邻和中间探针。surrogate 区域只参与数据表比较，不作为合法文本 scalar 输入。

字体检查的 variation selector 判定现已调用同一份属性表；Rust 测试逐个合法 scalar 核对其与既有明确字符范围一致，避免无意改变已验收的 cmap 变体语义。Default Ignorable 只是字符属性，不能据此丢弃作者内容、忽略不支持的 UVS，或宣称 Emoji 序列已被正确绘制。

## 计算接口

Rust：`mo_unicode::segment(text, limits, cancellation)` 和 `properties(char)`。`mo_kernel_api::analyze_text` 提供有总预算的批次。Native 开发 CLI：`mo-cli text-analyze request.json`；WASM：`analyze_text(requestJson)`。

请求示例：

```json
{"texts":["A😀","á","👩🏽‍💻"],"characters":[65039,8205]}
```

响应包含 profile、字簇边界的 scalar/UTF-8 byte/UTF-16 code unit 三套明确坐标，以及 default-ignorable/variation-selector 的全文 scalar 下标。空文本返回一个零边界哨兵；非空文本包含起点和终点。字符属性查询接受有效 Unicode scalar，拒绝 surrogate 和超范围整数。没有隐式 Unicode 规范化，也不转换作者换行。

默认每文本最多 65536 scalars / 262144 UTF-8 bytes；批次最多 256 文本、累计 262144 scalars 和 65536 属性查询。逐 scalar 与查询执行取消检查；失败不返回此前文本的部分结果。同步 WASM 入口仍须由宿主 Worker 提供外部硬取消，不能靠同一线程的消息中断循环。

字簇边界不是断行机会，也不自动成为可切换字体的边界。后续字体选择仍需结合 script/bidi、cmap/UVS、实际塑形 cluster、unsafe flags 和上下文重塑，避免拆开跨字簇 ligature 或复杂文字连接。此接口不应被 UI 或宿主当成简化字体回退器。

## 验证与再生成

```sh
python3 tools/unicode/fetch.py
python3 tools/unicode/generate.py --check
python3 tools/unicode/fixtures.py
node tools/verification/unicode-parity.mjs
python3 tools/verification/contracts.py --unicode-report .codex-work/unicode/verification/parity.json
```

前三步核对固定来源、重建数据与独立参考；跨端脚本需先构建 Native CLI 和 Rust WASM，见[开发说明](development.md)。更新固定 Unicode 版本必须同时更新数据、规则、profile、语料和版本声明，不能让运行时跟随 `latest`。

[本轮证据](../reviews/evidence/2026-09-24-unicode-text-verification.json)绑定全部 853 条官方测试（0 跳过）、4838 个属性探针，以及 65536 scalar 长组合/Indic 链、RI 配对、空内容、坐标、预算与错误请求。它们被组织成 21 组 Native/WASM 调用，不把批次数和标准用例数混加。既有 446 组跨端用例在新主内核上回归，总计 467 组；Rust 共 121 项测试，运行 Schema/TS 共 18 份。没有据此关闭字体选择、段落排版或 E0/E1/E2/E3。
