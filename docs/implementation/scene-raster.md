# 仿射场景与共享路径绘制

后续[渐变画笔](gradient-raster.md)已接入世界空间画笔，场景编译在精确重定位后进入 ABI 4。本节原测量保留为历史证据。
2026-09-24 · `mo-render` 已实现 Draw IR 的共享路径、变换树和有序实例，接入真实 Native/WASM 像素输出。页面、字形和后续原生对象可以使用同一变换语义；作者文稿到完整 Draw IR 的编译、图片/效果和播放仍需继续实现；后续已连接[纯色描边](stroke-raster.md)。本阶段不关闭 E0/E1/E3。

## 表示与边界

[SceneRasterRequest](../../contracts/generated/scene-raster-request.schema.json) 包含原有 viewport 和 `DrawScene`：共享路径表、仿射变换节点表、有序绘制实例。节点的 parent 指向先出现的外层节点，形成可共享父变换的森林；显式校验全部父引用和深度，不递归访问不可信对象树。只合并非空绘制实例需要的节点及祖先；未使用或仅服务空路径的变换不触发数值计算，也不改变绘制策略。实例通过 path/transform 索引引用资源，`transform: null` 表示恒等变换，颜色和次序属于实例。

`Affine.linear` 为 row-major `[xx, xy, yx, yy]`，系数是无量纲 Q32；translation 与路径同为 Q32 EMU。所有数值仍是规范 i128 十进制字符串。它表达**已求值的变换**，不替代文稿中的 rotation、flip、group viewport、占位符或来源字段。渲染器不反向修改作者模型或 PPTX；原生可编辑对象不能被这些派生路径取代。

新入口：CLI `render-scene <request.json> <new-output.rgba>`、WASM `render_scene(requestJson, RasterComponent)`、Native worker `--scene`。输出仍是独立元数据和 RGBA8 二进制通道；WASM 先读取 `.metadata`，再调用 `.take_pixels()`。文件授权、超时、完整退出、暂存读回和无覆盖发布复用[路径渲染宿主](path-raster.md)。[SceneRasterResponse](../../contracts/generated/scene-raster-response.schema.json)额外包含场景工作量和组合精度上界。

计算模块不依赖文档模型、模型推理、文件系统、网络或系统字体。当前是路径/仿射部分的实际实现，已接入[纯色描边](stroke-raster.md)以及部分[作者页面编译](page-render.md)，尚无渐变 brush、image、clip、group opacity、effect、layer/blend、GPU 或完整页面能力，不把未实现指令伪装为成功。

## 精度与资源复用

常规坐标用 checked i128 计算；中间乘加溢出时，使用固定版本 `num-bigint` 计算受限的精确中间值，只在最后缩窄。输入精度固定，单次计算仅包含几个 i128 乘积，不提供任意大数字或表达式执行。矩阵/坐标在 Q32 边界按 nearest-ties-away 舍入，进入 float32 时仍采用[路径编译器](path-raster.md)的 nearest-ties-even。

根变换先在精确表达式中减去 viewport 原点，再缩窄坐标。原点接近 i128 最大/最小值时，不能因为本来应抵消的全局值中间溢出而拒绝可见内容。内部非根空间仍须满足当前 Q32 运算范围，超限会明确失败。

常规编译按 `(source path, world linear matrix)` 缓存变换后的局部路径，平移单独保存；同一字形在不同位置出现时复用资源。合并矩阵是计算优化，不能自行成为正确性的依据：每个使用中的控制点都与**原始节点链**求值比较，传播逐层舍入误差，再和设备量化误差共同核对 viewport 容差。

若合并矩阵的数值范围或精度不满足要求，编译器在调用后端前改用原始节点链逐点计算，并重新检查全部精度/资源预算。它仍使用同一几何语义和 Skia 后端，不删对象、不放宽容差。`loweringAttempts=2` 明确记录该路径。已有反例证明：合并矩阵可能无法表示，或合并后的误差预算失败，而原始链能得到合格结果；两种情况均已验证。原始链仍不满足合同则返回明确失败。

`transformErrorBound` 是 Q32 EMU 单位的几何误差上界；`combinedCoordinateErrorBound` 是 Q32 像素单位，包含几何和最终 float32 控制坐标量化。独立参考直接以 Fraction 执行未舍入的整条变换链，再与实际设备控制点比较，不复用 Rust 的矩阵合并、舍入或区间算法。它证明本语料的控制坐标保证，不是像素覆盖率、完整曲线 ink 或目标应用视觉误差保证。

当前限制：4096 个源路径、262144 源指令、8192 变换节点、64 层深度、65536 实例、1048576 条被引用指令。每次尝试的点/节点工作量最多 4194304，最多两次尝试；生成路径仍受底层 4096/262144 预算约束。预算在后端调用前校验，取消覆盖编译、必要的第二次求值、像素核对和摘要；后置取消使组件实例失效。不满足预算不发布部分图片。生产缓存淘汰、增量更新、分块、GPU 和完整资源账本尚未完成。

