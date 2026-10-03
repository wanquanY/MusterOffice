# 字体合字光标查询

2026-10-03，原生编辑 P1 的字体数据层已实现。公开计算入口为 Rust
`mo_text::carets::query/query_verified`、`mo_kernel_api::font_carets_json`、WASM
`font_carets`、开发 worker `--carets` 和开发 CLI `font-carets`。合同由 Rust 生成。
本项尚未组成页面命中、文字选区或 IME 查询；后续必须使用同一排版的字素、字形簇、
双向顺序、glyph origin、字号、tracking、baseline shift 和页面变换计算最终位置。

## 语义与职责

输入明确绑定字体摘要、face、方向、可变轴和 glyph IDs，不读取系统字体。
可变轴的 16.16→f32 策略、字体验证和 scale 与既有塑形/度量完全相同。
成功值保留字体中的顺序，单位为 1/64 字体设计单位；负值和真实零均有效，空数组表示没有 GDEF 光标。
本层不推断字符数、不等分合字，也不把这些坐标当成页面坐标。
[HarfBuzz 公开接口](https://harfbuzz.github.io/harfbuzz-hb-ot-layout.html#hb-ot-layout-get-ligature-carets)
返回的是塑形前位置；最终光标必须结合实际 glyph placement。

GDEF Format 1 坐标与 Format 3 variation/device 由同一 HarfBuzz 实例解释。
组件不设置 ppem，沿用无 hinting 的设计单位 profile；device 像素修正在该 profile 下不启用。
原生与 WASM 都通过 `mo_hb_caret_font` 运行，没有 JS 排版或另一个字体引擎。

真实 Format 2 样本揭示固定 HarfBuzz 14.5.0 默认 OT font functions 未注册 contour-point callback，
公开查询会因而返回伪零。`mo_hb_caret_points.h` 在独立 sub-font 注册该项回调，其他字体函数委托原实例；
回调复用固定上游 `glyf/gvar` 点计算，包含原始点编号、复合变换、变体、侧边距及方向原点。
不使用 draw callbacks 反推点编号，因为绘制路径会重排或展开 off-curve 点。
上游源码未改写；此私有适配显式依赖固定版本的内部头文件，升级 HarfBuzz 必须重新验证该适配。

Format 2 在无 TrueType `glyf` 的字体、无效点编号或不可计算轮廓上整批失败，
不返回伪零或部分 glyph。坐标在整数转换前检查有限值及 i32 范围，同时检查方向原点相减范围。
CFF 的 Format 1/3 查询不受此限制；未承诺 CFF Format 2 支持。

## 有界执行与所有权

- 每批最多 64 个实例、4096 个 glyph；每实例最多 256 个互不重复的 glyph、64 个轴。
- 每 glyph 最多 64 个字体光标，超过限额整体拒绝；不截断。
- 每实例的 contour-point 计算共享 `HB_BUDGET_GLYPH = 2^24` 工作预算及单 glyph 缓存；
  上游复合深度/图边界与统一 256 MiB 分配上限继续生效。
- Rust 在第一次组件调用前检查所有实例，验证结果的身份、方向、精度、数量和完整长度。
- 字体驻留沿用 `FontSession`，页面预算经 `FrameBackend` 计费；未引入全局字体或结果缓存。
- 分配失败使整个 Native 进程组件/WASM 实例永久失效；后续塑形、轮廓、度量和光标查询共享该状态。
  宿主错误、非法回复及调用后取消也丢弃结果并使组件失效。普通输入/预算拒绝不假装内存损坏。

低层 ABI 见[组件说明](../../components/harfbuzz/README.md)。字体位置保持内核 Q32 之前的精确整数输入，
供后续统一 placement 计算；本层不增加像素舍入或浏览器测量。

## 已执行的验证

[证据清单](../reviews/evidence/2026-10-03-font-ligature-carets.json)绑定源码、原创样本、工具链、
真实产物与日志摘要。范围为本机 macOS arm64 Native、Node 中真实 Rust WASM 与 C++ WASM：

| 检查 | 结果 |
|---|---|
| `mo-text` 测试 | 118 项通过，新增 5 项光标/协议/取消测试；字体驻留测试增加光标操作 |
| Native 集成 | 新光标驻留测试与既有页面驻留/像素回归共 2 项通过 |
| Native/WASM 完整 JSON | 56 批相同，含 TTC/CFF 无光标、输入/资源/预算错误与实际 CLI |
| 独立 FontTools 4.61.1 对照 | 32 组、408 个光标位置完全相同，容差 0；覆盖非等距、负/零、Format 2/3、复合、off-curve、gvar、四方向 |
| ABI 异常 | 两套样本各 18 个非法请求，Native/WASM/ASan+UBSan 同结果 |
| 分配失败 | 横向 GDEF、纵向复合 gvar 各 300 位置；分别 37/46 个 Native 与 WASM 实际失败，失败输出为空且复用拒绝 |
| 原子性/失效 | 批次第二实例失败不发布第一实例结果；跨操作双向失效及新实例恢复通过 |
| 真实 WASM 驻留 | 12 次 shaping/metrics/outlines/carets 结果与未驻留调用一致，仅上传一次字体，释放及陈旧 handle 检查通过 |
| 回归与静态检查 | 28 项 playback-client 通过；严格 Clippy、Rust schema 检查、生成 TS 类型检查通过 |

分配故障覆盖上述两份原创样本，不代表穷尽所有字体。没有真实浏览器文字编辑、产品 IME、
Windows/Linux 执行、性能达标或完整编辑器验收结论；未切换 Musterwork 当前 step 固定开发发行。

## 重现

先按 `components/harfbuzz/lock.json` 将已验证的上游归档放入三个构建目录
`.codex-work/caret/{harfbuzz,fault,asan}`。工具链沿用固定组件说明；Python 使用 FontTools 4.61.1。
Native 常规、Native fault、Native fault+ASan/UBSan 分别在这三个目录构建；
WASM 常规与 fault 分别在前两个目录构建，使用 `tools/components/build-harfbuzz.py`。

```sh
.codex-work/font-tools-venv/bin/python tools/verification/caret-font-fixtures.py
.codex-work/font-tools-venv/bin/python tools/verification/caret-font-reference.py .codex-work/caret/reference.json
MO_HARFBUZZ_LIB_DIR="$PWD/.codex-work/caret/harfbuzz" cargo build --locked --release -p mo-text-worker -p mo-cli
cargo build --locked --release -p mo-wasm --target wasm32-unknown-unknown
.codex-work/toolchain/bin/wasm-bindgen target/wasm32-unknown-unknown/release/mo_wasm.wasm --target nodejs --out-dir .codex-work/caret/wasm-node
pnpm exec tsc --project packages/text-component/tsconfig.json --outDir .codex-work/caret/text-component
node tools/verification/font-carets-parity.mjs .codex-work/caret
node --expose-gc tools/verification/font-carets-safety.mjs .codex-work/caret
node --expose-gc tools/verification/font-carets-safety.mjs .codex-work/caret points
```

Node bindgen 输出目录必须有 `package.json` 声明 `{"type":"commonjs"}`。
如本机 Native CLI 需要 Skia，`MO_SKIA_LIB_DIR` 同样显式指向已验证的匹配 Native 组件。
完整构建参数保存在证据绑定的 `native-build.json` / `wasm-build.json`。
