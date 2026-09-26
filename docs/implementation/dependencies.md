# 实现依赖与采用证据

2026-09-24 · P00基础依赖。这里登记开发采用，不表示公开许可证已选定。所有workspace包暂为 `publish=false`，不复制Musterwork私有代码。

[产品内容文件恢复](product-content-lifecycle.md)复用既有 fs4 1.1.0（MIT OR Apache-2.0）、uuid 1.24.0（Apache-2.0 OR MIT）、rustix 1.1.4（Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT）和 Tokio。SQLite 的 tempfile 3.27.0 返回测试依赖；没有升级外部版本或改变计算内核。Windows 目录句柄分支使用 Rust 标准库，实际平台验收仍独立进行。下面流式阶段的依赖变更保留其历史含义。

[产品内容流式存储](product-content-stream.md)复用产品现有 sha2 0.11.0，并将 SQLite 已有 tempfile 3.27.0（MIT OR Apache-2.0）改为生产依赖；产品锁文件仅增加内部库的 sha2 依赖边，外部版本未变。MusterOffice 自身锁文件和计算实现不变；该改动未构建新的发行包，不声明安装体积或整体性能收益。

[Rust 嵌入 SDK](embedded-sdk.md)新增内部 mo-embedded-sdk facade；22 个生产库及 48 个 registry 包均来自当前已锁定依赖，没有选择新的外部版本。生成 SDK 移除开发依赖、禁用自动测试目标，保留原生产源码和所需数据/notice；完整发行许可闭包仍待验收。产品准备脚本只用已有 Node 标准库，未改变产品 Cargo/pnpm 锁。SDK 源码归档大小不代表运行体积，平台 worker 单独绑定摘要。

[产品原生清单读取](product-office-manifest.md)只使用 Musterwork 已有 serde、摘要/身份类型、浏览器 Web Crypto 和现有 TS 测试工具，没有新增外部依赖或改变两仓锁文件。产品私有实现留在其原仓库；只有本仓库自有、已封存的测试交付文件进入产品测试。内核与组件二进制未改动，不能据此宣称新的体积或性能收益。

[嵌入式原生导出](embedded-export.md)增加内部 mo-native-export 库与 mo-export-worker 进程，复用已有 OPC、操作服务、交付接收、Skia/HarfBuzz 与诊断；没有新增外部组件或版本。父进程适配器没有 SQLite、原生绘制 FFI 或异步运行时依赖，子进程包含绘图/塑形组件。两者均为 publish=false 的开发实现；完整产品依赖闭包、许可证发行材料和安装包仍须单独验收。

[原生 MCP stdio](mcp-stdio.md)在独立 `tools/mo-mcp` workspace 使用锁定 SDK/异步依赖，连接现有宿主；新增 [base64 0.22.1](../../components/base64/component.json) 仅编码标准资源 blob，MIT/Apache 原文已保留。当前解析图含 106 个 registry 包（含目标条件分支），不是完整许可闭包或实际链接体积结论。根 Cargo.lock 和计算/WASM 源码不变。下面的选型实验说明保留其当时范围，正式开发进展以新阶段为准。

[原生持久调度](native-scheduler.md)使用 Rust 标准线程、Condvar 和既有 SQLite owner，没有新增外部运行依赖。`mo-standard-host` 增加内部 `mo-presentation-compile` 开发依赖，仅用于真实 renderer 协调集成；MCP 候选继续留在独立实验 workspace。原生线程预算不等于完整内存预算，完整依赖发行与包体积继续单独验收。

[MCP 选型实验](mcp-sdk-evaluation.md)单独锁定 rmcp 3.4.0、Tokio 1.53.1、tokio-util 0.7.19、futures 0.3.34，主 workspace 的 Cargo.lock 不变。候选仅用于开发探针，关闭 SDK 默认 features；完整[上游许可](../../components/rmcp/LICENSE-UPSTREAM.txt)包含 MIT→Apache-2.0 过渡说明及两份文本，不能仅按 Cargo 的 Apache-2.0 声明处理。正式生产采用仍需实际依赖/许可闭包、并发调度与完整接入验证。

