# 共享场景的图片资源

状态：Rust、Native Worker、实际 WASM 已实现并经过开发验证，DRAFT。场景现在可以在同一绘制批次中组合图片、纯色和渐变画笔，复用已有路径、变换图、描边及合成顺序。这一步连接[图片画笔](image-raster.md)和[共享场景](scene-raster.md)，不表示编码图片解码或真实 PPTX 图片页面已经完成。

## 共享编译和资源生命周期

`mo-render::compile_images` 接受既有 `SceneRasterRequest` 及 `PreparedImages`。路径仍位于局部坐标，实例引用变换节点；图片画笔已经处于世界坐标。场景编译只对画笔减去一次视口原点，不再次应用路径变换。原生裁剪、平铺、拉伸及是否跟随对象旋转的语义，应由来源页面编译器先求成世界画笔。

纯色场景和带图片场景共用同一个非泛型几何编译函数，避免每增加一种资源绑定就要求编译器复制整套变换算法。只有末端资源绑定和结果类型不同。路径按原有线性变换规则复用，图片画笔独立去重；同一图片可以用于多个对象，不为每次绘制复制源像素。

变换图顺序、深度、资源数、绘制命令和点变换工作预算沿用原实现。矩阵优化候选必须通过原始变换链的误差检查；范围或精度不满足时，仍使用原始节点链重新编译，后端只在整个场景通过后调用一次。失败不发布部分像素，也不丢弃失败对象后继续绘制。

`CompiledImageScene` 持有私有绘制批次并借用已验证图片资源。`render_compiled_images` 直接使用既有图片执行器，保留取消、后端失效、输出长度/预乘通道和摘要检查。Rust 宿主可在多次编译/绘制间复用 `PreparedImages`；JSON 入口仍按调用准备资源，尚无生产驻留缓存。

路径的变换与设备坐标误差保存在 `scene.work`，独立世界图片画笔的系数量化误差保存在 `images`。两者分别受到视口容差约束；这些数值不表示滤波、抗锯齿或最终颜色误差。输入资源仍是已归一化的 sRGB RGBA8，图片方向、ICC、动画帧和解码由后续资源层处理。

## 接口

- Rust：`mo-render::{compile_images, render_images, render_compiled_images}`。
- 开发 Worker：`mo-raster-worker --image-scene`。输入为 `u32le JSON长度 + u32le 资源长度 + JSON + 资源`，输出为 `u32le 元数据长度 + u32le 像素长度 + 元数据 + 像素`。
- WASM：`render_image_scene(request, images, rasterComponent)`，使用同一个 TS `RasterComponent.rasterImages` 和固定 Skia 模块。
- [请求](../../contracts/generated/image-scene-request.schema.json)为 `{raster: SceneRasterRequest, images: ImageResource[]}`；[响应](../../contracts/generated/image-scene-response.schema.json)包含场景、图片资源工作量和完整资源摘要，像素独立传递。两份合同及 TS 由 Rust 生成。

原有无资源参数的场景接口仍明确拒绝图片画笔。没有引入第二个绘制器、浏览器 DOM 渲染或图片截图导出链路。新增入口尚未成为可安装 MCP/Skill/Plugin 产品包。

## 验证

[阶段记录](../reviews/evidence/2026-09-25-image-scene-verification.json)绑定源码、当前构建和执行完成标记。

6 个新增 Rust 测试覆盖路径/画笔复用、源像素借用、极大世界原点只消除一次、原始链精度恢复、无效图/资源预检、每个取消检查点及后端坏结果。全工作区 521 项 Rust 测试、严格 Clippy、格式和 76 份 Schema/TS 检查通过。

运行验证覆盖 124 个成功场景、16 个资源/几何/预算拒绝、7 个 JSON 拒绝，并额外执行一个等价世界路径参考，共 148 对新增 Native/WASM 调用。成功场景包括四类多层几何变换、独立图片仿射/平铺、两类 Alpha、两类采样、资源/实例复用、透明渐变以及有顺序的纯色/图片混合。WASM 另验证一次坏宿主结果的实例失效和空输出。

Python 使用精确有理数逐点展开原始场景变换，核对 560 个世界坐标顶点；再从资源像素独立反演图片矩阵、采样和 SrcOver，比较 31,744 个像素。每通道允许 1 个 RGBA8 舍入单位，不排除边缘像素。这些场景变换后的覆盖区域为整数边界矩形，**不作为新增曲线抗锯齿或图片描边的独立视觉验收**。新增有色混合场景同时与显式世界路径入口比较。

既有 568 对调用全部回放：307 个来源页面/文本、211 个文字页面/文本几何/图片绘制及拒绝、50 对来源图片查询/提取。已保存的元数据与成功像素不变。固定 Skia/HarfBuzz 组件及此前发行产物保持不变，未新增产品性能、RSS、Office/WPS 互操作或安装包结论。

## 重现

按[开发说明](development.md)准备固定组件，Native 构建继续使用 `MO_SKIA_LIB_DIR="$PWD/.codex-work/image-brush/component"`。将当前 Rust WASM 的 Node bindgen 输出写到 `.codex-work/image-scene/wasm-node/`，保留此前构建文件。完成原生 Worker、Schema/TS 与工作区检查后执行：

```sh
python3 tools/verification/image-scene-fixtures.py
node tools/verification/image-scene-parity.mjs
python3 tools/verification/image-scene-reference.py
node tools/verification/text-page-runtime-regressions.mjs .codex-work/image-scene .codex-work/image-brush/component .codex-work/image-brush/ts-raster/index.js
node tools/verification/source-images-regressions.mjs .codex-work/image-scene
node tools/verification/source-images-parity.mjs .codex-work/image-scene source-resources-new .codex-work/source-images/cases source-resources
```

资源回放的输出目录须为新的。全部调用完成后再核对证据，不覆盖历史记录。
