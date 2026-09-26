# 字体实例度量

2026-09-24 · 文本 E0 进展。新增 `font-metrics` / WASM `measure_font`，从显式字体字节、face 和可变轴取得实例度量。它与塑形共享字体构造、轴量化、设计单位和失效隔离，为后续行高、基线及装饰线计算提供输入。尚未产生行框、页面几何或绘制结果。

## 计算合同

`FontMetricsRequest` 绑定 SHA-256、face 及最多 256 个实例；每个实例包含最多 64 个显式轴和最多 28 个互不重复的度量名。整个请求先经 Rust 验证，后上传一次字体执行批次。摘要冲突、重复/未知/越界轴、重复度量和预算错误不调用组件，不返回部分实例。

28 项覆盖水平/垂直升部、降部、行间隙、水平裁剪边界、插入符斜率/偏移、x-height/cap-height、上下标大小与偏移、删除线和下划线。结果按请求顺序返回，位置使用有符号整数，精度为 1/64 字体设计单位。`positionUnitsPerEm = unitsPerEm * 64`，还不是像素或 EMU。轴输入为精确 16.16，返回实际 binary32 位；默认未列轴保持字体默认值，与塑形相同。

`position: null` 表示组件没有读到该度量，`0` 表示读到了真实零。OS/2 旧版本没有 x-height/cap-height、字体缺少 vhea 等情况不以估算值填充。当前字体检查不是完整字体净化；缺失与被上游判为不可用的表仍不能细分，所有变化表及布局程序合法性也未完整验证。

固定 profile 为 `harfbuzz-14.5.0-ot-metrics-design64-no-synthesis-v1`。使用公开的 `hb_ot_metrics_get_position`，不调用带合成值的 `_with_fallback`，不依赖私有 metric tag。该 profile 有明确的库语义：

- 水平三项在 OS/2 `USE_TYPO_METRICS` 置位时取 typo 字段，否则取 hhea；相应 MVAR tag 的变化继续由固定版本解释。
- 上游对升部/降部作正负号规范化，其他度量保留其接口定义；因此输出不是原始表字段的逐字复制。
- 可变实例包含 fvar 坐标归一化、avar 映射和 MVAR 变化。不能把静态 `mo-font` 元数据直接当作变化后的实例值。

这些是可重现的 HarfBuzz 计算选择，不是已确认的 Office/WPS 行高策略。原始 hhea/OS/2/vhea 仍由 `mo-font` 提供；后续目标应用策略需要明确选择度量来源、处理混合字号/字体和缺失值，并以外部文稿验证。

## 共享实现与隔离

`mo-text::metrics` 负责字体绑定、全批次预检、请求和返回值校验；`mo-kernel-api`、Schema 和 TS 类型同源。原生 worker 新增 `--metrics`，CLI 启动其隔离工作进程；WASM 通过现有 TS 组件适配器的 `measureBatch` 调用独立 C++ 模块。宿主负责字节、权限和进程/模块生命周期，核心不读系统字体或路径。

`components/harfbuzz/mo_hb_font.h` 集中字体对象生命周期、相同轴设置、UPEM×64 缩放和永久失败状态。塑形与测量共用这条实现，避免两条字体构造链路逐渐分歧。任一操作分配失败会使整个实例失效，另一操作也必须拒绝复用；重新建立 Rust 包装器或 `hb_font` 不解除失效。

测量使用独立 ABI magic、6-word 头部和 `[tag, available, position]` 输出。Rust 核对版本、长度、UPEM/scale、度量名及顺序、可用标记、缺失值零位表示和尾部；失败帧不得携带部分结果。整批结果未校验完或取消时不发布。空实例列表仍检查字体身份，且取消不会使未调用的组件失效。详细 ABI 见[组件合同](../../components/harfbuzz/README.md)。

## 验证与范围

[本轮证据](../reviews/evidence/2026-09-24-font-metrics-verification.json)绑定当前源码、运行产物、组件构建和实际结果：