[操作发现](operation-discovery.md)让 WASM 薄绑定引用已有纯操作服务，Schema 身份计算与文件生成器共享 `mo-common` 实现；仅增加内部依赖边，没有新增外部版本或底层组件。运行时 Schema 查询使相应生成代码进入 WASM 实际链接，本轮 Rust 模块增加 973,479 原始字节和 152,917 gzip-9 字节；这不包括 JS、底层组件、字体和媒体。宿主、SQLite、原生进程不进入这条纯操作依赖链，实际产物及依赖图见[阶段证据](../reviews/evidence/2026-09-26-operation-discovery-verification.json)。

[导出宿主](export-host.md)仅连接已采用的内部包：纯操作服务复用交付、PPTX/OPC/PNG 计算；标准宿主持有 SQLite，CLI 配置 Native renderer 和隔离 worker。未选择新外部组件或升级外部版本。当前依赖树、源码和实际二进制绑定[本轮证据](../reviews/evidence/2026-09-26-export-host-verification.json)；纯操作层 WASM 检查不含 SQLite、原生进程或绘制 FFI。正式发行仍需完整平台/字体/媒体闭包和 notices。

| 依赖 | 锁定版本 | 用途 | 上游许可 |
| --- | --- | --- | --- |
| Rust工具链 | 1.92.0 | Native/WASM、格式化及静态检查 | MIT / Apache-2.0 工具链条款 |
| serde | 1.0.229 | 类型化序列化，拒绝未知字段 | MIT OR Apache-2.0 |
| serde_json | 1.0.151 | 受约束的JSON wire与规范化输入 | MIT OR Apache-2.0 |
| schemars | 1.2.2 | 从Rust类型生成2020-12 Schema | MIT |
| sha2 | 0.10.9 | 文档/操作内容摘要 | MIT OR Apache-2.0 |
| thiserror | 2.0.20 | 类型化错误 | MIT OR Apache-2.0 |
| libm | 0.2.16 | 跨 Native/WASM 的颜色与原生几何数学；关闭默认特性，启用 force-soft-floats | MIT |
| read-fonts | 0.43.3 | 字体元数据与 cmap 读取；关闭默认特性、启用 libm；上层另做字节/语义/预算检查 | MIT OR Apache-2.0 |
| unicode-bidi | 0.3.18 | 双向层级算法；关闭默认特性，显式覆盖分类和括号为 Unicode 18 | MIT OR Apache-2.0（本组件选择 MIT） |
| proptest | 1.11.0 | 测试中的边界与性质验证 | MIT OR Apache-2.0 |
| wasm-bindgen / CLI | 0.2.126 | WASM 薄绑定与开发产物生成；CLI 不进入发行依赖 | MIT OR Apache-2.0 |
| rawzip | 0.5.1 | ZIP 索引、范围读取与流式写入；结构/关系/预算由内核另行校验 | MIT |
| quick-xml | 0.42.0 | XML 词法读取；命名空间、编码与限制由 mo-xml 统一处理 | MIT |
| flate2 | 1.1.9 | STORE/DEFLATE 中的 DEFLATE 解压/压缩；禁用默认特性、采用 rust_backend | MIT OR Apache-2.0 |
| crc32fast | 1.5.2 | 已有 ZIP 传递依赖，现显式用于流式 PNG 的块 CRC；版本不变 | MIT OR Apache-2.0 |
| json-schema-to-typescript | 16.0.0 | 从生成 Schema 产生 TS 类型；仅开发依赖 | MIT |
| TypeScript | 7.0.2 | 生成类型的严格编译检查与按 AST 声明拆分大文件；后者锁定该版本的 unstable API，仅开发依赖 | Apache-2.0 |

