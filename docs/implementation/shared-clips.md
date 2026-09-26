# 共享路径裁剪

状态：开发实现，接入已有路径/场景的 Native 与 WASM 运行接口。它为图片填充区域、文本框和其他内容提供统一几何机制；来源页面尚未自动生成这些裁剪节点，不能据此宣布完整 PPT 页面或 Musterwork 替换验收通过。

## 数据模型与职责

`mo-render::DrawScene.clips` 包含有序 `ClipNode { parent, path, transform }`；每个 `PathInstance.clip` 指向最后一层裁剪。节点与父节点的区域求交；根节点与 viewport 求交。裁剪使用独立的几何变换，子节点变换不会再次变换父裁剪，内容变换也不会移动裁剪。空路径产生空区域；开放轮廓按填充规则隐式闭合，支持 nonzero/evenodd。

所有引用和父顺序均校验，父必须先于子。场景仅编译实际引用的节点及其祖先，未用节点仍验证路径和变换引用。裁剪与内容共用路径资源缓存、Q32 变换、viewport 重基准、控制点误差界及原始变换链重试。额外裁剪几何计入相同的点变换预算；不能借用另一条没有精度校验的路径。

降低后使用 `mo-raster::PathClip { parent, path, origin }`，其路径和 origin 已位于文档世界坐标。路径原点、device float32 量化、最终控制点加法误差和范围检查与内容共用一处实现。裁剪没有描边宽度；内容的 stroke inflation 独立校验。路径坐标误差上界不代表像素覆盖误差，抗锯齿相交采用固定 Skia CPU 语义。

这些数据不涉及 HTML/DOM、字体查找、I/O、权限或 Agent 推理。图片 `sourceDomain` 决定采样范围与平铺，而路径裁剪决定目标页面上的覆盖区域；二者职责不同，不以 filtered decal 代替填充区域裁剪。

## 预算与复用

| 项目 | 上限 |
| --- | ---: |
| 裁剪节点 | 8,192 |
| 裁剪深度 | 64 |
| 全部输入裁剪的 placement commands | 1,048,576 |
| 连续绘制中的实际裁剪入栈 | 262,144 |
| 实际入栈的累计路径命令 | 1,048,576 |
| DrawScene 每次降低的点变换工作 | 4,194,304，沿用共享预算 |
| V7 帧 | 2,789,389 个 u32 |

几何资源上限与绘制命令上限继续生效。反复引用空路径也受入栈次数限制，未被绘制使用的重复大路径也受 placement 预算限制。Rust 与 C++ 对累计裁剪工作分别验证，C++ 在分配像素或调用 Skia 前完成整个 wire grammar 校验。

后端维护最多 64 层的当前裁剪栈，相邻绘制保留共同祖先，仅恢复离开的分支并压入新分支。入栈采用 `save → translate → clipPath(Intersect, AA) → resetMatrix`；内容绘制仍有自己的 save/restore。切回无裁剪状态、跨兄弟分支及 miterClip 描边的提前返回都保持栈一致。没有为裁剪建立图片副本，也不为每个文字/路径实例重建相同裁剪。

## 组件与合同

普通 ABI 保持 4，image extension 保持 2，decode extension 保持 1。增加可选 `mo_skia_clips_abi() == 1`；TS `supportsClips` 查询缺失或不匹配时，在调用旧组件前拒绝 V7，旧实例继续可用于其支持的请求。Native 链接经过来源核验的新固定组件。

没有裁剪节点的旧输入仍发出 V4/V5/V6，旧工作量字段保持原序，新增 clips 指标省略。Schema/TS 的可选字段由 Rust 同源生成；现有 `render_paths`、`render_scene`、`render_image_paths`、`render_image_scene` 及对应 Worker 入口直接接收，未新增平行渲染引擎。

V7 采用 13-word header：原 V6 的 12 words 后增加 clip count。普通入口要求 image count 和 image brush count 均为零。数据按路径、描边、渐变、图片描述符、14-word image brushes、4-word clips、7-word draws 顺序存储。

