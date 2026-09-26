# 静态旋转的原生导出表示

2026-09-24 · 新建 PPTX 的静态 `a:xfrm/@rot` 已统一写为一圈内的非负等价角度，解决了[组变换核对](group-placement-compatibility.md)中 15 个负角度/多圈角度的 LibreOffice 几何差异。作者模型、来源保留编辑和动画角度分别处理；本次没有扩大为完整互操作验收。

## 语义边界

`Transform.rotation` 继续保存调用者提供的 i32 作者值，不改写负号或圈数。模型提供 `normalized_rotation()`，按每圈 21600000 单位返回 Euclidean 余数；静态作者坐标求值和新建 PPTX 的序列化共用这一定义。负角度、完整圈数和 i32 极值都使用整数运算，不经过浮点角度换算。

新建导出器在形状、图片、组、连接线共享的变换 writer 中输出规范方向值。现有自由端点连接线只支持不需要进一步端点归一化的方向；整数整圈与零方向等价，已纳入同一判断。其他尚未支持的非零旋转或翻转端点映射仍明确拒绝，未放宽为错误坐标。

来源保留编辑继续逐字节保留未改动的 XML 属性及其他压缩条目，不能为了新建导出的表示规则重写导入文件。完整来源几何编辑仍需独立实现。本规则只针对静态对象变换，**不能用于动画的旋转增量、转动方向或累计圈数**。

[ECMA-376 Part 1 §20.1.10.3](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip) 将 ST_Angle 定义为 1/60000 度的有符号整数，负角度本身合法。这里采用一圈内表示是经过实际目标应用验证的导出选择，不将原始负角度判为无效输入，也不修改规范含义。

## 实际文件和目标应用

首先仅对三份自有文稿的旋转值做等价处理，确认差异来自序列化表示；正式实现随后用原始作者请求重导出，得到与实验文件逐字节相同的 PPTX。检查覆盖 48 个非对称图形及 12 个定位标记，包含临界角、负角、多圈、父子翻转和嵌套组。

- WPS 12.1.22553 重新打开三份正式输出。与先前原始角度文件在相同窗口中的画布区域比较，1730538 个截图像素全部相同；四个标记的配准值也相同。这证明此次表示变化没有改变这些样本的 WPS 画面，不代表内核与 WPS 的全像素保真。
- LibreOffice 26.2.0.3 重新打开正式输出并生成 PDF，48 个图形均在此前设定的 0.2 pt 几何观察容差内，最大顶点到边界偏差约 0.10992 pt。原来存在明显差异的 15 个样本全部进入该容差，原始文件、旧 PDF 与历史证据保留。
- 三份诊断文稿及七份已有成功导出文件共十个包经过独立 ZIP/XML 差异检查，只改变 23 个旋转属性值；其余解压后的内容字节完全相同。路径、组层级、媒体、文本和可编辑对象没有被展平或重绘替换。
- 另有 20 批角度专项，每批将原始作者请求与等价方向请求分别送入真实 Native/WASM 导出，并检查来源索引；文件逐字节相同。涵盖 i32 最小/最大值、负值、整圈、多圈与角度分区边界，也覆盖自由端点连接线的整圈等价输入。
- 4 批来源保留专项在包含负值、多圈及 i32 极值旋转的组内部编辑文字，Native/WASM 候选字节一致。独立 XML 检查确认只替换指定文字，80 个未修改压缩条目的 payload 原样保留。
- 30 份实际输出经 `python-pptx`、`lxml` 和官方 ECMA Transitional XSD 独立校验：288 个部件实例、529 个原生对象及其静态旋转。原生对象种类、顺序、组归属、文字、路径、图片摘要和连接引用均检查。

截图通过 `cua-driver` 在调用者已有权限下后台获取。完整窗口包含其他页签，仅留于忽略的临时目录；公开证据只记录自有页面测量值、画布区域摘要、输入输出摘要和版本，不发布用户界面内容。

## 回归范围和限制

237 项 Rust 测试、严格 Clippy、46 份 Schema 和 TS 检查通过。已有 2230 批非导出计算/来源操作/作者坐标结果保持不变；17 批导出回归中，七份成功文件仅产生已核对的旋转表示变化，十项拒绝保持原行为。新增 20 批导出与 4 批来源保留专项后，主内核累计 2271 批跨端验证。原有 67 批作者坐标的请求和响应均不变。

未压缩开发产物为 CLI 4113632 字节、文字 worker 2375232 字节、绘制 worker 2839936 字节、Rust WASM 3737534 字节和 Rust glue 25307 字节；两套 C++ WASM/薄层不变。这些仍只是部分内核开发产物。

没有新增外部运行依赖、更新锁文件或改变 wire Schema。完整内核和 Musterwork 安装包的体积、性能仍未验收；本轮也未验证其他 OS、浏览器 Worker、PowerPoint、外部编辑后重存、动画播放或完整文稿保真。

```sh
python3 tools/verification/group-placement-fixtures.py .codex-work/angle-export canonical-
node tools/verification/group-placement-parity.mjs .codex-work/angle-export
node tools/verification/angle-export-parity.mjs
python3 tools/verification/angle-source-fixtures.py
node tools/verification/angle-source-parity.mjs
```

独立文件验证使用开发 Python 执行 `angle-export-independent.py`。WPS/LibreOffice 实际观察随后由 `group-placement-observations.py .codex-work/angle-export` 和 `angle-export-observations.py` 核对；窗口位置及截图必须重新目视确认，不能以旧截图代替新执行。完整重现还需既有导出回归报告、官方 XSD、原始角度语料和实际生成的 PDF，相关摘要见[验证证据](../reviews/evidence/2026-09-24-static-rotation-export-verification.json)。

接下来继续完整页面编译、来源几何映射与目标应用编辑往返。E0/E1–E3 及 Musterwork 替换门禁仍未完成。
