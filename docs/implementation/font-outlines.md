# 字体实例轮廓

已实现固定字体实例的批量单色轮廓提取，并连接 Rust、隔离 Native 进程、独立 C++ WASM 和 TS 薄层。这是实际绘制所需的几何输入；页面渲染、彩色字体、hinting、合成与目标应用视觉验收继续实现。

入口为 `mo_text::outlines::extract/extract_verified`、`outline_font_json`、CLI `font-outlines`、worker `--outlines` 和 WASM `outline_font`。请求显式提供字体字节、摘要、face、变化轴、glyph ID 和工作预算，不读取路径、网络或系统字体。CLI 才负责路径与进程生命周期。运行合同见 `contracts/generated/font-outlines-{request,response}.schema.json`。

## 同一字体实例

轮廓与塑形/度量复用同一个字体构造路径：固定 HarfBuzz 14.5.0、OpenType font funcs、face、有效 binary32 变化轴以及 `upem * 64` 比例。请求中的轴仍是精确 16.16 整数，结果报告实际 binary32 坐标。字体身份、全部实例和 glyph 范围在第一次组件调用前验证，不能边调用边发现后面的非法实例。

一次 C ABI 调用处理一个实例的多个字形，整个 Rust → TS/Native backend 批次只上传一次字体。没有逐字形跨语言调用，也没有跨请求隐式缓存。重复 glyph ID 保持原次序，不在这一层修改调用者的索引对应。

新接口 `mo_hb_outline_font` 使用有返回状态的 `hb_font_draw_glyph_or_fail`，而非忽略失败的旧 void 入口。二次曲线和三次曲线保持原类型，不先转折线。输出 profile 为 `harfbuzz-14.5.0-monochrome-unhinted-design64-nearest-away-v1`：

- 坐标以字体原点为基准，Y 轴向上，单位为 1/64 设计单位；后续放置需使用实际字形原点、字号和坐标变换。
- HarfBuzz draw callback 给出浮点数，组件检查有限值及 int32 范围后，按最近整数、正负半值远离零舍入。它不是无限精度的原始轮廓；后续几何与视觉评估需保留该 profile。
- `path: []` 是有效空轮廓，例如 space；`path: null` 表示本次无法取得单色轮廓。失败解释器可能已经输出前缀，组件会丢弃该字形的所有前缀，不把它当作完整轮廓。
- `null` 本身不能区分无该表示、未支持或损坏程序；不得将请求的 `outlined` 状态解释为所有字形均可绘制。调用方必须检查每个字形。
- 返回的 `colorTables` 只是字体级 COLR/CPAL/SVG/CBDT/CBLC/sbix/EBDT/EBLC 表存在信息，不证明某个字形使用它，也不构成彩色绘制支持。取得单色轮廓也不能据此丢弃彩色表示。

Rust 解码器逐项校验完整批次、字体比例、glyph 身份、可用性、指令预算和保留字段，强制每个轮廓以 move 开始、close 结束。非法命令、不闭合路径、额外数据和截断结果导致整批失败及实例失效。当前不提供 bounds，控制点包围框不能冒充曲线极值或实际 ink bounds。

## 预算与失败

最多 64 个实例，每个最多 64 条轴、256 个 glyph，整个请求最多 4096 个 glyph。所有实例的 `maxCommands` 总和不超过 262144；每个实例的 `maxOperations` 为 1–1048576。

`maxCommands` 限制所有 callback 尝试生成的指令，包含随后被丢弃的失败前缀。结果采用有上限的增长缓冲区，只在整次调用成功后移交所有权。输出预算耗尽返回 status 4，公共合同映射为 `LIMIT_EXCEEDED`，不发布已成功的前面实例。