准确传递依赖及 license 字段随 Cargo.lock 和 `cargo metadata` 核查；增加依赖须再次审查。本轮已核查 Cargo 依赖声明，包含 Unicode-3.0 等附加许可；发行时仍需完整 notices 和依赖闭包。Schemars 当前随类型编译，后续发布构建需要测量并通过 feature 隔离避免把生成器带入运行包。TS 绑定来自同一类型/Schema 源，不另建手写文档模型；pnpm 传递依赖由根锁文件固定。

WASM CLI 安装在忽略的 `.codex-work/toolchain`，没有改动全局工具链。其上游锁文件报告开发依赖 `der 0.8.0` 已撤回、`buf_redux`/`multipart` 存在未来编译兼容提示；本次只调用本地 WASM 转换命令。这些不进入当前核心依赖，但正式构建工具链冻结前必须复查/更新工具依赖并重跑生成与一致性验证。

HarfBuzz 已固定源码构建，并通过原生工作进程与独立 WASM 实例接入 Rust 文本计算。Skia 已按固定提交实现 CPU 路径组件，并连接 Rust 路径渲染操作与隔离进程；媒体组件尚未作为实现依赖。本机有相关库不等于发行可以依赖本机安装。

颜色计算增加内部 `mo-color` 与外部 `libm 0.2.16`，未升级已有 Cargo 依赖。`force-soft-floats` 禁用体系结构/内建快速路径；运行时不调用宿主的隐式数学实现。锁文件、启用特性和来源声明与[颜色求值证据](../reviews/evidence/2026-09-24-pptx-colors-verification.json)绑定。标准预设色来自 ECMA 数值事实，未复制第三方办公库代码；完整颜色兼容性仍待目标软件验证。

OPC 新增传递依赖包括 `miniz_oxide 0.8.9`、`adler2 2.0.1`、`simd-adler32 0.3.10`、`crc32fast 1.5.2`；声明许可随[本轮快照](../reviews/evidence/2026-09-24-opc-dependencies.json)记录。对 ZIP 路径、解压预算、重复部件、局部/中央记录、描述符范围与 OPC 关系进行独立校验，不将库成功读取视为完整 OPC 验证。描述符规则按 [PKWARE APPNOTE 4.3.9](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT)实现并用 Python 独立写入器覆盖小文件 ZIP64 和流式输出。

基础 PPTX writer 本轮只增加内部 `mo-pptx` crate，没有增加外部 Cargo 依赖。独立验证使用本机 `python-pptx 1.0.2`（声明 MIT）、`lxml 6.0.2`（声明 BSD-3-Clause），合同验证使用 `jsonschema 4.26.0`（声明 MIT）；这些是开发工具，非核心运行依赖。官方 ECMA XSD 仅保存在忽略的缓存目录，记录来源与摘要，不复制到发行包。LibreOffice 26.2.0.3 仅用于外部打开/渲染/重存验证；已安装的 WPS 12.1.22553 尚未执行验证。

PDF 检查/预览使用桌面环境提供的 PyMuPDF 1.28.0，其包声明 AGPL-3.0 或 Artifex 商业双许可；未链接核心、未复制或选择为项目发行依赖。本轮记录只声明实际开发验证工具和已安装包的许可元数据，不代替完整分发许可审查，见 [PPTX 依赖快照](../reviews/evidence/2026-09-24-pptx-export-dependencies.json)。

MCE 逻辑处理在现有 `mo-xml` 和 `mo-pptx` 中实现，没有新增外部 Cargo/pnpm 依赖。ECMA-376 Part 3 第五版 PDF 从官方站点获取，仅在忽略的开发缓存中阅读；下载与 PDF 摘要记录于 [MCE 证据](../reviews/evidence/2026-09-24-pptx-mce-verification.json)。独立树投影继续使用开发工具 lxml，没有把办公软件或 Python 解析器引入内核运行依赖。

