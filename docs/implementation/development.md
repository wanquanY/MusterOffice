# 开发与验证

日常代码改动使用[当前代码门禁](current-verification.md)：`tools/verification/current.py` 统一执行 Rust、独立 MCP、合同、TS、真实 worker 和文档 Native/WASM 检查，参数与先决条件显式传入，每次使用新输出目录。宿主数据库当前为 schema 6，升级／回滚边界和未关闭架构问题见[评审整改](review-corrections.md)。

下方按阶段保存历史验证方法与当时版本，供重放旧证据使用；其中冻结脚本的复制／改写方法不作为新增测试的组织方式。当前门禁不代替各专项的视觉、Office/WPS、产品链路和发行性能验收。

原生 MCP 服务在独立 `tools/mo-mcp` workspace 构建，配置、真实协议及二进制命令见[适配器说明](../../tools/mo-mcp/README.md)。本阶段使用 `.codex-work/mcp-server/`，依次运行包内 `check.py`、`check_transport.py` 和 `check_files.py`，每次失败后的重跑使用新目录。`tools/verification/mcp-stdio-evidence.py` 封存当前范围，`--check` 复核；不要覆盖历史阶段。该服务未达到完整 MCP/HTTP/SDK/Skill/Plugin 或 Musterwork 产品验收。

Rust 工具链由根目录 `rust-toolchain.toml` 固定为 1.92.0，依赖由 `Cargo.lock` 固定。所有本地包 `publish=false`。计算核心禁止 unsafe，无文件/网络/系统字体访问；原生组件的私有 FFI 集中在 `mo-harfbuzz-sys` / `mo-skia-sys`。完整 workspace 验证前先按[塑形说明](harfbuzz-component.md)和[绘制说明](skia-component.md)准备无注入 Native 静态库，构建时校对其摘要；Rust WASM 主模块不链接该静态库。

## 原生持久任务调度阶段

[原生调度](native-scheduler.md)使用独立 `.codex-work/native-scheduler/`。先生成 Schema/TS，再运行 `native-scheduler-workspace.py`。该程序包含真实 worker 集成、全仓及最终宿主检查，数据库当前版本为 v5。构建后用具备 jsonschema/lxml/python-pptx/Pillow 的 Python 运行 `native-scheduler-reference.py <新目录>`，核对持续控制会话、自动导出、旧库副本升级和十份实际 Native/WASM Schema；随后运行 `native-scheduler-parity.py <新目录/export/candidate>`。旧版验证程序按摘要冻结，本阶段只在新目录生成预期版本变为 v5 的重放副本，不修改旧证据。

开发入口为 `mo-host <database> <principal> <scope> --scheduled 2`，可附加 `--preview-worker <path> <sha256>`。控制流是持久 NDJSON，`job/sync/auto` 都由同一有界执行池执行。二进制上传/下载目前通过该数据库的独立 `append/read-asset` 命令；完整 MCP 资源端口仍待接入。

## 统一宿主分发与发现阶段

[操作发现](operation-discovery.md)使用独立 `.codex-work/operation-discovery/`。`operation-discovery-workspace.py` 核对并复用冻结的全仓验证程序，仅重定向输出目录，再构建 release CLI/worker。`operation-discovery-reference.py <新目录>` 用具备 jsonschema/lxml/python-pptx/Pillow 的 Python 环境运行，查询实际 Native/WASM Schema、读取旧 v4 公开交付，并重跑完整开发 profile 导出。随后运行 `operation-discovery-parity.py <该目录/export/candidate>`；最终 `operation-discovery-evidence.py` 封存并用 `--check` 复核。所有原阶段源码/输入/产物保持不变，不覆盖旧证据。

## 导出作业与公开资产阶段

[导出宿主](export-host.md)使用独立 `.codex-work/export-host/`。`export-host-workspace.py` 检查全仓、生成合同、纯操作层 WASM、实际 Native 集成及依赖边界；显式集成绑定本次 worker 的摘要。release CLI 与 worker 构建后，`export-host-reference.py <新目录>` 执行真实创建/上传/导出/读取和旧库副本迁移，使用包含 jsonschema/lxml/python-pptx/Pillow 的 Python 环境。随后运行 `export-host-parity.py <该目录/candidate>` 对当前 CLI/WASM 及公开交付页面进行独立重放和历史比较，由 `export-host-evidence.py` 封存并 `--check`。脚本拒绝覆盖旧产物，失败后的重跑使用新目录；workspace 各检查可从指定步骤继续并保留历次日志。

## 演示文稿交付计算阶段

[私有交付计算](presentation-delivery.md)的输出和失败记录位于 `.codex-work/delivery-pipeline`。`delivery-workspace.py` 执行全仓、合同、WASM 构建和依赖检查；另显式运行真实 worker 集成测试、`delivery` 例程、`delivery-reference.py` 和 `delivery-parity.mjs`。本阶段新建 PPTX 的文字默认样式有意改变，不能再要求新建文件与旧版本字节相同；需要独立核对 XML 差异并对新构建运行 Native/WASM 一致检查。已有来源文件的读取与修改仍要求保持历史结果。详见阶段记录中的输入和验证边界。

`delivery-native.py` 绑定实际 worker 摘要后执行磁盘例程、显式原生集成和独立检查；`delivery-style-delta.py` 对照冻结旧 Writer 的七份文件并运行锁定 XSD。`delivery-evidence.py --check` 复核已有阶段记录。运行脚本使用忽略目录中的全新输出名；历史产物和失败记录不覆盖。当前冻结记录已经存在，后续开发使用新的阶段目录和证据，不重写历史结论。

## 任务私有输出阶段

当前[任务私有输出](job-results.md)使用独立 `.codex-work/job-results` 目录。运行 `tools/verification/job-results-workspace.py` 检查全仓、同源合同和依赖边界，再运行 `job-results-reference.py`；后者只复制此前封存的 v2 数据库，调用 release `job_output` 例程和 `mo-host`，独立核对实际结果块、旧表内容和真实租约过期清理。最后用 `job-results-evidence.py` 封存并 `--check`。这个阶段不改核心/WASM 算法，历史内核产物按摘要核对，不把历史批次计作新增绘制或互操作验收。

## 流式封存导出阶段

当前私有输出阶段还运行 `job-results-parity.mjs`，将当前 debug CLI 的作者导出、来源读取/编辑及文档编辑与冻结 WASM 比较。debug 可执行文件本身不假定与父阶段相同；所有新报告和旧结果保持独立。

