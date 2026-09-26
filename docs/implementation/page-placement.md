# 作者对象与页面坐标

2026-09-24 · `mo-presentation-compile` 已实现作者对象到页面坐标的独立计算阶段，使用与导出相同的 Document。它输出来源对象、局部锚点、已求值矩阵及误差上界；尚未实现完整页面绘制，不把这一阶段称为整页渲染或兼容性验收。

## 组合语义与适用范围

首版 profile 为 `drawingml-l4.7-separate-scale-rotation-q96-enclosure-v1-draft`，参考 [ECMA-376 Part 1 第五版](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip) 附录 L.4.7.3–L.4.7.6。该附录是解释性材料，不代替目标应用实测。坐标向右、向下为正，角度单位为 1/60000 度，正角度顺时针。

当前 profile 已改为 `drawingml-sector-scale-reflection-q96-enclosure-v2-draft`，依据实际 WPS 组变换样本修正：按子对象角度区间选择父级缩放轴、按父级翻转奇偶决定旋转方向，并使用已求值的父组空间定位后代。它替代首版的轴向比例直接相乘、旋转直接相加、原始矩阵链定位规则。详细公式、目标应用证据和原始分歧样本见[组变换兼容核对](group-placement-compatibility.md)。底层 [Draw IR 场景](scene-raster.md) 的普通仿射变换保持原义，领域规则由作者编译层负责。

当前作者模型已有的形状、图片、连接线及组合都参与坐标计算。矩形/椭圆/圆角矩形、图片和连接线的本地尺寸取对象 size；自定义路径和组合使用显式 viewport。零尺寸形状轴跳过缩放。现有模型的路径/组 viewport 严格为正、原点为零，尚不能表示导入来源中的任意 chOff 或零 chExt；不得将此范围冒充完整 DrawingML 导入支持。

页面关联的母版、版式和页面分别返回上下文，保持作者对象次序及组内先序；组记录表达坐标空间，不是绘制指令。当前输出不做占位符替换、定义对象显隐或绘制顺序求值。隐藏页面仍可显式查询，响应携带 hidden。

## 精度与运行边界

内部计算使用向外取整的 Q96 整数区间，复用已记录的 num-bigint；没有新增外部依赖或使用系统三角函数。角度先做精确象限约化，非直角在不超过 π/4 的区间计算 24 项正弦/余弦级数，并加入严格小于一个 Q96 单位的余项界。π 常量的包围区间通过独立 Machin 公式整数测试核对，90 度整数倍保持精确。

输出 `anchor` 是精确本地中心；应先从源点中减去 anchor，再使用 `affine`。矩阵已经位于页面空间，不能再次叠加父组。`uncertainty.linear` 是无量纲 Q32 系数误差，`uncertainty.translation` 是 Q32 EMU 中心误差，包含区间计算及最终 Q32 量化。

对锚点相对向量 `(x,y)`，每个轴的作者误差上界为 `error_xx * abs(x) + error_xy * abs(y) + error_translation`，单位换算须向上取整；进入 raster 后还须加上场景编译及设备坐标误差。场景自身的误差报告只覆盖已量化矩阵，不能代替作者误差。四个早期诊断样本已核对这个组合预算；现已由[作者页面编译](page-render.md)统一传递作者、几何和设备三阶段预算，当前支持纯色形状与路径，完整页面内容继续接入。

请求最多 32 MiB，全稿最多 8192 个对象，组深度沿用模型的 128 层校验。输入先执行模型完整性校验，遍历不递归，超出输出 Q32 范围明确失败；不修改输入、截断对象或发布部分布局。取消覆盖对象遍历、三角求值及结果边界，模型验证与规范摘要目前仍是调用前后检查，尚未细分到其中每一步；宿主 Worker 取消和延迟验收继续推进。

## 入口和证据

[PagePlacementRequest](../../contracts/generated/page-placement-request.schema.json) 包含 document 和 slide，输出 [PagePlacementResponse](../../contracts/generated/page-placement-response.schema.json)。开发入口为 `mo-cli page-placements <request.json>` 和 WASM `page_placements(requestJson)`；它们不读取隐式字体、网络或媒体资源。

[首版验证记录](../reviews/evidence/2026-09-24-page-placement-verification.json)保持不变，历史范围包含：

- 64 批 Native/WASM 请求，52 批求值、12 批明确拒绝；覆盖角度边界、400 个角度样本、翻转、非均匀嵌套、128 层组、8192 个对象、极大整数、退化尺寸及非法引用。
- 140 位 Decimal 独立参考使用 Chudnovsky π，逐对象重走中心坐标链，核对 8807 个对象、52842 项系数/中心和 44042 个源控制点；不复用 Rust 区间或矩阵合并实现。
- 新增 7 项 Rust 测试，共 232 项；此前 2180 批在新产物上语义结果不变，主内核累计 2244 批。46 份 Schema、TS、Clippy 通过，44 份原有 Schema 不变。
- 四个明确限制为矩形的诊断场景由真实 Skia 绘制，Native/WASM 像素相等；同一作者文档的 PPTX 导出也逐字节相等。诊断桥检查具体内容/样式，不能作为完整页面绘制入口。

首版四份 PPTX 在 LibreOffice 26.2.0.3 的 PDF 中均有明显差异，顶点到另一侧边界的最大距离约 28.18、60.04、60.02、28.16 pt。该结果触发了作者规则修正，原始证据没有覆盖。v2 对同一输入的四项观察现均低于 0.1 pt；新增 48 个非对称形状已与 WPS 实际画面核对，当时另有 15 个角度样本与 LibreOffice 不同，后续已由[静态旋转导出](static-rotation-export.md)解决其序列化表示差异。最新为 237 项 Rust 测试、2271 批主内核跨端结果；原始测量见[组变换兼容核对](group-placement-compatibility.md)，不能将这些局部观察算作完整目标应用兼容通过。

```sh
cargo test --workspace --locked --offline
cargo build -p mo-cli -p mo-text-worker -p mo-raster-worker --release --locked --offline
cargo build -p mo-wasm --target wasm32-unknown-unknown --release --locked --offline
node tools/contracts/build-wasm-node.mjs
node tools/verification/page-placement-parity.mjs
python3 tools/verification/page-placement-reference.py
python3 tools/verification/contracts.py --page-placement-report .codex-work/page-placement/parity.json
node tools/verification/page-placement-render.mjs
```

外部观察另需先用独立临时用户配置的 LibreOffice 将四个自有诊断 PPTX 导出到 `.codex-work/page-placement/libreoffice/`，再用带 PyMuPDF/Pillow 的开发 Python 执行 `tools/verification/page-placement-external.py`。这些工具不加入运行依赖。

后续将此阶段连接几何求值、文本布局、样式继承和完整 Draw IR，补充子坐标原点、更多目标应用语义与生产资源预算。播放、高级可编辑对象、Agent 接入及 Musterwork E0–E3 仍未完成。

更新：作者页面已新增实际编译与绘制入口，最新增量范围和 242 项 Rust 测试、2345 批主内核跨端结果见[作者页面绘制](page-render.md)。本文中的 237/2271 为静态旋转阶段的历史统计。