## 初始仿射阶段的验证和局部测量

[当前证据](../reviews/evidence/2026-09-24-scene-raster-reachability-verification.json)记录 181 批 Native/WASM 场景请求：165 批成功、16 批拒绝。[首版证据](../reviews/evidence/2026-09-24-scene-raster-verification.json)保留修正前 179 批的状态；这些既有结果在当前产物上保持不变，另有两个用例验证无关变换不会触发第二次计算。覆盖既有路径语料的恒等场景、29 个旋转段落、翻转/非均匀缩放/剪切/透明叠加、32 组八层有理变换、共享资源、两个数值/精度重算反例、i128 原点和资源上限。两端元数据、像素逐字节相同；设备批次再由独立 C++ 入口核对。

Fraction 参考核对 67050 个实例、2249746 个控制坐标和 5259471 次逻辑变换应用。这里包含重复实例和预算用例，不将它们说成同等数量的独立文稿。CLI 无覆盖发布、真实组件堆耗尽、失败无像素、拒绝复用与替换实例恢复也已验证。原始链第二次求值的取消点同样有测试。7 份旋转预览由实际像素编码，已查看中文和原创三次曲线；不是 Office/WPS 对照。

旧 1999 批在新产物上回归，语义响应不变；主内核累计 2180 批。Rust 225 项测试、Clippy、44 份 Schema 和 TS 检查通过，42 份既有 Schema 不变。Skia/HarfBuzz 组件与薄层没有改变，独立组件历史证据只在核对相同产物摘要后复用，不声称本阶段重新执行了那些测试。

局部 WASM 测量使用 Apple M4 Max、arm64、macOS Darwin 25.2.0、36 GiB 内存、Node 23.5.0。输入为已有 Latin 语料中的一个实际字形轮廓，1000 个带位移的实例，共享一次相同 shear；1024×768 RGBA8，显式 scale=1/9525、相同颜色/抗锯齿和像素结果。每种情况预热 3 次、交替顺序测量 12 次，无强制 GC：

| 表示 | 请求字节数 | 编译路径数 | 中位耗时 |
| --- | ---: | ---: | ---: |
| 1000 份重复资源 | 3616498 | 1000 | 73.13 ms |
| 共享路径、独立变换节点 | 186040 | 1 | 31.46 ms |

计时包含 JSON 解析、仿射计算/精度核对、CPU 绘制、响应序列化和像素复制；不包含字体加载/排版、启动、文件发布、峰值 RSS 或完整页面。原始样本及字体/构建摘要保留在证据中。此观察不代表完整内核性能门禁、冷缓存或 Musterwork 性能收益，也不把现有重复表示当作其他产品的实现。

当前未压缩产物：CLI 3965840 字节、文字 worker 2375200 字节、绘制 worker 2839920 字节、Rust WASM 3661022 字节（另有 24735 字节 glue）。相较前一阶段，绘制 worker 增加 115104 字节，Rust WASM 增加 109547 字节。两个 C++ WASM 和薄层保持相同字节。全部仍是未完整的开发产物，不包含全部字体、图片/媒体/宿主和安装格式。

## 构建与后续

新增 `mo-render` 和 `num-bigint 0.4.8` / `num-integer 0.1.47`；`num-traits 0.2.19` 原已在开发依赖图中，现在也属于数值运行闭包。默认特性关闭，包归档摘要与 MIT/Apache 原文保存在[数值组件记录](../../components/rust-numeric/component.json)。workspace 的 num-traits 额外出现兼容 `i128` feature，实际 release feature 树另存证据；不把开发图等同于运行闭包。未更改 Skia/HarfBuzz 绘制/塑形源码或其构建参数。

```sh
cargo build -p mo-cli -p mo-text-worker -p mo-raster-worker --release --locked --offline
cargo build -p mo-wasm --target wasm32-unknown-unknown --release --locked --offline
node tools/contracts/build-wasm-node.mjs
node tools/verification/scene-raster-parity.mjs
python3 tools/verification/scene-raster-reference.py
python3 tools/verification/contracts.py --scene-raster-report .codex-work/scene-raster/parity.json
python3 tools/verification/skia-previews.py --directory .codex-work/scene-raster --name-prefix rotate-paragraph-
node tools/verification/scene-raster-benchmark.mjs
```

语料依赖前面的[路径渲染验证](path-raster.md)，基准应在无并行构建/测试时单独运行。当前 Native 仍只在 macOS arm64、WASM 只在 Node 验证。下一步由页面编译将作者对象、组 viewport/旋转/翻转和文字结果映射到此场景，并补充画笔、图片、裁剪与效果；完整 Agent 接入、播放、高级可编辑对象和 Musterwork 替换门禁持续推进。