[封存导出](sealed-export.md)使用新的 `.codex-work/sealed-export` 目录。`sealed-export-workspace.py` 运行全仓、合同、Native/release/WASM 构建及依赖边界检查；`sealed-export-parity.mjs` 运行既有作者导出/来源编辑/文档编辑双端程序，并与上阶段 WASM 比较。`sealed-export-benchmark.py` 在构建和测试结束后顺序测量两个本地 debug CLI，包含预热、原始 RSS/耗时和字节一致性，不能外推到发行版或桌面安装包。`sealed-export-independent.py` 运行已固定的独立文件校验；`sealed-export-publication.py` 核对共享暂存路径的冻结像素输出，最终由 `sealed-export-evidence.py` 封存并 `--check`。首次基准准备需要在构建改动前留存上阶段开发 CLI 到该目录的 `baseline/mo-cli`，脚本核对其父证据摘要。所有产物和既有证据封存后不覆盖。

## 持久资源宿主阶段

[资源宿主](resource-host.md)产物位于 `.codex-work/resource-host`。先生成同源 Schema/TS，运行 `tools/verification/resource-host-workspace.py` 和 `resource-host-cli.py`；后者要求新的输出数据库并对上一阶段封存数据库的副本执行迁移。通过 `cargo run -p mo-standard-host --example resource_pptx --locked --offline -- .codex-work/resource-host/pptx` 写出真实素材支持的 PPTX，再运行 `resource-host-pptx.py` 独立检查。最后由 `resource-host-evidence.py` 汇总及 `--check` 复核。旧证据、旧产物和旧数据库不能覆盖。底层库及 CLI 功能测试为 `cargo test -p mo-standard-host -p mo-host --locked --offline`。资源范围读取不是导出交付验收，示例也不提供另一套生产发布入口。

## 持久化文档操作宿主阶段

[文档操作宿主](operation-host.md)产物位于 `.codex-work/operation-host`，Native 绘制组件沿用 `.codex-work/gradient-coordinates/component`。依次运行 `tools/verification/operation-host-workspace.py`、`operation-host-dependencies.py`、`operation-host-parity.mjs` 和 `operation-host-cli.py`，由 `operation-host-evidence.py` 汇总并以 `--check` 复核；新阶段不可覆盖已封存产物。release CLI 脚本拒绝复用已有测试数据库。运行前用 `mo-contract-codegen write contracts/generated` 和 `pnpm generate:types` 生成合同，workspace 脚本只校验。独立服务/数据库/CLI 测试可运行 `cargo test -p mo-operation-service -p mo-standard-host -p mo-host --locked --offline`，无需绘制库。数据库路径和身份由 CLI 的操作者参数配置，不能放入 Agent 操作 JSON。SQLite 只进入独立宿主，不进入纯计算或生产 WASM 内核。

## 保留式时间区间复用阶段

[保留式时间区间复用](retained-timing.md)产物位于 `.codex-work/retained-timing`。运行 `retained-timing-workspace.py`，随后运行 `retained-timing-parity.mjs` 和 `retained-timing-replay.py`。构建／测试全部结束后单独运行 `retained-timing-benchmark.mjs`，再由 `retained-timing-evidence.py` 封存并以 `--check` 核对。历史程序和产物不可覆盖；重放只改输出位置和当前模块入口。基准测量实际 prepare 后的作者会话，包括完整序列化和画面计算；不包含文档准备，也不等同于实际显示 FPS。

## 当前容器生命周期阶段

[容器生命周期](container-lifecycle.md)产物位于 `.codex-work/container-lifecycle`。运行 `container-lifecycle-fixtures.py` 和 `container-lifecycle-workspace.py`，随后执行 `container-lifecycle-parity.mjs`、`container-lifecycle-native-reference.py`、`container-lifecycle-replay.py` 和 `container-lifecycle-preview.py`；全部检查结束后单独运行 `container-lifecycle-benchmark.mjs`。最后 `container-lifecycle-evidence.py` 封存并以 `--check` 核对。旧阶段程序在新目录按摘要重放；中间主动停止、Clippy 格式修正和独立参考／脚本错误保留在 attempts。查询成本是同输入暖态公共 WASM 调用，不是 FPS 或最终产品成本。

## 此前行为结束条件阶段

[行为结束条件](end-conditions.md)产物位于 `.codex-work/end-conditions`，Native 组件仍使用 `.codex-work/gradient-coordinates/component`。运行 `end-conditions-fixtures.py` 与 `end-conditions-workspace.py`，随后运行 `end-conditions-parity.mjs`、`end-conditions-native-reference.py`、`end-conditions-replay.py` 和 `end-conditions-preview.py`；最终由 `end-conditions-evidence.py` 封存并以 `--check` 复核。旧来源会话、时间树、变速及重复边界语料在新目录重放，原文件不变。当前没有新增性能基准或外部应用验收。

## 此前重复时长阶段

[重复次数与时长](repeat-bounds.md)产物位于 `.codex-work/repeat-bounds`，Native 组件仍使用 `.codex-work/gradient-coordinates/component`。先运行 `repeat-bounds-fixtures.py`、`repeat-bounds-workspace.py`，再运行 `repeat-bounds-parity.mjs`、`repeat-bounds-native-reference.py`、`repeat-bounds-replay.py` 和 `repeat-bounds-preview.py`；最终 `repeat-bounds-evidence.py` 封存并以 `--check` 核对。旧来源会话、时间树与行为变速语料在新目录重放，旧文件保持不变。当前没有新增性能基准。

## 此前行为时间变换阶段

[行为时间变换](time-transforms.md)产物位于 `.codex-work/time-transform`，Native 组件仍使用 `.codex-work/gradient-coordinates/component`。依次运行 `time-transform-fixtures.py`、`time-transform-workspace.py`，随后运行 `time-transform-parity.mjs`、`time-transform-native-reference.py`、`time-transform-replay.py` 和 `time-transform-preview.py`；最终由 `time-transform-evidence.py` 封存与 `--check` 复核。回归在新目录中重放旧来源会话和时间树，逐字节比较历史结果。旧阶段文件保持不变，当前没有新增性能基准。

## 此前分层时间树阶段

[分层时间树](timing-trees.md)产物位于 `.codex-work/timing-tree`，Native 组件仍使用 `.codex-work/gradient-coordinates/component`。运行 `timing-tree-fixtures.py` 与 `timing-tree-workspace.py`，随后执行 `timing-tree-parity.mjs`、`timing-tree-native-reference.py`、`timing-tree-replay.py` 和 `timing-tree-preview.py`；最终由 `timing-tree-evidence.py` 封存并以 `--check` 复核。回归工具先验证历史脚本摘要，再将副本的运行路径指向本阶段产物；历史文件和已封存结果均保留。当前无新增性能基准，不把功能验证时长当作性能测量。

## 此前来源资源会话阶段

[来源资源播放会话](source-playback-sessions.md)产物位于 `.codex-work/source-session`，Native 静态库仍为 `.codex-work/gradient-coordinates/component`。先运行 `source-session-fixtures.py`、`source-session-workspace.py`、`source-session-parity.mjs`，再运行 `source-session-replay.mjs`、`source-session-author-replay.mjs`、`source-session-source-replay.mjs`。构建/测试均结束后单独运行 `source-session-benchmark.mjs`；最终由 `source-session-evidence.py` 封存并以 `--check` 复核。常驻 Native worker 使用 `--pptx-playback-session`，WASM 使用 `PptxPlaybackSession`，协议见实现文档。旧阶段产物保持不变。

