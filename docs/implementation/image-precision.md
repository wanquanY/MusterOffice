# 图片参数误差与重复相位

2026-09-25。该阶段为[原生图片局部布局](image-layout.md)到[共享图片画笔](image-raster.md)的误差传递提供入口，修正无限平铺时仅检查单个图片区域的缺陷。完整来源图片页面、旋转基准选择及 Musterwork 接入尚未完成。

## 实现与合同

`mo-raster::ImageBrush` 增加可选 `uncertainty`。其 `origin`、`xStep`、`yStep` 是逐分量非负 Q32 世界 EMU 误差，步长单位为 EMU/源像素；`sourceDomain` 是左、上、右、下四个非负 Q32 源像素误差。没有显式源区域时，源区域误差必须全零。缺省表示输入 Q32 参数精确，显式全零与缺省产生相同结果。

世界原点重定位只修改中心值，不改变误差。上游误差经精确有理设备比例向外取整，与 Q32 → float32 转换误差相加；负误差、非法域、不能证明非奇异的变换及预算超限在调用组件前失败，不返回部分像素。图片字节、摘要、解码格式与资源上限不变。

误差对象采用可选间接存储，避免将其完整体积加入所有画笔枚举实例。画笔仍按最终参数共用；每次使用先检查并累计误差，再查去重表，不会因较精确的第一次使用而漏掉后续较大误差。

Native 与 WASM 通过既有 `render_image_paths` / `render_image_scene` 入口使用同一 Rust 实现。没有新增 C++ ABI、图片副本、图片解码或采样算法；V5/V6/V7 参数字节布局不变。Rust 类型生成 Schema/TS，没有平行手写合同。

## 误差边界

旧实现以图片尺寸/源区域边缘作为仿射系数误差的乘数。对于 Repeat/Mirror，可见区域的未折返源坐标会超过一张图片；例如每源像素约 1/3 设备像素的平铺，在 8192 像素视口上会累积数万次步长。修正前新增测试稳定失败，修正后拒绝超限请求。

新计算区分有限填充与重复轴：

1. 将 float32 矩阵、总输入误差及源区域误差包成向外的区间，校验区域正宽高及矩阵非奇异。
2. 对 Repeat/Mirror，求整个矩阵误差盒在视口四角的逆坐标界，包含源矩阵与实际编码矩阵，并在视口外留一个设备像素。重复轴使用未折返的完整范围；Clamp/Decal 使用有限图片/区域范围。
3. 用区域的最小可能周期约束重复次数，累计区域两边误差；Mirror 的两边方向变化使用保守系数。
4. 累加系数误差、平移误差以及源区域误差，后者由包含误差的矩阵映射，以覆盖系数/区域的交叉项。最终结果必须满足已有设备误差预算。

逆坐标与周期界使用不分配堆内存的 f64 区间验证器：输入 float32 可在 f64 精确表示，后续加减乘除及大整数转换均向外包围，不能用固定 epsilon 代替范围证明。采样仍由原来的组件执行。

`ImageWork.coordinateErrorBound` 包含上游误差与参数转换。它不承诺采样器内部逆矩阵算术、滤波色值误差或抗锯齿覆盖率；特别是硬边界附近，很小坐标变化也可能改变像素颜色。精度不足是显式错误，不通过缩小误差报告或降级成截图绕过。

## 复现

使用[共享裁剪阶段](shared-clips.md)的固定 Native/Cpp WASM 组件；其代码与产物没有改变。新 Rust 构建、Node bindgen 与结果保存到 `.codex-work/image-paint`，禁止覆盖历史证据目录。

```sh
export MO_SKIA_LIB_DIR="$PWD/.codex-work/clips/component"
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
cargo build -p mo-wasm --target wasm32-unknown-unknown --release --locked
.codex-work/toolchain/bin/wasm-bindgen target/wasm32-unknown-unknown/release/mo_wasm.wasm --target nodejs --out-dir .codex-work/image-paint/wasm-node
# 在该 bindgen 输出目录写入 {"private":true,"type":"commonjs"} 的 package.json。
node tools/verification/image-precision-parity.mjs
python3 tools/verification/image-precision-reference.py
node tools/verification/image-precision-regressions.mjs
```

独立参考使用 Python Fraction 精确运算，直接比较误差盒各顶点下源矩阵与编码矩阵的对应纹理坐标。它不复用生产区间/范围算法，也不是图片颜色或整页应用互操作的验收。

## 已完成验证

556 项 Rust 测试（新增 4 项）、严格 Clippy、格式及 78 份 Schema/TS 同源检查通过。288 个新图片组合覆盖缩放、斜切、翻转、轴交换、四种延展、分数区域、巨大世界原点与可选误差；两个真实入口连同 34 个组件前拒绝共形成 610 对 Native/WASM 调用。新增误差声明保留相同帧与像素，全零声明还保留相同元数据。

192 组非重复参考输入经 941,760 次精确有理数比较，全部偏差在报告界限内；所采样的最大坐标偏差约 0.000857 设备像素，不代表所有输入、滤波或抗锯齿的全局实测上限。650 对历史图片/裁剪调用的像素、帧摘要及全部元数据不变。

完整摘要、源码、合同、构建产物与探针文件绑定于[阶段证据](../reviews/evidence/2026-09-25-image-precision-verification.json)。本阶段没有新增第三方依赖或改变固定 C++ 组件，也未重跑其 Sanitizer；没有新增完整内核/安装包体积、整页延迟或峰值内存基准。

## 原生图片旋转仍需核对

本阶段另生成 24 个自有 PPTX 探针，组合 picture/shape 图片填充、0/45/90 度、`rotWithShape` 两值与水平翻转两值。LibreOffice 26.2.0.3 导出的 12 对旋转开关样例均逐像素相同，不能据此认定开关没有语义或目标应用应当如此。

Microsoft 的[图片填充定义](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.blipfill?view=openxml-3.0.1)说明开关控制是否随形状旋转，[Office 缺省说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/a9897c2b-0404-4676-aa5c-8f25bc6d66ca)给出缺省 true；这些文字没有单独解决 false 与嵌套组缩放、翻转、填充边界的完整组合。不能简单去掉旋转并沿用原始填充盒。

本机 WPS GUI 验证未执行，cua-driver 缺少辅助功能和屏幕录制权限。没有使用未验证的应用行为实现旋转策略；该分支不影响继续实现资源绑定、误差传递及统一页面绘制。后续须将旋转/填充基准、共享裁剪和文本/图片绘制顺序接到同一个来源页面编译器。