字体检查新增内部 `mo-font` 及 `read-fonts 0.43.3`；传递新增 `font-types 0.12.5`（MIT OR Apache-2.0）、`core_maths 0.1.1`（MIT）、`bytemuck 1.25.2` 与 `bytemuck_derive 1.12.1`（Zlib OR Apache-2.0 OR MIT），没有升级既有版本。精确闭包与特性绑定[字体资源证据](../reviews/evidence/2026-09-24-font-resources-verification.json)。版本与本轮塑形候选共享解析层，但采用解析器不等于采用塑形后端。

FontTools 4.61.1（MIT）仅用于独立开发验证及生成原创测试字体；下载 wheel 与官方 PyPI 哈希核对。五个 Noto 字体及相应 OFL 1.1 文本按固定上游 commit/文件摘要保存在忽略缓存，未纳入运行包；字体发行授权及字体包成本仍须独立审核。HarfRust 0.13.3（MIT）仅核对候选源码，发现公开塑形 API 的内部成功状态可见性缺口，未作为实现依赖，也未修改上游源码或系统安装。

HarfBuzz 14.5.0 按[组件锁文件](../../components/harfbuzz/lock.json)固定官方 tar.xz 的 SHA-256/长度，构建 Native 静态库及独立 WASM。已阅读上游 `COPYING`，其声明为 Old MIT，并要求检查适用子目录的附加许可；Unicode 数据等 notices 与完整发行闭包仍须归档，不能只列一个 MIT 标签视作完成。没有迁入未核实的办公库或私有代码。组件 profile 与输入/失败边界见[实现说明](harfbuzz-component.md)。

组件 Native 本轮使用 Apple Clang 17.0.0，确定性归档使用本地 SDK 的 LLVM `llvm-ar` 24.0.0git。WASM 使用 Emscripten 6.0.10，emsdk 源码 commit、下载源码归档摘要和 SDK release ID 记入[组件证据](../reviews/evidence/2026-09-24-harfbuzz-component-verification.json)。SDK 仅安装在忽略目录，没有改动全局 shell 配置；官方 SDK 二进制压缩包被安装器自动删除，本轮未取得该压缩包的 SHA-256，不宣称整个工具链供应链已完成逐字节锁定。实际编译器版本、命令和产物已记录；Emscripten MIT/UIUC-NCSA 声明以及实际链接的 LLVM/libc/运行库完整 notices 仍待发行归档。

同版未修改的 `hb-shape` CLI 用作开发对照，依赖本机 GLib 2.86.4（上游 LGPL-2.1-or-later 声明，发行需以实际闭包复核）。GLib、FontTools、Node 和测试观察器均不作为组件的运行依赖。没有把系统 `hb-shape` 的不同版本混用为本次参照。

Skia `8d6d37b063afe87fd361de55359fb4cb6b6f443c` 及内部 skcms 已进入独立绘制组件，源码归档、GN/Ninja、裁剪参数和实际构建证据见[Skia 组件](skia-component.md)。没有新增 Cargo/pnpm 外部依赖或另一份字体引擎。维护的 GN 修改只增加外部 SDK/线程的显式构建开关；绘制源码不变。BSD 和 Khronos 头文件声明已保留，最终工具链/平台运行库 notices 仍待完整发行归档。当前 CPU profile 的小体积不能代替完整演示内核或安装包估算。

文本接入新增内部 `mo-text`、`mo-harfbuzz-sys`、`mo-text-worker` 和 TS 薄层，未增加外部依赖版本；原生 build 复用已有 sha2/serde_json 校对组件产物。unsafe 只在私有 C ABI 文件允许；Rust 计算核心仍禁止 unsafe。Native 运行时依赖编译器的 C++ 标准库，WASM 使用所记录的 Emscripten 运行库，不能把系统提供的 C++ 库记为全平台零成本。最新构建和端到端证据见[文本塑形记录](../reviews/evidence/2026-09-24-text-shaping-verification.json)。

