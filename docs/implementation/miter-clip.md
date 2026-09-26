# 已求值截断尖角与设备 ABI 3

2026-09-24：新增独立 `miterClip` 绘制原语，已连接 Rust 路径/场景请求、Native worker 与 WASM。依据[目标应用调查](miter-compatibility.md)，现有 miter-to-bevel 不能表示渐进截断；本实现补齐缺少的几何能力，不猜测 WPS 的私有算法，也不改变作者/PPTX 的现有映射。沿既定 Rust/TS/精选 C++ 路线推进。

## 明确的几何合同

`StrokeJoin::MiterClip { limit: Fixed }` 的 limit 是无量纲 Q32，比率范围为 `[1,1024]`。宽度沿用页面/世界空间的已求值笔宽，经 viewport 比例转换为设备宽度。设半宽为 r，连接处两段外侧切线形成的完整尖角离连接点距离为 d：

- d ≤ r × limit：保留完整尖角。
- d > r × limit：在外侧角平分线方向距离 r × limit 处，用垂直于角平分线的直线截断外侧连接区域。
- 精确折返没有唯一角平分线，明确采用斜角；接近折返仍计算截断，不设角度 epsilon 将其偷偷转为斜角。
- 线宽 0 明确表示宽为一个设备像素的实际轮廓。正宽度不提升为一个像素；旧三类连接的 Skia hairline 语义保持原样。

这个基本截断定义参考 [SVG 2 连接几何](https://www.w3.org/TR/SVG2/painting.html#StrokeLinejoinProperty)，但受限参数域、精确折返和设备细线是本 profile 的明确约定，不宣称完整 SVG 实现或 WPS 等价。作者 miter 的百分比定义与求值连接类型是不同层次；当前作者 miter 仍使用原来的 miter-to-bevel，导出的 `a:miter/@lim` 不变。

连接点支持直线、二次/三次曲线及闭合末端。曲线的偏移、细分、端点、内部轮廓和 winding 继续由 Skia 描边器负责；新回调只负责真实段落连接处的尖角几何。单一描边轮廓一次绘制，透明笔画不会因为另外叠加一个尖角而重复混色。

截断的两个角还有垂直于角平分线的侧向距离。范围检查使用保守扩张 `r × (limit + 1)`，同时覆盖方形端点；不能只以 `r × limit` 作为所有角的边界。0 宽度在该检查中取实际的半像素半宽。参数 float32 量化误差仍独立报告，不将其称为 ink 或抗锯齿误差上界。

## 组件与协议边界

固定 Skia 提交、CPU 裁剪和许可不变。新增[私有描边接口扩展](../../components/skia/custom-stroke-join.patch)只影响 `src/core/SkStroke.h/.cpp`：给内部描边器传入显式连接回调，custom 路径不走矩形专用连接算法。原有入口仍选择原有 JoinFactory；没有扩充公共 `SkPaint::Join`，也不影响 Skia 其他后端、序列化或画笔位字段。

[几何实现](../../components/skia/mo_miter_clip.cpp)属于组件职责；不在宿主或作者页面中加补偿图形。每次构建从核对的上游归档提取源码，以零 fuzz 应用扩展；Native、WASM 和 sanitizer 采用相同来源。编译闭包重验为 Native 497、WASM 496 个上游编译单元，唯一变化的上游绘制单元是 `SkStroke.cpp`。最终动态依赖仍只有 libc++/libSystem；WASM imports 与上轮相同。

设备批次升为 ABI 3，保留头部/路径/画笔/实例表布局。join=3 表示 miterClip，使用现有 miterLimit 槽；round/bevel 仍要求该槽全零。组件、Rust FFI、批次编译器和 TS 适配器同步升级，旧 ABI 1/2 明确拒绝。绘制 profile 为 `skia-8d6d37b-q32-local-paths-srgb-premul-rgba8-v3-draft`。作者页面 profile v2 不变，其输出包含当前绘制 profile。

## 实测范围

[封存证据](../reviews/evidence/2026-09-24-miter-clip-verification.json)绑定源码、构建、产物与实际输入输出：

- 新增 74 组路径请求：65 成功、9 明确拒绝；新增 65 组场景请求。涵盖限值/角度/线宽、两种方向、四分之一圈旋转、短段、闭合矩形、重复顶点、精确/近似折返、曲线、透明度、超大原点抵消和 viewport 比例。
- 全部路径 267 组、场景 316 组、作者页面 131 组的 Native/WASM 元数据和像素逐字节一致。原有 193/251/131 组输入、状态和像素不变；只回映射 ABI、绘制 profile 与相应帧摘要即可重建旧帧和响应/计划摘要。
- 独立多边形相交/半平面裁切参考核对 59 组直线形状的 159024 个内部像素和 1972706 个外部像素。事先排除离多边形边界不足一个像素的位置；另有 3 个精确像素探针区分真实截断与完整尖角/斜角。它不是曲线偏移或 AA 边缘的保真门禁。
- 197 组独立组件输入在 Native、WASM、ASan/UBSan 下通过，最大通道差 0；61445 个既有独立参考像素、真实输出分配失败和 Skia 内部分配 abort 重新验证。原有 174 组组件输出/状态不变。
- 累计 253 项 Rust 测试、2688 批主内核逻辑用例通过。49 份 Schema 中 3 份增加新求值连接类型，46 份保持不变；格式、Clippy、TS 和 C++ 适配器严格警告检查通过。独立组件用例不加进主内核计数。

未新增依赖或修改 Cargo/pnpm 锁文件。实际未压缩开发产物：Rust WASM 3890970 字节（+5317），Skia WASM 1805239 字节（+1457），raster worker 3651392 字节（+17712）。CLI 和文字 worker 字节数分别保持 4365264、2375376。字节数相同不等于二进制摘要相同；全部摘要在证据中。上述不是完整一期体积、Musterwork 安装包估算或性能/RSS 验收。

## 复现及后续

依[开发说明](development.md)重新构建配套组件、Rust 和 TS；先生成既有文字/路径输入，然后执行以下新原语及相关回归：

```sh
python3 tools/verification/skia-fixtures.py
node tools/verification/skia-parity.mjs
node tools/verification/path-raster-parity.mjs
node tools/verification/scene-raster-parity.mjs
node tools/verification/page-render-parity.mjs
python3 tools/verification/path-raster-reference.py
python3 tools/verification/scene-raster-reference.py
python3 tools/verification/page-render-reference.py
python3 tools/verification/miter-clip-reference.py
python3 tools/verification/miter-clip-previews.py
python3 tools/verification/skia-closure.py
python3 tools/verification/miter-clip-evidence.py
```

像素参考使用 NumPy，预览编码使用 Pillow；二者是本地验证工具已有依赖，不进入内核运行闭包。完整封存还要求最新文字/文件等回归、Schema、格式和类型检查日志。`--seal` 只允许新建证据，不覆盖历史证据。

作者 miter 限值省略、完整描边继承、目标应用映射、曲线/边缘保真和 Office/WPS 编辑往返继续实现。本轮没有重新打开外部应用，旧观察仅作为保留的差异证据。页面文字/图片、高级对象、播放、公开 Agent 接入与 Musterwork E0–E3 仍未完成。
