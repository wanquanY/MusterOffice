# 原生图片世界坐标与填充裁剪

2026-09-25。`source_image_paint::compile` 已将[原生图片局部布局](image-layout.md)与[原生放置](source-placement.md)组合为世界坐标画笔，向[共享图片精度检查](image-precision.md)传递上游误差，并产生单独的拉伸填充裁剪。

这是库级计算能力。当前真实 PPTX 验证宿主可调用新函数，输出的场景使用已有 Native/WASM 绘制入口；新函数尚未由完整来源页面或 WASM 产品操作调用。不能将这部分验证当作完整图片页面、Office/WPS 互操作或 Musterwork 替换验收。

## 坐标与误差

局部布局从有效源坐标框的左上角开始，原生放置则对 `point - anchor` 求值。组合时使用 `effectiveAnchor = anchor - sourceOrigin`；这样组坐标框存在 `chOff` 时也不会将偏移重复计算。放置已包含组缩放、旋转和翻转，不能再次应用父变换。

世界图片参数按下式求值：

- 原点：`A × (localOrigin - effectiveAnchor) + translation`。
- 横向步长：`A` 的第一列乘局部横向步长。
- 纵向步长：`A` 的第二列乘局部纵向步长。

源矩阵、平移和局部布局的 Q32 值及各自误差进入 Q96 向外区间计算，最终世界画笔只量化一次。结果误差进入 `ImageBrush.uncertainty`，包含矩阵与步长误差的乘积；源区域四边及其误差继续使用归一化后的源像素单位。结果无误差时省略可选对象，避免额外分配。

拉伸填充产生 `ImageFillClip`：其路径已减去有效 anchor，携带同一原生 affine，以及逐轴世界 EMU 的 `upstreamError`。该误差只覆盖上游布局/放置计算；消费者仍须加入共享场景的变换和浮点误差。裁剪需与形状本身的填充区域相交，不能用图片透明延展代替。平铺不产生这层矩形裁剪，仍受形状填充区域限制。

裁剪路径保留局部坐标及共享 affine，没有预先烘焙成世界坐标位图，也没有复制或重新解码图片。源资源索引和目标身份不变。输入布局 profile、目标与放置的对应关系、负误差、折叠区域、数值溢出和取消均显式处理，不返回部分画笔。

## 旋转策略边界

随形状旋转的 Object/Picture/Line 使用已有原生放置。背景使用页面基准，本身没有形状旋转。不随形状旋转的对象目前返回 `OrientationRequired`；没有通过忽略 `rotWithShape`、套用浏览器旋转或删除矩阵旋转来生成近似结果。该规则与组缩放、翻转、填充边界的完整组合仍需完成，见[上一阶段探针](image-precision.md#原生图片旋转仍需核对)。

当前世界函数接受同一来源链路计算出的 `ImageSourceLayoutPlan`，不是供宿主反序列化后替换原生记录的文件协议。完整页面接入仍负责绑定源摘要、资源身份、对象顺序与共同预算。

## 验证与复现

原验证宿主 `source_image_layout` 新增开发参数 `--paint`，复用读取、索引、图片关系、提取、实际解码和局部布局，再调用世界计算。原参数及 `--audit` 输出保持兼容；新增参数不会写回源文件，也不会注册 Agent 或产品操作。

```sh
export MO_SKIA_LIB_DIR="$PWD/.codex-work/clips/component"
cargo test --workspace --locked
cargo build --workspace --examples --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 tools/verification/image-world-fixtures.py
python3 tools/verification/image-world-reference.py
cargo build -p mo-wasm --target wasm32-unknown-unknown --release --locked
.codex-work/toolchain/bin/wasm-bindgen target/wasm32-unknown-unknown/release/mo_wasm.wasm --target nodejs --out-dir .codex-work/image-world/wasm-node
# 在该 bindgen 输出目录写入 {"private":true,"type":"commonjs"} 的 package.json。
node tools/verification/image-world-parity.mjs
python3 tools/verification/image-world-reference.py --pixels
```

新输出目录为 `.codex-work/image-world`，固定 C++ 组件及 TS 薄层继续复用裁剪阶段产物；不得覆盖历史报告。世界库本身仅新增 Rust 代码，不引入第三方依赖。

560 项 Rust 测试（新增 4 项）、严格 Clippy、格式及既有 78 份 Schema 检查通过。28 个自有 PPTX 的 56 个页面/演示部件通过官方 XSD；覆盖六种角度、翻转、两层非均匀组缩放与 `chOff`、拉伸/平铺、图片/形状填充/背景。26 个源请求成功产生 74 个画笔；两个不随形状旋转的对象请求明确拒绝。

独立 Fraction 运算共核对 4,474 个误差盒顶点的世界原点、步长和裁剪边界值。26 个输出场景通过实际 Native/WASM 绘制，102 个历史 Native 图片布局请求输出逐字节不变。测试桥接代码将画笔、形状区域与裁剪放进统一 DrawScene，并扣除上游几何误差；它不代替生产页面编译器。

独立像素参考核对 40,171 个内部像素，全部相同；参考排除了形状/裁剪边缘两像素范围及接近纹理像素切换的位置，仅证明所选最近邻采样和遮挡结果。它不覆盖任意抗锯齿、过滤效果或目标应用视觉一致性。完整绑定见[阶段证据](../reviews/evidence/2026-09-25-image-world-verification.json)。

## 剩余接入工作

将这些结果接入同一 SourcePage/SceneBuilder：背景在前、picture blipFill 与 spPr 分别处理、文字保持对象顺序、裁剪与形状共用资源和预算。还需完成资源预检/解码生命周期、静止画笔策略及完整页面接口，随后验证真实 Native/WASM 产品链路。高级对象、编辑导出、播放及 Musterwork 宿主验收继续按完整目标推进。
