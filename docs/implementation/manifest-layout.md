# 字体清单复用与实际文字路径

2026-09-25。[原生字体绑定](native-fonts.md)之后，显式清单现已连接自动换行、字体度量和字形路径计算。`PreparedManifest` 验证一次资源与清单后，可连续计算多个段落；已有段落、行、混合字体和路径入口共用同一资源加载实现。

## 生命周期与计算边界

内部 `FontResources` 持有不可变字体字节的借用、已核实的 face、原始资源描述及绑定表。它通过 `Arc` 共享验证对象，不能从 JSON 元数据反序列化构造。请求中的摘要、face、范围和顺序必须与句柄绑定完全一致；当前操作的资源数量/字节预算和取消检查仍然有效。资源不能比其借用字节活得更久，宿主也不能在借用期间修改这些字节。

字体元数据和 cmap 不随段落复制或重新校验。每段文字仍独立持有字符、边界、塑形上下文、候选搜索和工作预算；前一段的已用预算、文本或取消状态不会流入下一段。相同范围/face 的别名只验证一次，不允许通过别名改变预期摘要。当前仍按不同 face 分别验证；没有声称跨文件位置或 TTC 多 face 的全部哈希工作均已合并。

`PreparedManifest::load` 在首次组件调用前验证清单的资源、原始名称记录及全部已填实例，包括未使用项。它借用不可变清单，同时共享已验证字体。三个库方法分别提供 `shape_paragraph`、`layout_paragraph` 和 `paragraph_paths`；输入使用借用视图，不要求为每段复制清单或字体字节。

每次操作继续验证自己的语言、特性、字体槽、几何样式、宽度、行距、预算和取消。路径方法直接进入现有布局/度量/轮廓流程，不先塑形一遍只为校验清单，再重复塑形生成路径。已选行沿用候选搜索的字形结果。`FlowWork.verifiedFaces` 保持旧输出含义，表示该操作可用的已验证 face 数，包括未选中的 face；不能把它累计成实际重复读取或哈希次数。

组件生命周期由调用者单独持有。组件失败使该次操作失败；已验证的字体字节保持可用，调用者可以替换失效组件后继续。资源句柄不证明组件健康、字体嵌入权利、替代字体的视觉等价，也不授权任何文件/网络访问。

这些是段落级计算入口。跨段落页面预检、完整页面工作预算和一次性提交仍须由来源页面编译器完成；目前没有新增持久缓存、运行时 Agent 操作或生产组件池。

## 代码与验证

- [`resources.rs`](../../crates/mo-text/src/resources.rs)：唯一资源校验与共享句柄；现有 Context 改为持有共享资源。
- [`manifest/binding.rs`](../../crates/mo-text/src/manifest/binding.rs)：名称/轴及每段样式绑定。
- [`manifest/prepared.rs`](../../crates/mo-text/src/manifest/prepared.rs)：可复用清单、借用输入和三个计算入口。
- [`manifest_paths`](../../crates/mo-harfbuzz-sys/examples/manifest_paths.rs)：独立开发进程调用真实 Native HarfBuzz，比较旧入口与一次准备、多次使用的清单入口。

新增六项 Rust 测试覆盖资源对象实际共享、独立段落状态、过期描述、资源别名、重新应用预算/取消、连续塑形/布局/路径、操作样式预检，以及各取消检查点和后置轮廓失败。全仓 434 项 Rust 测试、严格 Clippy、格式检查、mo-text 的 WASM 目标编译及 68 份 Schema 生成检查通过。现有 Schema 与 TS 文件不变；Cargo 锁文件只增加开发示例对两个已有本地包的依赖，没有新增外部依赖版本。

真实 Native 探针复用一份已验证的原创合成字体，连续处理 20 个请求。与摘要核实后的旧 CLI 对照，18 个得到相同计算结果，2 个得到相同的多段落输入诊断；其中 14 个返回场景结构（包含空段落），其他请求保留控制内容的未决诊断。新旧路径、布局和所有诊断逐项一致。独立 Fraction 参考重算 42 个字形的 84 个原点坐标；合成字体均为线段轮廓，另直接核对 22 个路径和 36 个局部/整体边界。该语料不证明真实办公字体的视觉质量。

当前 Native 工作进程另重新执行六组冻结语料：34 个段落塑形、17 个混合字体、34 个行重塑、35 个行几何、37 个自动布局、50 个字形路径，共 207 个请求，响应逐字节不变。这包含既有真实多语字体、轴、布局与错误输入；没有重放那些报告中另行记录的组件故障注入场景。本轮新增取消/失败测试使用可控后端，真实 Native 探针使用正常组件，二者分别记录。

上述为当前 Native 执行与 WASM 目标编译证据，**不是新增 Native/WASM 运行对比**。此前 10064 批仍只认证冻结的[文本框运行时产物](../reviews/evidence/2026-09-25-text-body-verification.json)。本轮未重建那 11 份发布产物，也未测完整安装包、产品延迟/RSS；跨段落消除重复资源校验是已验证的实现事实，尚无产品级速度倍数结论。

## 复现

先按[开发说明](development.md)准备固定 Native HarfBuzz 组件。在仓库根目录运行：

```sh
mkdir -p .codex-work/manifest-layout/previous
cargo run -p mo-harfbuzz-sys --example manifest_paths --locked > .codex-work/manifest-layout/native-paths.json
python3 tools/verification/manifest-layout-baseline.py .codex-work/manifest-layout/previous/frozen-runtime-paths.json
python3 tools/verification/manifest-layout-reference.py
cargo build -p mo-text-worker --locked
python3 tools/verification/manifest-layout-regressions.py
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p mo-text --target wasm32-unknown-unknown --locked
```

基线捕获要求旧 CLI/worker 等发布产物摘要符合冻结记录，且输出不存在；已有基线直接复用，不覆盖。207 项回归读取旧报告及其摘要绑定的输入/字体缓存，并把当前响应写入独立目录，不修改历史报告。参考仅使用 Python 标准库与仓库内的有理数参考模块。

本轮冻结记录为[清单布局库级证据](../reviews/evidence/2026-09-25-manifest-layout-library-verification.json)。下一步接入实际 PPTX 的 run/语言/字体槽、文本框内布局、字形变换与文字画笔；当前来源页面仍拒绝含文字的可见对象。autofit、竖排、复杂段落、完整高级内容与播放、Office/WPS 验收及 Musterwork E0–E3 保持未完成。

后续已连接实际原生文本、字体选择与局部字形计算，见[来源字形编译](source-glyphs.md)。本文件的 434 项测试和运行基线仍只描述此阶段。
