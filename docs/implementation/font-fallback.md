# 片段内混合字体回退与边界重塑

2026-09-24 · `mo-text::fallback` 已接入段落准备的 Native worker / WASM 路径。同一逻辑片段可以使用不同的显式候选字体，例如 `A👩A` 保留两个 Latin 字形和中间的 Emoji。输出仍是字体单位下的字形及逻辑范围；按宽度断行、基线、度量缩放、绘制和完整目标应用验收尚未完成。

## 边界与选择

本层复用原子候选的字体验证、资源范围、参数预检和预算计数；没有引入另一份字体解析器。字体摘要、face、所有候选及未使用样式在首个组件调用前核对，缺字与组件失败分别处理。

固定策略 `grapheme-cluster-probe-and-reshape-ucd18-hb14.5-cmap14-v1` 的步骤为：

1. 在调用者提供的脚本、方向和样式片段内建立 Unicode 18 扩展字簇边界。每个候选先对整个原片段塑形，保留全文逻辑上下文。
2. 按实际 glyph cluster 覆盖和 `unsafeToBreak` 移除不可独立选择字体的边界；收集已探测候选的约束并取保守并集。RTL 字形先按逻辑 cluster 范围解释，不能按输出数组位置切文字。
3. 各剩余区域优先使用候选顺序中没有缺字或未解决变体的字体，相邻同候选区域合并。尚有未解决区域才继续探测下一字体；不会仅凭 nominal cmap 提前淘汰可分解塑形的字体。
4. 选中的部分范围重新调用塑形器，仍提供完整逻辑文本上下文，仅在原片段边缘保留原 BOT/EOT 标记。同一候选的多个范围批量传递，每批最多 256 runs。仅当选择范围就是原片段时，才可直接复用那次完整探测。
5. 如果边界重塑重新产生缺字或变体问题，记录该候选在该范围的拒绝证据，再尝试后续候选。成功的精确范围可复用，失败范围不当作成功结果。没有可用候选时保留明确的 `unresolved` 范围。