`maxOperations` 是 HarfBuzz 的加权轮廓解释预算，不是 CPU 指令数、时间或所有字体加载工作的总量。每个实例内的字形共享同一个剩余值；glyf、gvar、CFF/CFF2 和 VARC 的上游路径使用该 draw-session 预算。结构净化等初始化仍受上游自身限制、组件内存上限和宿主时间/进程边界管理，不能宣称这一计数覆盖整个任务。当前同步 C ABI 不能在回调外中途检查 Rust 取消信号；宿主仍负责终止工作进程/模块。

组件自身继续使用 256 MiB 跟踪分配上限；WASM 线性内存上限为 512 MiB，Native worker 使用现有超时回收。分配失败、trap、传输异常或无效回复会永久隔离整个库进程/WASM 实例；重新创建 wrapper 不能复活它。塑形、度量与轮廓共享这一失效状态。

Native/WASM 构建显式加入 `-ffp-contract=off`，避免轮廓浮点运算的隐式乘加融合差异。旧塑形、度量和布局结果已在新构建上重验；这不等于所有硬件/字体的普遍数值证明，也没有完成性能门禁。

## 真实验证

[证据记录](../reviews/evidence/2026-09-24-font-outlines-verification.json)绑定本轮源码、合同、产物和输入摘要：

- 34 批 Native/WASM 完整响应一致，覆盖 TrueType、CFF、TTC face、空轮廓、缺少轮廓表示、重复字形、轴、最大实例/字形总量、输入与解释预算、CLI/worker 实际链路。
- 独立上游入口逐条比较 101 个实例、4277 个字形和 21990 条命令。该入口仍使用同版 HarfBuzz，不作为独立解释器证明。
- FontTools 4.61.1 的独立 glyf/gvar/CFF 解释器验证原创曲线字体等 32 个字形、222 条命令和 420 个坐标，量化结果完全一致。该参考没有覆盖任意恶意字体、CFF2 或 VARC。
- FontTools 4.61.1 的 `Glyph.draw` 在 composite 分支不应用传入 offset，参考程序依据它自身计算的 `lsb-xMin` 对根 composite 补足字体原点变换，子 component 不重复平移；这不使用被测输出求偏移。该差异与适用范围保存在参考记录中。
- 20 个组件 ABI 用例在 Native、WASM、ASan/UBSan 中一致；300 个故障位置各触发 34 次实际分配失败，均无部分输出且永久拒绝复用。真实第二实例失败验证跨操作隔离与新实例恢复。
- 原有塑形组件 45 个用例/300 个故障位置、度量组件 16 个用例/300 个故障位置也在新组件上重验。旧 1799 批主内核结果与 36 份既有 Schema 不变。
- 当前共 195 项 Rust 测试、1833 批 Native/WASM 主内核对比、38 份 Schema。独立组件故障位置不重复计入主内核批数。

```sh
python3 tools/verification/outline-font-fixtures.py
node tools/verification/font-outlines-parity.mjs
python3 tools/verification/font-outlines-reference.py
node --expose-gc tools/verification/font-outlines-safety.mjs
python3 tools/verification/contracts.py --font-outlines-report .codex-work/font-outlines/parity.json
target/release/mo-cli font-outlines .codex-work/font-outlines/owned-quadratic-variable.request.json fixtures/fonts/owned-outlines.ttf
```

FontTools 参考环境沿用固定 4.61.1；独立 HB 参考由 `tools/verification/harfbuzz-outlines-reference.cpp` 和固定上游源码单独编译，不链接本组件。原创 `owned-outlines.ttf/.otf` 仅作测试，不随运行组件隐式打包；无新增第三方依赖版本。

下一步把这些轮廓与实际布局原点连接到通用路径几何、边界计算和绘制，再推进页面合成及视觉比较。完整颜色/字体策略、高级可编辑对象、播放、Office/WPS 往返与 Musterwork 的 E0–E3 替换门禁仍未完成。

接口依据：[HarfBuzz draw callbacks](https://harfbuzz.github.io/harfbuzz-hb-draw.html)、[HarfBuzz font draw 状态](https://harfbuzz.github.io/harfbuzz-hb-font.html)。