## 此前来源动画阶段

[原生来源动画](source-playback.md)产物位于 `.codex-work/source-playback`，C++ 仍使用 `.codex-work/gradient-coordinates/component`。`source-playback-fixtures.py` 从已封存自有源包添加原生时间树并独立建立静态对照；依次运行 workspace、parity、replay、session-replay 工具。全部构建/测试结束后单独运行 benchmark，最后 evidence 封存与复核。公共 CLI 为 `render-pptx-playback-page <request.json> <source.pptx> <fonts.bin> <new-output.rgba>`；输出仍经隔离 worker 与既有原子发布。

## 此前播放会话阶段

[播放采样会话](playback-sessions.md)产物位于 `.codex-work/playback-session`，C++ 仍指向 `.codex-work/gradient-coordinates/component`。依次运行 `playback-session-workspace.py`、`playback-session-parity.mjs` 与 `playback-session-replay.mjs`；全部构建/测试结束后独立运行 `playback-session-benchmark.mjs`。`playback-session-evidence.py` 封存源码增量、实际常驻命令流、旧像素结果、合同与成本；不要重写旧阶段。原生 `--playback-session` 是显式 JSON/二进制流，仍由宿主负责进程和显示生命周期。

## 此前动画共享绘制阶段

[动画共享绘制](playback-rendering.md)产物位于 `.codex-work/playback-render`。C++ 组件仍指向 `.codex-work/gradient-coordinates/component`，重新构建本阶段 Rust Native/WASM。`playback-render-workspace.py` 完成 workspace/合同；`playback-render-parity.mjs` 执行实际编译、绘制和 CLI；`playback-render-reference.py` 独立核对时间、组合矩阵与直角像素；`playback-render-replay.mjs` 重放上阶段静态结果。全部构建和测试结束后再单独运行 `playback-render-benchmark.mjs`，当前测量包含逐请求计划创建，不能外推保留式播放性能。

## 此前原生旋转时间图阶段

[原生旋转时间图](timeline-rotation.md)产物位于 `.codex-work/timeline`。C++ 组件保持 `.codex-work/gradient-coordinates/component`；Rust WASM 在新阶段单独构建，不覆盖历史产物。依次运行 `timeline-workspace.py`、`timeline-fixtures.py`、`timeline-parity.mjs`、`timeline-reference.py` 和本阶段回归/证据工具。原生公共入口为 `evaluate-timeline <request.json>` 与 `pptx-timing <request.json> <source.pptx>`；它们返回状态或结构化诊断，尚不负责实际播放像素。

## 此前渐变坐标精度阶段

[渐变坐标精度](gradient-coordinate-precision.md)的产物位于 `.codex-work/gradient-coordinates`，原生宿主通过 `MO_SKIA_LIB_DIR` 链接其中的 `component`；Rust WASM 复用 `.codex-work/transform-edit/wasm-node`。先完成独立数值检查和三种 C++ 构建，再运行 workspace、组件、公共入口、刚性变换与独立参考检查；性能测量在这些构建/测试结束后单独执行。

所有复现工具以 `gradient-coordinate-` 开头。旧像素的变化必须记录，元数据只允许已核实的实际像素摘要变化，源帧和其他字段保持核对。既有独立颜色参考和新的零容差坐标/刚性变换检查分开统计。`attempts` 保存未采用或被后续优化取代的产物，不计作当前验证。证据封存后不得覆盖。

## 此前来源对象变换编辑阶段

[对象变换编辑](source-transform-edit.md)的产物位于 `.codex-work/transform-edit`，C++ 仍使用 `.codex-work/elliptic-fast/component`。先运行 `transform-edit-checks.py` 构建原生宿主和 Rust WASM，并生成/检查同源 Schema/TS；`transform-edit-fixtures.py` 生成自有原生输入；`transform-edit-parity.mjs` 执行真实双端及旧接口回归；`transform-edit-reference.py` 使用 Expat、lxml/libxml2 和官方 XSD 独立检查文件。

`transform-edit-render.mjs` 比较两端绘制，并报告独立像素变换差异；`transform-edit-numeric.py` 记录逆矩阵浮点诊断。它们正常退出只表示观察完成，报告中的 `rigidTransformInvarianceAchieved=false` 和未解决状态必须保留，不能当作精度通过。已有阶段封存后不得覆盖。

## 此前来源属性编辑阶段

[来源属性底座](source-attribute-overlay.md)的产物位于 `.codex-work/attribute-edit`，C++ 组件仍为 `.codex-work/elliptic-fast/component`。`attribute-edit-checks.py` 检查 workspace、同源合同，重建原生宿主与 Rust WASM；另将同一 `mo-xml` 编译进无裸指针的 Native/WASM 诊断探针，不增加产品导出接口或依赖。

`attribute-edit-reference.py` 生成独立词法预期，并用 lxml/libxml2 校对返回 XML；`attribute-edit-product-parity.mjs` 比较当前 Native/WASM 与上一阶段 WASM 的来源检查、文字编辑及 CLI 文件提交。`stationary-gradient-fixtures.py` 只准备自有观察输入，不产生 WPS 结论。已有阶段产物封存后不得覆盖；后续工作使用新目录。

## 此前椭圆渐变性能阶段

[性能实现记录](elliptic-gradient-performance.md)对应新的 `.codex-work/elliptic-fast/component`。使用 `build-skia.py` 在该目录分别构建 Native、WASM 和 Native `--sanitize`，继续使用 `.codex-work/image-codec/deps-native`、`deps-wasm`、`deps-asan` 的固定依赖。当前 C++ 源码已变化，Rust 的 `MO_SKIA_LIB_DIR` 必须指向新构建；旧目录只用于历史对照，不能当作当前源码构建。

`elliptic-fast-checks.py` 构建独立数值探针并运行精确参考，`elliptic-fast-workspace.py` 验证 workspace 和重建原生宿主，`elliptic-fast-components.mjs` 重放底层语料，`elliptic-fast-replay.mjs` 重放实际公共接口及 CLI。Rust WASM 与 TS 没有改变，沿用已封存的 `.codex-work/elliptic-source` 产物。`elliptic-fast-benchmark.mjs` 与 `elliptic-fast-pages-benchmark.mjs` 分别测量底层绘制和真实 PPTX API，测量时不并行执行其他构建/测试。`elliptic-fast-evidence.py` 绑定源码、结果、构建与历史增量。已有阶段封存后不得覆盖，后续必须使用新目录。

## 此前圆形渐变来源页面阶段