这里不裁剪并拼接先前探测得到的字形数组。`unsafeToConcat` 也不等于禁止字体切换；缺少单侧标记不能证明拼接安全，本实现会重新塑形整个选中范围。边界含义依据 [HarfBuzz glyph flags](https://harfbuzz.github.io/harfbuzz-hb-buffer.html#hb-glyph-flags-t) 与 [cluster 说明](https://harfbuzz.github.io/working-with-harfbuzz-clusters.html)。这些规则不是 Unicode 自动断行算法，也不是 Office 的字体选择策略。

## 合同与有效样式

`shape-paragraph` 请求保持不变；响应 profile 升级为 `unicode18-auto-items-hb14.5-mixed-font-reshape-v2`，原来的 `cascade` 字段替换为 `fallback`。这是开发合同的显式变更，尚无稳定发布版本。独立的 `shape-cascade` 原子片段入口及其合同保持不变。

`shapedItemIndices` 将每个 fallback item 映射到原始文字项。每项包含完整探测证据、受保护的字簇边界、重塑拒绝和按逻辑顺序完整覆盖该项的 `fragments`；每个成功片段记录字体、候选和重新塑形的结果。片段顺序不是最终视觉绘制顺序，不同字体的 units-per-em 也不能直接相加。

样式合并使用计算属性：语言标签忽略 ASCII 大小写；独立轴按标签排序；相同 SHA-256 / face 的字体绑定别名视作相同资源。作者输入保持原样，候选优先级和 features 的顺序保留。`en`/`EN` 或等价轴顺序不能意外打断 `office` 的连字。原始重复轴、非法绑定、未使用样式仍须通过完整验证，不会被样式合并隐藏。

## 预算、失败与取消

沿用原子候选的 128 MiB bundle、32 个绑定、256 个文字项、每项 32 候选和累计 1024 shaping runs；增加累计 1024 个输出片段、1024 条重塑拒绝。上下文和 glyph 累计预算继续生效。计数用足够宽的整数检查后收窄，Native/WASM 不因地址宽度产生不同拒绝结果。

结果分别记录 `shapingRuns` 和 `componentCalls`，因为一批组件调用可能包含多个 run。任何参数、组件、限额或取消错误均使整次准备失败，没有可提交的部分结果。Native 隔离工作进程和 WASM 失效实例拒绝复用继续生效；生产宿主硬取消、实例池和全部内存峰值记账仍待完成。

## 实际验证

先完成[开发构建](development.md)与[字体语料准备](font-resources.md)，再运行：

```sh
node tools/verification/itemization-parity.mjs
node tools/verification/font-fallback-parity.mjs
python3 tools/verification/contracts.py --itemization-report .codex-work/itemization/parity.json --itemization-report .codex-work/font-fallback/parity.json
node --expose-gc tools/verification/font-fallback-benchmark.mjs
```

[本轮证据](../reviews/evidence/2026-09-24-font-fallback-verification.json)记录 17 个新增真实 Native/WASM 用例，包括 Latin/Emoji 混排、Latin 连字和字偶距、Arabic 连写、组合字、肤色/家庭 ZWJ、旗帜字簇、未解决变体、样式等价与限额。33 个选中片段的 58 个字形、cluster、标志和位置与同版未经修改的 `hb-shape` 一致。这里只验证这些明确样本，不能推广为全部 Emoji 的组合外观已经合格。

先前 34 个段落用例已按新响应合同重新验证，30 个选中片段、112 个字形与上游一致。197 个纯分段及其他 1375 批旧响应保持不变；合计 1623 批跨端对比。新增 8 项 Rust 测试覆盖保护边界、RTL、重塑后缺字、取消、失败和预算；全仓 152 项 Rust、26 份 Schema、严格 Clippy/TS 检查通过。Schema 中 25 份保持不变，只有段落响应及相应生成类型升级。

真实 C++ WASM 分配故障发生在两次成功探测之后的第三次调用，即首次边界重塑。结果整批失败且实例失效，复用被拒绝，新实例与 Native 正常结果一致。重塑后新缺字导致换候选的分支另有模拟后端测试；这两项证据不能互相替代，也未声称完成所有分配位置的故障扫描。

## 局部性能测量

Apple M4 Max、36 GiB RAM、macOS Darwin 25.2.0、Node 23.5.0；固定 Noto Sans SC（17772300 字节）与 Noto Emoji，输入 `A👩` 重复 32 次。两个路径都执行相同 Rust 校验、分段、候选探测及 64 个最终片段的重塑，66 个 shaping runs 的完整响应逐字节相同。对照路径仅在传输边界将批次拆成单 run，再重组回复。

内存中预热两对，交错测量九对；每次测量前显式 GC，其时间不计入区间。单 run 上传路径中位 83.01 ms，批量路径 74.53 ms，约减少 10.2%。实际组件调用从 66 次降至 4 次，累计字体上传从 651911568 字节降至 39509792 字节。累计上传量不是内存峰值，也不是最终包大小；C++ 仍按每个 run 构造 face/font，尚无跨请求字体结果缓存。

原始样本、字体/产物摘要和统计口径均在证据中。测量排除了文件 I/O、模块构造、冷启动、工作进程调度、断行/几何、绘制和桌面宿主；不能外推完整内核、不同设备或 Musterwork 的性能。

## 后续边界

本层仍保守依赖 cmap-14 变体证据；GSUB-only 变体、Emoji 视觉策略、主题字体/FontManifest、字簇内部作者样式以及目标应用字体规则尚未完整实现。探测依赖取并集可能使区域比必要范围更大，重塑拒绝也暂时覆盖整次尝试范围；它们保留诊断，不通过拆开连字或丢弃字符获得成功。

后续已实现并验证[Unicode 默认断行机会](line-breaking.md)。继续推进目标排版约束、宽度拟合、行边界重塑、字体度量/基线以及实际绘制。当前证据不关闭 E0–E3，也不代表已经能够替换 Musterwork。