- 29 批 Native/WASM 返回 JSON 逐字节一致，覆盖 TTF/CFF/TTC、五份固定 Noto 字体、原创变化字体、缺少度量、真实零、轴和合同拒绝、空批次以及 256 实例上限。
- 286 个实例的 7980 个值与独立入口、未修改的同版 HarfBuzz 一致，其中 256 个是相同字体的批次上限压力输入；这不是 286 种字体。该对照仍共享 HarfBuzz 算法。
- 另外用 FontTools 4.61.1 读取表、单独计算 avar 和 ItemVariationStore，核对 10 个实例、280 个值，容差为零。这些原创样本的坐标和区域权重为 binary-exact；另一个非精确轴输入由上游对照覆盖。
- 私有 ABI 的 16 组正常/拒绝输入在 Native、WASM、Native ASan/UBSan 一致。300 个分配位置在三端各触发 23 次测量失败，失败无输出并拒绝复用；其余位置等于基线。另验证同一批次先成功一个实例、第二个真实分配失败时全批次丢弃，以及塑形和测量相互拒绝复用。
- 共享构造调整后，旧 1664 批主内核跨端语义结果保持不变；旧组件的 45 个用例与 300 个故障位置重新通过，151 次塑形故障和上游重试的两个错误成功反例仍可复现。新增 6 个 Rust 测试，总计 166；运行合同为 30 份。

故障位置只覆盖当前固定样本，不是穷尽所有字体/变化表。没有本轮端到端性能收益结论。C++ 度量代码增加、链接布局变化及实际文件大小见证据，不能推导完整内核或 Musterwork 安装包体积。

## 可重复步骤

先按[组件构建](harfbuzz-component.md)和[开发说明](development.md)重建常规 Native/WASM、带注入 Native/WASM、ASan/UBSan 组件、Rust CLI/worker/WASM 及 TS 适配器。已有工具链、固定字体和上游归档为前置条件，不选择系统 HarfBuzz。

```sh
.codex-work/font-tools-venv/bin/python tools/verification/metric-font-fixtures.py
.codex-work/font-tools-venv/bin/python tools/verification/font-metrics-reference.py --prepare
clang++ -std=c++17 -O2 -DHB_NO_FEATURES_H -I .codex-work/harfbuzz/release/source-native/harfbuzz-14.5.0/src .codex-work/harfbuzz/release/source-native/harfbuzz-14.5.0/src/harfbuzz.cc tools/verification/harfbuzz-metrics-reference.cpp -o .codex-work/font-metrics/hb-metrics-reference
node tools/verification/font-metrics-parity.mjs
.codex-work/font-tools-venv/bin/python tools/verification/font-metrics-reference.py
node --expose-gc tools/verification/font-metrics-safety.mjs
python3 tools/verification/contracts.py --font-metrics-report .codex-work/font-metrics/parity.json
target/release/mo-cli font-metrics .codex-work/font-metrics/cli.json fixtures/fonts/owned.ttf
```

两份原创 MVAR 字体由 `metric-font-fixtures.py` 从本仓库原创 `owned.ttf` 生成，未使用第三方轮廓；摘要见[样本清单](../../fixtures/fonts/owned-metrics.json)。FontTools 与参考可执行程序仅用于开发验证，不进入运行依赖。

下一步将显式字号、实例度量、字体片段、断行机会与按行重塑结合，生成可核对的基线和行几何。目标字体策略、缺失字体/复杂字体策略、完整字体净化、变体/Emoji、竖排、行边界重塑、分页/溢出/绘制和 Office/WPS 验收仍需完成；本轮不代表替换门禁通过。

依据：[HarfBuzz 度量 API](https://harfbuzz.github.io/harfbuzz-hb-ot-metrics.html)、[OpenType MVAR](https://learn.microsoft.com/en-us/typography/opentype/spec/mvar)、[OS/2 fsSelection](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fsselection)。