[原生圆形渐变页面](elliptic-source-pages.md)阶段使用当时固定的 `.codex-work/elliptic-render/component`，当时无新增 C++ 构建或依赖。`elliptic-source-checks.py` 执行 workspace 测试、同源合同和 Native/WASM 构建，结果在 `.codex-work/elliptic-source`；`elliptic-source-fixtures.py` 生成自有失败输入；随后运行 `elliptic-source-final-checks.py`，核对公共接口、历史结果、独立参考、CLI 提交和 Sanitizer。`elliptic-source-evidence.py` 封存源码增量与实测产物。已有产物封存后不得原地重跑，应新建阶段目录。

## 此前椭圆绘制阶段

[V12 绘制组件](elliptic-gradient-raster.md)使用 `.codex-work/elliptic-render/component`，与 V11 相同的固定解码依赖，分别构建 Native、WASM、ASan。Rust 使用 `MO_SKIA_LIB_DIR` 指向新目录；当前组件源码已变化，不能再用旧构建记录假装与新源码一致。`elliptic-render-workspace.py` 运行现有 Rust/合同检查，`elliptic-render-components.mjs` 核对三端像素、拒绝和回归，`elliptic-render-reference.py` 运行独立像素参考，`elliptic-prepared-checks.py` 验证预计算核，`elliptic-render-benchmark.mjs` 测量实际绘制调用。产物冻结后不得覆盖；后续使用新输出目录。

## 此前椭圆标量阶段

[有界首次交点求解](elliptic-gradient-scalar.md)是独立 C++ 计算核，尚未链接进 Skia 或公共页面 API。`python3 tools/verification/elliptic-scalar-checks.py` 构建并核对 Native/WASM/ASan/UBSan；`node tools/verification/elliptic-scalar-benchmark.mjs` 执行独立微基准。固定 Emscripten 已安装，脚本离线运行；本阶段不安装依赖或重建 Rust。结果目录为 `.codex-work/elliptic-scalar`，已有冻结产物不得覆盖。下一阶段必须新建输出目录再继续验证。

## 此前径向焦点校准阶段

[WPS 校准记录](radial-gradient-calibration.md)说明 v2 的参考矩形修正和未关闭的精度差异。使用 `radial-anchor-checks.py`，产物写入 `.codex-work/radial-observation`，继续复用固定 `.codex-work/rect-gradient/component`。一份请求合同只增加 profile 枚举；v1 几何结果保留。实际像素着色器尚未实现。

## 此前径向几何阶段

局部径向布局的 Native/WASM/CLI、两份新增合同及独立高精度验证见[径向渐变几何](radial-gradient-layout.md)。`radial-layout-checks.py` 复用已固定的 `.codex-work/rect-gradient/component`，新 Rust/WASM/TS 产物写入 `.codex-work/radial-layout`。当前无需修改或重建 C++ 组件；查询只输出几何，不输出径向页面像素。

## 此前矩形渐变阶段

当前源码需要包含 V11 矩形场查询符号的 C++ 组件。构建目录使用 `.codex-work/rect-gradient/component`，分别对应 Native/WASM 与 native-asan 的固定解码依赖；构建全部结束再运行 `rect-gradient-checks.py`。共享场、17 字渐变头、数值范围及验证入口见[原生矩形渐变](rectangular-gradients.md)。不要混用旧组件的静态库与新查询头。

## 此前 Office 渐变阶段

该阶段源码需要包含 V10 可选 Office 插值能力的 C++ 组件；旧组件不能链接新增 Native 查询符号。使用新目录 `.codex-work/office-gradient/component` 构建 Native/WASM 与 native-asan，命令仍为下面的 `build-skia.py`，只更换目录且 ASan 对应 `deps-asan`。实际协议、JSON 浮点功能及验证入口见[Office 渐变插值](office-gradient-interpolation.md)。当前固定 serde_json 开启 `float_roundtrip`，不要在下游独立打包时省略该功能。

## 此前图片组件构建示例

当前 `mo-skia-sys` 要求带解码及源区域采样扩展的固定构建。先按[编码图片说明](image-codec.md)取得锁文件中的源码包，放入显式 archives 目录；下列命令不会联网。既有组件和历史发行产物不得覆盖。新采样扩展与 ABI 兼容范围见[图片源区域](image-domain.md)。

```sh
python3 tools/components/build-image-codecs.py --target native --directory .codex-work/image-codec/deps-native --archives .codex-work/image-codec/downloads
python3 tools/components/build-image-codecs.py --target wasm --directory .codex-work/image-codec/deps-wasm --archives .codex-work/image-codec/downloads
# component 目录也需包含既有锁定 Skia 源码压缩包。
python3 tools/components/build-skia.py --target native --directory .codex-work/image-domain/component --image-codecs .codex-work/image-codec/deps-native
python3 tools/components/build-skia.py --target wasm --directory .codex-work/image-domain/component --image-codecs .codex-work/image-codec/deps-wasm
export MO_SKIA_LIB_DIR="$PWD/.codex-work/image-domain/component"
```

WASM 的 TS 适配层使用新的输出目录：`node_modules/.bin/tsc --project packages/raster-component/tsconfig.json --outDir .codex-work/image-domain/ts-raster`。Node bindgen 输出目录需写入 `{"type":"commonjs"}` 的 `package.json`。不要覆盖旧阶段冻结的组件。

以下是此前图片分辨率阶段的重现命令；该阶段复用当时固定 C++ 组件和 TS 薄层，只重建 Rust Worker / WASM / 生成合同。当前源码应使用上面的源区域组件并将新验证保存到新的输出目录，不能用当前源码覆盖历史证据。`jpeg-fixture` 是解码阶段用固定 libjpeg 构建的自有样例生成器。

```sh
python3 tools/verification/image-resolution-fixtures.py .codex-work/image-resolution/cases .codex-work/image-codec/jpeg-fixture
node tools/verification/image-codec-parity.mjs .codex-work/image-resolution
node tools/verification/image-codec-parity.mjs .codex-work/image-resolution/codec-regressions .codex-work/image-codec/cases .codex-work/image-codec/component .codex-work/image-codec/ts-raster/index.js .codex-work/image-resolution/wasm-node .codex-work/image-codec/parity.json
```


原生 Worker 的 `--decode-image` 以两个 little-endian u32 输入长度、请求 JSON、编码字节为一帧；输出沿用两个长度、响应 JSON、像素字节。WASM `decode_image` 使用同一 Rust API。成功不表示图片已经绑定到某个 PPTX 页面。

原生图片局部布局的库级验证见[图片布局](image-layout.md)：`cargo build -p mo-skia-sys --example source_image_layout --locked` 构建真实源包/解码验证宿主，`python3 tools/verification/image-layout-reference.py` 生成并核对自有 PPTX。该示例不注册产品协议，也不输出完整页面像素；它验证来源资源、尺寸、局部矩阵和裁剪要求，为共享页面编译器提供计算结果。

