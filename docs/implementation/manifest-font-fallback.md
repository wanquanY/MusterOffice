# 显式字体覆盖回退

2026-10-03：原生模板压力测试暴露了清单与排版器之间的缺口。底层已支持按完整字素/塑形边界进行字体回退，但 FontManifest 将每个来源样式收敛成单个候选，宿主无法声明其他字体来覆盖该字体缺失的字符。增加某种语言的测试字符不能自动增加渲染资源。

`ManifestTypeface.fallbacks` 是同一清单内 typeface 名称的有序列表，默认空。段落先选择当前 typeface 的确切样式实例，再依序选择显式列出的 typeface 的同一样式实例。列表是平面的，不递归展开候选自己的列表；对称配置不导致递归或候选膨胀。主字体已有覆盖的字素继续使用主字体；底层现有回退算法保护字素、连字和连接边界，并保留未解析片段与探测证据。

每个样式总共至多 32 个候选（主字体加最多 31 个回退）。未知引用、自身引用、重复引用及超限列表在字体后端调用前拒绝。所有资源哈希、名称记录、字重/宽度轴、未使用实例仍须通过原验证。缺失所请求的 regular/bold/italic/boldItalic 槽会返回具体缺失的 typeface 和样式；不合成粗体或斜体，不用 regular 偷换 italic。宿主持有资源许可、字体字节、替代策略以及显式清单；内核不搜索系统字体或网络。

`ManifestStyleBinding` 保持每个输入样式一项，主字体字段保持原义，新增 `fallbacks` 记录候选的 typeface、face、轴及替代策略。实际采用的字体由原 shaping/flow 结果描述，候选存在不等于实际采用。没有回退的旧 JSON 省略新字段，反序列化后保持单候选行为和原有序列化输出；有回退的清单需要配对的新 SDK/worker，不能单独修改宿主 JSON 并混用旧 worker。

此修复不改变 PPTX 来源中声明的字体，不承诺其他编辑器拥有同一字体包，也不把压力测试或模板质量验收转换成无条件通过。平台字体包缺少相应字符时，应由宿主提供有权使用的资源并更新冻结环境；内核只实施明确的候选选择。

## 验证

- `cargo test --locked -p mo-text --lib`：113 项通过，含 5 个新增清单回退/兼容/边界测试。
- `MO_FONT_CORPUS=/absolute/pinned-corpus cargo test --locked -p mo-harfbuzz-sys --test manifest_fallback -- --include-ignored`：使用既有 `fixtures/fonts/upstream.json` 固定的真实字体；覆盖 regular/bold 的拉丁、中文、阿拉伯文及混排，并验证移除回退后确实出现未解析片段。本次真实执行通过：1 个集成测试覆盖 8 组字体/样式输入及移除回退的反例。
- Schema 与 TypeScript 从 Rust 合同重新生成，生成一致性、TS 编译和 mo-text 严格 Clippy 检查通过。Native/WASM 产物、注册表消费者及实际模板导出另行验收，不以库测试替代。