Unicode 基础新增内部 `mo-unicode`，没有新增外部 Cargo 版本。官方 Unicode 18.0.0 属性与测试数据按长度/SHA-256 固定；运行时生成表 16502 字节，Unicode-3.0 许可原文随源码保留。再生成不复制第三方算法实现。`unicode-segmentation 1.13.3`（MIT OR Apache-2.0）只在忽略目录的独立开发对照工程中运行，其内部 Unicode 17 在官方 Unicode 18 语料上有 10 条差异，未采用为内核依赖。来源与实际结果见[Unicode 证据](../reviews/evidence/2026-09-24-unicode-text-verification.json)。

字体候选计算复用已有内部 `mo-font` / `mo-text` / `mo-unicode` 和固定 HarfBuzz；没有新增外部依赖或数据版本。借用的已验证字体对象不改变字体许可、字体净化或隔离边界。实际绑定与验证见[字体候选证据](../reviews/evidence/2026-09-24-font-cascade-verification.json)。

双向文字新增 `unicode-bidi 0.3.18`，关闭所有默认特性，没有引入其他传递版本。内置 Unicode 16 数据不用于本实现；`BidiDataSource` 的分类与 normalized opening bracket 两个方法均由固定 Unicode 18 数据覆盖。发布归档及 MIT/COPYRIGHT/AUTHORS 保存依据见[组件记录](../../components/unicode-bidi/component.json)，Unicode 18 生成表为 14007 字节，沿用 Unicode-3.0 许可。完整官方语料和数据验证见[双向文字实现](bidi.md)及[证据](../reviews/evidence/2026-09-24-bidi-verification.json)。

自动分段与段落准备未增加 Cargo/pnpm 外部依赖。新增官方 Unicode 18 Script/Script_Extensions/PropertyValueAliases 数据锁，运行表 16670 字节；许可沿用已有 Unicode-3.0 原文。算法与数据分开记录，不把本项目分段策略冒称唯一标准算法。实际闭包与字体/上游塑形对照见[自动分段证据](../reviews/evidence/2026-09-24-text-itemization-verification.json)。

混合字体回退复用同一套固定 Unicode / HarfBuzz / 字体资源实现，没有新增外部依赖、特性、字体或数据包。候选探测、边界重塑及实际组件故障的产物与源码绑定见[回退证据](../reviews/evidence/2026-09-24-font-fallback-verification.json)。

默认断行在现有 `mo-unicode` 中实现，没有增加内部 crate 或外部版本/特性。官方 LineBreak/EastAsianWidth/DerivedGeneralCategory/LineBreakTest 固定为 Unicode 18，UAX #14 固定 revision 57；运行表 31752 字节，数据沿用 Unicode-3.0 许可。未复制第三方算法代码，未将宿主 Unicode 表作为数据源；实际闭包与验证见[断行证据](../reviews/evidence/2026-09-24-line-break-verification.json)。

字体实例度量复用固定 HarfBuzz 14.5.0 和 FontTools 4.61.1 开发验证，没有新增外部版本或 Cargo feature。新增 C++ 接口进入原有独立组件，FontTools/参考可执行程序不进入产品依赖；原创 MVAR/avar 字体仅为测试输入。构建与闭包见[实例度量证据](../reviews/evidence/2026-09-24-font-metrics-verification.json)。

行上下文重塑仅调整现有 Rust 模块与绑定，没有新增 crate、依赖版本或 feature，C++ 模块及 TS 组件适配器保持原字节。当前独立参考沿用已固定上游 CLI；闭包和回归绑定见[行上下文证据](../reviews/evidence/2026-09-24-line-shaping-verification.json)。

显式行几何在现有 Rust 文本模块内实现，复用已校验字体和度量路径；没有新依赖、feature、字体或 Unicode 数据，C++ 与 TS 组件适配器字节不变。实际闭包与产物见[几何证据](../reviews/evidence/2026-09-24-line-geometry-verification.json)。

自动段落布局重用现有 Rust Unicode/文本模块，新增可复用段落 bidi 和候选行计划；无新外部版本、feature、字体或属性表，组件字节保持不变。源码闭包与实际产物见[自动布局证据](../reviews/evidence/2026-09-24-paragraph-layout-verification.json)。