## 原生计算与合同

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p mo-contract-codegen --locked -- check contracts/generated
python3 tools/verification/contracts.py
cargo build -p mo-cli --release --locked
```

Python 合同验证使用 `jsonschema` 4.26.0；无网络解析 Schema 引用。首次安装开发依赖需要网络，完整缓存后可追加 `--offline` 执行 Cargo 命令。

TS 合同生成和检查使用根目录 pnpm 锁文件；工具只作开发依赖。生成链是 Rust 类型 → Schema → TS，不维护平行的手写模型：

```sh
pnpm install --frozen-lockfile --ignore-scripts
pnpm generate:types
pnpm check:types
```

CLI 从 stdin 读取一份 `KernelRequest`，stdout 返回一份 `KernelResponse`。例如把 `fixtures/presentations/basic-shape.json` 包在 `{"operation":"initialize","document":...}` 中传入。`prepare` 只计算候选；不读写任意文件、不修改用户文稿、不建立产品提交。

`mo-cli opc-inspect <package>` 是显式路径的开发宿主入口；它打开普通文件并把范围读取器交给核心，输出 `PackageInspectionResponse`。核心没有路径/网络能力。成功只表示当前 OPC/XML/关系检查通过，不表示 PPTX 语义、签名有效性或兼容验收通过。输入文件应由宿主保持不可变；生产接入须使用受权且封存的资源句柄。

## WASM 与跨端比较

```sh
cargo install wasm-bindgen-cli --version 0.2.126 --locked --root .codex-work/toolchain
cargo build -p mo-wasm --target wasm32-unknown-unknown --release --locked
node tools/contracts/build-wasm-node.mjs
node tools/verification/native-wasm-parity.mjs target/release/mo-cli .codex-work/wasm-node/mo_wasm.js > .codex-work/foundation-parity.json
python3 tools/verification/contracts.py --parity-report .codex-work/foundation-parity.json
```

脚本运行真实原生进程及 WASM，比较响应字节和预期状态；没有用另写的 JS 模型模拟内核。Node 是测试宿主，浏览器 Worker、取消和 UI 尚待实现。这里只验证文档计算，不能据此宣称渲染一致、Office/WPS 互操作、性能预算或 Musterwork 替换已通过。

## OPC 实际二进制与独立验证

在上述原生/WASM 构建后执行：

```sh
cargo run -p mo-opc --example package_fixture --locked -- .codex-work/opc-fixtures
python3 tools/verification/opc-fixtures.py .codex-work/opc-fixtures
node tools/verification/opc-parity.mjs target/release/mo-cli .codex-work/wasm-node/mo_wasm.js .codex-work/opc-fixtures/manifest.json > .codex-work/opc-parity.json
python3 tools/verification/contracts.py --parity-report .codex-work/foundation-parity.json --opc-report .codex-work/opc-parity.json
```

Rust 生成和重写真实 OPC 文件；Python 用独立 ZIP/XML 实现检查输出、CRC、摘要及未知内容保留，并生成包含错误输入的二进制语料。21 个用例由 CLI 与 WASM `inspect_package(Uint8Array)` 读取并比较。这组 OPC 测试产物存放于忽略目录，不是可交付 PPTX。通用 XML 覆盖不是公开 Agent 操作；PPTX 来源文本修改已有 Native/WASM 开发入口，见后文。

## 实现分工

| 路径 | 职责 |
| --- | --- |
| `crates/mo-common` | 身份、数值、规范摘要、严格 JSON |
| `crates/mo-unicode` | 固定 Unicode 属性、扩展字簇、显式段落双向分析与 scalar/UTF-8/UTF-16 坐标；生成数据与许可可追溯 |
| `crates/mo-font` | 显式字体字节身份、SFNT/TTC 与元数据/cmap 检查 |
| `crates/mo-text` | 字体绑定、资源复用、类型化批次、自动文字分段/段落准备、原子片段候选选择、范围与轴校验、字形结果核对和诊断 |
| `crates/mo-presentation-compile` | 作者/真实 PPTX 到页面坐标与纯色路径页面；解析几何、统一误差预算、来源对象；文字/图片等继续接入 |
| `crates/mo-render` | Draw IR 共享路径/变换树、有界误差核对、CPU 编译及原始链重算 |
| `crates/mo-raster` | 精确 Q32 路径编译、设备坐标误差预算、像素核对和取消 |
| `crates/mo-skia-sys` | 固定 Skia 的私有 FFI、内存所有权和永久失效 |
| `tools/mo-raster-worker` | 独立原生路径绘制进程，显式 JSON/二进制帧 |
| `crates/mo-harfbuzz-sys` | 原生组件私有 FFI、所有权与失效实例边界，仅在工作进程使用 |
| `components/harfbuzz` | 固定 C++ 塑形组件及私有 ABI；Native/WASM 共用 |
| `tools/mo-text-worker` | 独立原生塑形工作进程，显式帧输入/输出，无输入文件路径 |
| `packages/text-component` | 独立 C++ WASM 实例的 TS 薄层，复制、调用、I/O 拒绝及失效回收 |
| `crates/mo-color` | 保留工作色彩空间的纯颜色数学、变换与显示采样 |
| `crates/mo-presentation-model` | 演示领域记录及语义校验 |
| `crates/mo-presentation-edit` | 版本检查、私有工作版本、原子候选、变更集与收据 |
| `crates/mo-xml` | 有预算的 XML 事件、MCE 逻辑投影、编码和语法感知的来源文本覆盖 |
| `crates/mo-opc` | ZIP/OPC 包图、关系、流式读写和部件保留 |
| `crates/mo-presentation-source` | 来源声明、对象/文本与占位符索引、继承解析、来源绑定与原生时间读取 |
| `crates/mo-pptx` | 原生 PresentationML 写出；兼容转导既有来源读取 API |
| `crates/mo-kernel-api` | 共用计算入口与稳定错误响应 |
| `crates/mo-wasm` | WASM 薄绑定 |
| `tools/mo-cli` | stdin/stdout 计算、显式文件检查、暂存校验后发布 PPTX 的开发宿主 |
| `tools/mo-contract-codegen` | Rust 类型生成 Schema |
| `tools/verification` | 运行合同与原生/WASM 比较 |

当前事务使用隔离的整文档工作副本和保守布局失效，优先确保原子性。这不是已完成的增量实现；持久结构、依赖索引、资源/布局缓存与大文稿测量仍需按 P01/P02/P04 实现。Schema derive 目前随类型编译，发行前需按实际 link/wasm 产物测量并隔离生成工具，不把开发依赖计为已削减。

## 基础原生 PPTX 导出

完成上述 Native/WASM 构建后，使用仓库拥有的合成输入生成真实 PPTX。输出目标必须不存在，CLI 拒绝覆盖已有文件：

```sh
mkdir -p .codex-work/pptx-output
target/release/mo-cli pptx-export fixtures/presentations/native-export/request.json fixtures/presentations/native-export/resources.bin .codex-work/pptx-output/native.pptx
node tools/verification/pptx-parity.mjs target/release/mo-cli .codex-work/wasm-node/mo_wasm.js > .codex-work/pptx-parity.json
```

请求遵循 `pptx-export-request.schema.json`。资源通过单独的二进制 bundle 和范围绑定提供。WASM 对应 `export_pptx(requestJson, Uint8Array)`，返回 `Uint8Array` 或错误；没有在 JSON 中编码文件。CLI 对实际暂存文件执行 OPC 重读，核对包摘要/部件/关系后，在同目录发布新名字；这不是 Musterwork Artifact/CAS 提交实现。

独立检查需要本轮已使用的 `python-pptx 1.0.2`、`lxml 6.0.2`。官方 XSD 首次取回需要网络，后续使用经 SHA-256 校验的本地缓存：

```sh
python3 tools/verification/fetch-ecma-schemas.py .codex-work/ecma376
python3 tools/verification/pptx-independent.py .codex-work/pptx-output/native.pptx --request fixtures/presentations/native-export/request.json --xsd-directory .codex-work/ecma376/xsd
```

脚本只验证当前合成子集。独立读取覆盖类型、层级/顺序、坐标/旋转、路径、显式填充透明度、文字方向、文本、图片摘要、连接引用；官方 XSD 覆盖输出中的 PML/DML 部件。完整 OOXML 合规、导入后的来源保留、布局/字体效果和目标应用交互必须另验。`--external-roundtrip` 用于检查外部应用重存结果，逐项记录几何表示差异、不套用任意容差；其 XSD 路径尚未串接 MCE 预处理，仍跳过外部 XML 的 XSD 验证，也不做世界坐标等价判定。报告的 `result` 必须检查，不能把进程完成当成往返通过。

`pptx-parity.mjs` 在报告中返回 `artifactDirectory`，包含每个成功/失败用例的实际输入以及成功 PPTX，可逐个运行独立验证。`pptx-fixture.py <directory>` 可重建初始请求/棋盘格图片，产物不含第三方模板或字体。当前导出限制及后续路径见 [PPTX 导出实现](pptx-export.md)。

来源读取/编辑入口、测试语料生成和独立保留校验见[来源编辑说明](pptx-source.md)。`pptx-inspect` 使用范围读取器，`pptx-edit-text` 接收独立源包和修改请求，并复用 CLI 的暂存/检查/无覆盖发布。它们仍是开发宿主，不代表已完成的 Agent OperationService 或 Musterwork 提交协议。

占位符、父关系、主题部件栈和位置/尺寸来源的测试入口见[来源继承](source-inheritance.md)。独立库只用于验证有效几何；MCE 投影副本只在验证进程内存中使用，实际编辑输出仍保留原始 XML。

主题声明、物理来源序号、颜色/字体/格式分项覆盖与独立 XSD 核对入口见[来源主题](source-themes.md)。主题来源读取不依赖本机字体，也不代表已经计算渲染颜色或效果。

颜色角色映射、来源继承及外部纯色观察器见[颜色映射](source-color-mapping.md)。外部应用/PDF 工具仅用于开发验证，不是内核的渲染兜底或发行依赖。

字体真实字节的 Native/WASM 检查、固定语料获取、原生 `font-inspect` 和独立 FontTools 比较见[字体资源实现](font-resources.md)。字体覆盖查询不代替塑形、布局或 Office/WPS 验收。

HarfBuzz C++/WASM 构建、无注入 profile、分配失败与 ASan/UBSan 命令见[塑形组件实现](harfbuzz-component.md)。类型化 Rust 入口、原生隔离进程、WASM 薄层、`shape-text` 和端到端验证见[文本塑形](text-shaping.md)。Cargo 原生 worker 构建需要预先准备固定组件；主 CLI 本身不链接 C++，运行塑形时需同目录的 `mo-text-worker`。生产池、浏览器 Worker 与完整资源策略仍待落实。

固定 Unicode 18 数据、完整官方字簇测试、`text-analyze` 和跨端坐标验证见[Unicode 文本边界](unicode-text.md)。运行库使用随源码保存的生成数据，不要求在构建或运行时联网；只有开发再生成和标准验证需要已核对的原始缓存。

`VerifiedFont` 复用、有序候选字体、`shape-cascade` bundle 与独立测量入口见[字体资源复用与候选选择](font-cascade.md)。CLI 仍使用隔离 worker；该入口要求调用方先明确 script/direction item，不是自动段落布局。

`bidi-analyze` 的显式段落、L1/L2 行分析、完整官方测试及独立属性核对见[双向文字分析](bidi.md)。数据版本与算法组件版本分别固定，L2 索引不能用来重排塑形输入；作者文本始终保持逻辑次序。

`itemize-paragraph` 和 `shape-paragraph` 已将固定脚本属性、字簇、bidi 与显式样式连接至真实字体候选/塑形。数据再生成、独立全部码点核对、自动字形验证及当前边界见[自动文字分段](text-itemization.md)；它尚不提供行宽拟合或页面布局。

`shape-paragraph` 已进一步支持片段内混合字体与边界重塑，响应使用 `fallback` 和 v2 profile。实际故障测试、重复报告的合同验证及保持相同完整输出的批量上传性能比较见[混合字体回退](font-fallback.md)。独立 `shape-cascade` 仍维持原子片段合同。

`line-break-analyze` / WASM `analyze_line_breaks` 已提供固定 Unicode 18 默认断行机会和属性查询。完整官方语料、密集属性核对、预算边界及默认规则与实际排版的区别见[Unicode 断行](line-breaking.md)。它不依赖字体或塑形组件，也不选择最终行宽。

`font-metrics` / WASM `measure_font` 通过同一隔离组件取得可变字体实例度量，采用类型化批次、显式缺失值及与塑形共享的轴/scale/失败状态。原创 MVAR/avar 语料、独立参考与故障验证见[字体实例度量](font-instance-metrics.md)。

`shape-lines` / WASM `shape_lines` 按显式行计划执行行级 bidi 和上下文重塑，保留逻辑全文及原坐标。使用相同字体资源检查与隔离组件，独立对照与故障验证见[行上下文与重塑](line-shaping.md)。

`layout-lines` / WASM `layout_lines` 在显式行计划上计算字号、基线和字形坐标；输入使用显式字体与整数 EMU。实际跨端、分数参考与故障验证见[显式行几何](line-geometry.md)。

`layout-paragraph` / WASM `layout_paragraph` 按显式框宽连接断行、候选重塑和实际几何，保留作者文本。支持范围、完整枚举参考与资源边界见[自动段落布局](paragraph-layout.md)。

`font-outlines` / WASM `outline_font` 提取绑定字体实例的批量单色曲线，显式提供 glyph ID 和预算。它不等于彩色绘制或 ink bounds，验证和故障注入入口见[字体轮廓实现](font-outlines.md)。

`paragraph-paths` / WASM `paragraph_paths` 在同一布局内取得未舍入的原点与共享单色路径，提供有界误差的曲线外边界。验证、诊断 SVG 与实际绘制的区别见[段落路径](paragraph-paths.md)。

独立 Skia CPU 组件已能把实际路径批次绘制为 RGBA 像素，提供 C ABI、Native 探针和 TS/WASM 实例边界。固定构建、真实 OOM/abort、跨端像素与许可范围见[Skia 组件](skia-component.md)。后续 `render-paths` / WASM `render_paths` 已完成 Rust 计算、隔离 worker 与有界二进制输出，见[路径渲染](path-raster.md)。生产进程池、浏览器 Worker 与完整 Draw IR 仍需实现。

`render-scene` / WASM `render_scene` 已提供共享路径及仿射节点森林的实际绘制，复用同一隔离 worker、精度检查和原子输出。语料、独立分数参考、许可与局部复用测量见[场景渲染](scene-raster.md)；它仍是已求值表示，不替代完整文稿/OOXML 到页面编译。

作者页面坐标入口 `mo-cli page-placements <request.json>` 和 WASM `page_placements` 见[实现说明](page-placement.md)。输出已经位于页面空间，必须携带作者误差预算进入后续绘制。

作者页面开发入口 `mo-cli compile-page` / `mo-cli render-page` 和 WASM `compile_page` / `render_page` 见[页面编译与绘制](page-render.md)。它们共享 Rust 编译与绘制链，失败不发布部分页面。

路径和场景的可选 `stroke` 参数见[纯色描边](stroke-raster.md)；当前配套 ABI 3 和独立 `miterClip` 求值连接见[截断尖角](miter-clip.md)。主 Rust、C++ Native/WASM 和 TS 适配器必须按同一源码版本构建，不能混用旧 ABI 1/2。显式作者描边和原生 PPTX 映射已接通，页面 profile v2 与实际外部差异见[作者描边](stroke-author.md)；完整继承和尖角保真仍在继续实现。

真实 PPTX 的对象/主题线条与 `lnRef`、作者缺省尖角限值、独立 XML/XSD 和编辑保留验证见[来源线条](source-lines.md)。来源声明与有效继承值分离；`contracts.py --source-line-report` 校验真实文件读取和编辑候选响应。

真实 PPTX 的逐属性线条继承、显式 draft 默认值、Native/WASM 查询与独立来源校验见[线条继承](line-styles.md)；颜色数值、来源页面绘制与目标应用保真仍需继续验收。

原生线条的颜色组合、工作精度与依赖诊断、真实 Native/WASM 查询和 Decimal 验证见[线条颜色](line-colors.md)；完整画笔、来源页面编译与目标应用保真继续推进。

原生预设/自定义形状的来源记录、物理绑定、真实文件编辑保真和标准语料核对见[来源几何](source-geometry.md)；公式求值、路径编译与来源页面绘制继续推进。

真实 PPTX 的导引公式、路径坐标、手柄/连接点求值和独立高精度核对见[几何求值](geometry-evaluation.md)；预设展开、路径编译与完整页面绘制仍在推进。

原生路径空间缩放、椭圆弧编译、逐段来源和误差界及真实 Native/WASM 验证见[原生路径](native-paths.md)；完整来源页面绘制、预设展开与目标应用验收继续推进。

对象、背景、主题和图片的效果读取与平坦来源目录见[来源效果](source-effects.md)。复现新增跨端与独立结构检查：

```sh
python3 tools/verification/source-effect-fixtures.py
node tools/verification/source-effect-parity.mjs
python3 tools/verification/source-effect-independent.py
python3 tools/verification/contracts.py --source-effect-report .codex-work/source-effects/parity.json --fill-style-report .codex-work/source-effects/fill-parity.json
```

语料生成使用已核对摘要的原创基础 PPTX；独立检查使用既有固定 ECMA XSD 缓存。历史证据只绑定对应阶段的源码和产物；`source-effect-evidence.py --seal` 还要求准备前一阶段报告快照、完整回归及构建日志，不能在改动产物后把旧证据当作当前验收。

纯色、渐变停靠点与图案颜色的工作精度计算、按需原生占位上下文，以及 Native/WASM/Decimal 验证见[填充颜色](fill-colors.md)。这一步尚未完成空间画笔或来源页面渲染。

线性/圆形径向渐变的实际路径与场景绘制、统一浮点 profile、像素验证和局部性能基线见[渐变画笔](gradient-raster.md)。后续原生线性渐变已接入来源页面，见[独立渐变平面](native-linear-gradients.md)；路径渐变等仍在推进。

187 种原生预设的真实 PPTX 展开、调整参数、来源合同和跨端路径/绘制验证见[预设展开](preset-expansion.md)。完整来源页面与 Musterwork 接入继续推进。

## 来源对象坐标

CLI `pptx-placements <query.json> <source.pptx>` 与 WASM `place_pptx_objects` 共享 Rust 范围读取/计算。请求绑定摘要、表面与原生对象 ID；配置和误差使用方式见[来源页面坐标](source-placement.md)。这仍是开发入口，并非完整来源页面或公开 Agent 服务。

在既有预设验证语料和构建完成后运行：

```sh
python3 tools/verification/source-placement-fixtures.py
node tools/verification/source-placement-parity.mjs
python3 tools/verification/source-placement-reference.py
python3 tools/verification/source-placement-source-check.py
python3 tools/verification/contracts.py --source-placement-report .codex-work/source-placement/parity.json
```

源 XML 检查需要既有本地 ECMA Transitional XSD。目标应用观察是独立人工/驱动步骤，不能由上述数学和跨端测试代替。

真实源包的 `compile-pptx-page` / `render-pptx-page` 和 WASM 同源入口、二进制 worker 帧、整页失败/原子发布及重现步骤见[来源页面编译](source-page.md)。输出 RGBA，开发 CLI 拒绝覆盖已有文件；这不是已完成的 Musterwork Artifact 提交。

## 来源文本声明

已有 `pptx-inspect` / `inspect_pptx` 响应增加文本声明目录，范围见[来源文本](source-text.md)。在已有来源页面语料、固定 ECMA XSD 缓存及 Native/WASM 构建完成后运行：

```sh
python3 tools/verification/source-text-fixtures.py
node tools/verification/source-text-parity.mjs
python3 tools/verification/source-text-reference.py
python3 tools/verification/source-text-enums.py
python3 tools/verification/contracts.py --source-text-report .codex-work/source-text/parity.json
node tools/contracts/split-types.test.mjs
pnpm check:types
```

`source-text-regressions.py` 重跑既有主内核报告；执行前先按前一阶段冻结记录保存报告及其绑定 JSON，随后 `source-text-regression-audit.py` 严格核对新增目录之外的旧结果。`source-text-evidence.py --seal` 要求完整构建/验证日志、历史快照和当前文件摘要一致，并拒绝覆盖已有冻结记录；它不是单独运行就能完成的完整产品验收。

真实 PPTX 的文本框逐属性继承、主题文本默认项、autofit 来源和同源查询入口见[文本框继承](text-body-styles.md)。后续段落/run 的库级实现、绝对路径证据导出和独立 XML 校验见[文字样式计算](text-cascade.md)。主题字体名称、显式清单及真实 HarfBuzz 开发探针见[原生字体绑定](native-fonts.md)，跨段落复用和布局/轮廓接线见[清单布局](manifest-layout.md)，实际 PPTX 到局部字形路径及原生探针见[来源字形编译](source-glyphs.md)，精确行宽、原生间距/对齐和整框失败处理见[原生文本框布局](source-frame.md)；来源页面纯色文字现已接入[双端运行接口](text-page-runtime.md)。

新库入口的整页文本预检、原生纯色画笔、实际绘制、独立像素核验及旧 Native 页面回归见[原生文字到页面像素](source-text-page.md)。旧无字体资源的来源页面入口仍保持明确拒绝文字。

文字页面运行入口 `mo-cli render-pptx-text-page` / WASM `render_pptx_text_page`、显式字体清单与字节、三段 worker 帧及新 Schema/TS 见[来源文字页面运行接口](text-page-runtime.md)。这是开发入口，尚未实现 Musterwork 任务和 Artifact 接入。

字体选择缺失的类型化资源要求、原生 run 位置及完整 Native/WASM 重试探针见[文字资源诊断](text-resource-diagnostics.md)；验证产物使用独立的 `.codex-work/text-resource-diagnostics`，不覆盖前一阶段报告。

单下划线和单删除线的真实绘制、可变字体度量及独立像素参考见[原生文字装饰](text-decorations.md)，产物使用 `.codex-work/text-decorations`；不要将开发态成功样本等同于 Office/WPS 视觉验收。

后续独立下划线填充使用 `.codex-work/underline-paint`，见[实现与复现说明](underline-paint.md)。保留此前证据，不覆盖其输入和输出；本阶段没有新增运行依赖或修改公开运行合同。

原生基线偏移使用 `.codex-work/native-baseline`，复现见[基线偏移](native-baseline.md)。共享布局接口支持原有整数 EMU 和新增 Q32 对象，修改后需重新生成并检查 3 份受影响的 Schema/TS 请求合同。

段落百分比间距使用 `.codex-work/paragraph-spacing`，复现见[段落间距](paragraph-spacing.md)。共享行距新增按样式选择高度的合同；同样检查 3 份 Schema/TS，并使用当前产物重放前期页面、基线几何及其他文本请求，不能只复用旧二进制结果。

原生字符间距使用 `.codex-work/character-spacing`，复现见[字符间距](character-spacing.md)。新增原创 GSUB 字体用于验证可选/必需连字；零间距仍使用同一笔位计算，需要重放旧布局及装饰线语料。

图片绘制新增 `mo-raster-worker --images` / `render_image_paths` 开发入口，完整协议与固定组件重现见[图片绘制](image-raster.md)。本轮组件位于 `.codex-work/image-brush/component/`；Rust 构建应将 `MO_SKIA_LIB_DIR` 设为此目录的绝对路径，避免误用旧组件记录。

来源图片增加 `pptx-images` / `pptx-extract-images` CLI 与同源 WASM 接口，明确区分嵌入快照、外链请求和未解析目标。资源绑定、原子新文件输出、预算和独立 ZIP/XML 验证见[来源图片](source-images.md)；该入口提取编码字节；解码现由上述独立接口提供，页面画笔编译继续推进。

共享场景增加 `mo-raster-worker --image-scene` / `render_image_scene`，将显式图片资源接入原有路径变换与精度预算；协议、资源生命周期与独立参考见[场景图片](image-scene.md)。

共享路径/场景裁剪使用 `.codex-work/clips`，见[实现与复现](shared-clips.md)。Native 构建选择此目录的固定组件，TS 先查询 `supportsClips` 再调用 V7；现有 CLI 已支持裁剪结果的像素校验和原子发布。来源页面尚未自动生成这些节点。

图片参数误差阶段复用裁剪组件，Rust 与报告另存 `.codex-work/image-paint`，见[计算边界与复现](image-precision.md)。该阶段不修改图片 ABI，旧绑定可省略 `uncertainty`；新的布局/放置生产者必须显式传递其计算误差，不能把近似值当成精确 Q32 输入。

原生世界图片库级验证使用 `.codex-work/image-world`，见[坐标、裁剪与测试边界](image-world-paint.md)。既有 `source_image_layout` 开发宿主可追加 `--paint`，该阶段的验证场景经既有共享渲染接口执行；后续实际页面调用见[来源资源页面](source-resource-page.md)。


### 来源资源页面

`mo-cli render-pptx-resource-page <request.json> <source.pptx> <fonts.bin> <new-output.rgba>` 和 WASM `render_pptx_resource_page` 从实际 PPTX 完成图片关系、解码、原生布局、形状/文字顺序、裁剪和绘制。请求显式声明 `imageSource`、`sampling` 和字体清单；没有文字时使用 `fonts: null` 与空字体文件。共用预检、一次解码、误差/内存口径及未完成内容见[实现说明](source-resource-page.md)。该入口仍是开发合同，尚无 Musterwork 任务/Artifact 适配或可安装的 Agent 插件。

后续共享合成使用 `.codex-work/compositing/component`，构建时以 `MO_SKIA_LIB_DIR` 显式选择；旧组件不支持 V8。TS 通过 `supportsCompositing` 识别能力。捕获预算、背景恢复开发策略、应用差异及复现脚本见[背景合成](background-compositing.md)。此前阶段目录保留原始产物，不用新源码覆盖旧证据。

当前新增 V9 渐变平面的构建与测试使用 `.codex-work/gradient-field/component`，以 `MO_SKIA_LIB_DIR` 显式选择。TS `supportsGradientPlanes` 查询可选能力，旧组件不得处理新 V9 帧。实际来源线性渐变、协议布局、应用差异及复现见[原生线性渐变](native-linear-gradients.md)。此前阶段的目录与证据保持封存。

后续[画笔共享存储与预算](paint-storage-admission.md)复用上述固定组件，新输出位于 `.codex-work/paint-admission`。`GradientStops` 的内存共享保持既有 JSON/Schema；Rust 构造使用 `Vec::into()`，需要修改已求值色标时显式调用 `make_mut()`。局部存储测量及其平台、缓存与统计范围见实现记录。