- Clip：`[parent + 1 或 0, path, originX f32 bits, originY f32 bits]`。
- Draw：原来的 6 words 后加 `clip + 1 或 0`。
- clip/path/transform 的 JSON 引用为 0-based；wire 的可空 clip 引用使用 1-based，不能混用。
- V7 image brush 始终包含 source domain；未声明 domain 的整图使用原有快速采样路径。
- 新缓存 profile 为 `skia-8d6d37b-shared-path-clips-srgb-premul-rgba8-v7-draft`，旧 profile 不变。

## 验证与复现

验证工具使用自有几何和显式 RGBA 字节。整数矩形/孔洞/嵌套区域由独立逻辑计算遮罩，结合每层未裁剪的画笔结果校验实际像素；这检验裁剪与合成，不宣称独立验证全部渐变或图片采样。另有曲线路径的 Native/WASM 一致性测试。测试包含不同图形/裁剪变换、Q32 大坐标、64 层复用、兄弟分支、空路径、图片区域采样、渐变和 miterClip 描边。

新产物必须使用独立目录，不能覆盖已封存的前期构建与语料。固定工具和 codec 依赖准备方式沿用[图片源区域采样](image-domain.md)。

```sh
python3 tools/components/build-skia.py --target native --directory .codex-work/clips/component --image-codecs .codex-work/image-codec/deps-native
python3 tools/components/build-skia.py --target wasm --directory .codex-work/clips/component --image-codecs .codex-work/image-codec/deps-wasm
python3 tools/components/build-skia.py --target native --sanitize --directory .codex-work/clips/component --image-codecs .codex-work/image-codec/deps-asan
MO_SKIA_LIB_DIR="$PWD/.codex-work/clips/component" cargo test --workspace
MO_SKIA_LIB_DIR="$PWD/.codex-work/clips/component" cargo build --workspace --examples --bins
MO_SKIA_LIB_DIR="$PWD/.codex-work/clips/component" cargo clippy --workspace --all-targets -- -D warnings
cargo build -p mo-wasm --target wasm32-unknown-unknown --release
```

新 Rust WASM 使用固定 wasm-bindgen 写入 `.codex-work/clips/wasm-node`，设置该目录为 CommonJS；TS 适配层以 `tsc --project packages/raster-component/tsconfig.json --outDir .codex-work/clips/ts-raster` 构建。更新/检查 78 份 Schema/TS 后执行：

```sh
node tools/verification/clip-parity.mjs
node tools/verification/clip-components.mjs
node tools/verification/clip-image-regressions.mjs
node tools/verification/text-page-runtime-regressions.mjs .codex-work/clips .codex-work/clips/component .codex-work/clips/ts-raster/index.js
node tools/verification/source-images-regressions.mjs .codex-work/clips .codex-work/clips/component .codex-work/clips/ts-raster/index.js
```

ASan/UBSan 检查直接执行 raw V7 帧，覆盖截断、无效父子关系、引用、NaN/Infinity、设备边界、预算与旧组件能力拒绝；不包括 LeakSanitizer。开发基准不代表 Office/WPS 的最终视觉一致性、目标文稿规模下的性能或安装包体积。

实际结果见[封存证据](../reviews/evidence/2026-09-25-shared-clips-verification.json)：552 项 Rust 测试、78 份合同及严格 Clippy 通过；新增 50 个场景、242 对运行调用、36,864 个独立遮罩参考像素（通道最大偏差 0），125 组底层三端/消毒器检查，1,144 对旧请求回归及两种 CLI 原子发布检查通过。C++ WASM 组件为 2,322,200 字节，相比此前增加 2,165 字节，第三方锁定版本未变。上述规模均为这组有界验证语料，不是完整 PPT、安装包或用户文稿性能验收。

## 下一步

将[图片局部布局](image-layout.md)的目标矩形降低为此模型，并把完整 paint basis（包括 `rotWithShape`、组变换、翻转、原点）与来源页面绘制顺序接通。图片、形状及文字保持同一 SceneBuilder，资源预检与整页失败保持原子性。随后仍需完成其余文字/媒体/动画/SmartArt/公式及 Agent/Musterwork 的整体接入和全部验收关卡。