字体轮廓扩展已固定的 HarfBuzz 14.5.0 组件，未新增外部版本或 feature；构建加入 `-ffp-contract=off`，Native/WASM、旧塑形/度量和故障注入均重验。新增原创测试字体由既有 FontTools 4.61.1 生成，未加入运行包。源码与实际大小见[轮廓证据](../reviews/evidence/2026-09-24-font-outlines-verification.json)。

段落路径新增内部 `mo-geometry`，复用已有 serde/schemars/thiserror/mo-common；行布局 Q32 算术迁入共享模块。没有新增第三方版本、feature 或 Unicode 数据，C++/TS 组件保持原字节，详见[段落路径证据](../reviews/evidence/2026-09-24-paragraph-paths-verification.json)。

路径渲染新增内部 `mo-raster`、`mo-skia-sys`、`mo-raster-worker`，无新外部 Cargo/pnpm 版本或 feature。Native 在已有同版 Skia 归档之外单独构建私有 C ABI 适配归档，Rust build 核对双方与组件源码摘要；没有重复引入图片/字体引擎或本机办公库。WASM 只增加类型化字节/长度检查，复用现有 wasm-bindgen。实际闭包与产物见[路径渲染证据](../reviews/evidence/2026-09-24-path-raster-verification.json)。

仿射场景新增内部 `mo-render`，以及 `num-bigint 0.4.8` / `num-integer 0.1.47`。`num-traits 0.2.19` 原为已锁定开发依赖，现在也进入数值运行闭包；workspace feature 新增兼容 `i128`。普通计算走 checked i128，只有中间值需要更宽表示时使用有界精确运算。归档与双许可原文记录于[数值组件](../../components/rust-numeric/component.json)，组件选用 MIT；这不改变项目整体尚未公开授权的状态。没有新增字体、数据包或 C++/TS 组件版本，实际依赖图和产物见[场景证据](../reviews/evidence/2026-09-24-scene-raster-verification.json)。

作者页面坐标的 Q96 区间和角度求值复用上述 num-bigint 运行依赖，没有新增外部版本或 feature；仅新增内部 `mo-presentation-compile`。PyMuPDF/Pillow 仅用于开发期的 PDF 观察和真实像素编码，不进入运行产物。验证范围见[作者页面坐标](page-placement.md)。

作者页面绘制复用现有 Q96 三角、mo-geometry、mo-render、mo-raster 与 Skia，没有新增外部包或版本；`mo-presentation-compile` 新增两个内部 crate 依赖。曲线精度与当前边界见[作者页面绘制](page-render.md)。

来源几何求值复用已有 `libm 0.2.16` 和相同特性，Cargo.lock 仅增加 `mo-pptx` 到该既有包的依赖边，没有新增版本或传递依赖。原生公式解释器自行实现；标准几何数据只在忽略的验证目录中使用，没有迁入第三方办公库代码。数值解释与未完成的目标应用验证见[几何求值](geometry-evaluation.md)。

原生路径编译复用既有 Q96 区间和 `num-bigint 0.4.8`，没有新增外部版本或 feature。内部增加 `mo-presentation-compile → mo-pptx`（消费来源求值类型）和 `mo-kernel-api → mo-geometry`（公开 Q32 容差类型）；格式读取仍由 `mo-pptx` 持有，计算层不引入宿主 I/O。独立开发验证继续使用 lxml 与 Decimal；没有复制外部办公库的弧算法或模板，见[原生路径](native-paths.md)。

187 种预设展开采用固定的 ECMA-376 Part 1 附件数据及显式实现派生目录，来源、逐项修订和默认 Ecma 版权许可全文见[组件记录](../../components/drawingml-presets/README.md)。运行表及索引共 350386 字节；使用已有 XML/导引实现，没有新增 Cargo/pnpm 版本或运行文件依赖。它不改变项目整体未选定公开许可证的状态，完整发行需携带相关 notices。
