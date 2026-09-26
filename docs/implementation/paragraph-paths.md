# 段落路径与曲线边界

已将自动断行、实际字形位置和真实字体轮廓连接成可复用的段落路径场景。Native/WASM 使用同一 Rust 几何算法；绘制后端可以直接消费已定位路径，无需重新排版。当前范围为单色、无 hinting、nonzero 填充的字形几何，尚未实现完整 CPU 栅格、GPU 合成、彩色字体、笔刷/效果或页面渲染。

入口为 `mo_text::scene::paragraph_paths`、`paragraph_paths_json`、CLI `paragraph-paths`、worker `--paths` 和 WASM `paragraph_paths`。请求包含既有 `ParagraphLayoutRequest` 以及显式 `boundsTolerance`。结果保留完整布局证据，另有 `scene` 或明确的未决问题。作者文字和软折行语义不改写。

## 精度与共享职责

新增纯计算 crate `mo-geometry`，只持有 Q32 数值、点/矩形、线段/二次/三次路径和有界的几何计算，不依赖字体、演示模型、宿主或绘制库。既有行几何的 i128 Q32 算术迁入该 crate，旧布局继续使用同一除法、半值舍入和溢出规则，避免字体布局与绘制位置拥有两套实现。

`Fixed` 的 wire 值是原始有符号 i128 的规范十进制字符串，实际数值为该整数除以 2^32。它本身不附带长度单位，所属几何 profile 定义单位；本场景中全部路径/原点/边界/容差的单位都是 EMU。Schema 记录范围，Rust 反序列化拒绝非规范表示和越界；JSON 数字不能代替这些字符串。

正常布局接口仍输出整数 EMU，不为该调用保留额外原点数组。路径场景在同一次布局内部取得未经过 wire 舍入的 Q32 原点，直接连接真实字形。典型 0.6 EMU 的原点会保留，而不是先变为 1 EMU 再作绘制变换。字号/基线、双向视觉次序和原始字形索引均沿用实际布局。

字体设计坐标按 `fontSize / positionUnitsPerEm` 转成 Q32 EMU，Y 向上转为场景 Y 向下；不把 glyph advance 当作轮廓宽度。字体设计坐标本身仍采用[轮廓入口](font-outlines.md)的 design64 量化 profile，Q32 不会恢复此前已经舍入的精度。

## 曲线边界的保证

`mo_geometry::path_bounds` 计算路径段的几何位置范围，支持开放轮廓、闭合线段、二次和三次 Bézier。Move-only 路径无范围；退化线段仍可产生零宽/零高范围。它不是“所有填充后非透明像素”的精确边界，也不包含 stroke、阴影、滤镜、抗锯齿覆盖或裁剪。

算法采用带向外舍入的整数区间 de Casteljau 细分，不靠浮点求根或把整个控制点框直接交作最终边界：

1. 每个控制点初始为精确 Q32 单点区间，中点计算的下/上界分别向负/正方向取整。先除后加，避免两个极端 i128 直接相加溢出。
2. 子曲线的控制区间形成包含整条子曲线的外框。采样端点区间提供“真实最小值不大于此值、真实最大值不小于此值”的见证界。
3. 当外框每边与见证界相差不超过容差时接受该叶子，否则继续细分。见证界只会加强，因此已接受叶子仍然满足最终保证。
4. 合并所有叶子得到覆盖整条路径的范围，每条边超出真实曲线极值的量不超过所请求容差。没有通过预算或深度限制时返回错误，不把粗控制框当作达标结果。

核心几何最小容差为 256 个 Q32 单位，段落入口另限定最大 2^32（1 EMU）；验证一般使用 2^26（1/64 EMU）。这些保证针对**输出的量化曲线**，不是未量化字体或最终像素。整数区间本身不使用浮点数，120 位 Decimal 求导/求根仅存在于独立验证程序。

工作栈在一条路径中复用，子分支深度最多 64；请求共享 2097152 个路径指令/细分节点预算。每个节点有取消点，超出预算或数值范围时整批失败。

## 资源与可用性

场景包含字体实例、可复用路径资源和按视觉顺序排列的字形实例。原点与路径分开；重复字符不复制整条路径。相同字体实际资源、有效 binary32 轴和 glyph ID 只取得一次源轮廓；不同字号复用该源轮廓，再分别缓存已缩放的路径/边界。字体绑定别名可归并，片段的实例解析也复用。

本阶段最多 64 个选中字体实例、4096 个唯一源 glyph、262144 条源轮廓指令和 262144 条已缩放路径指令。单次字体调用最多 256 个唯一 glyph，解释预算继续由[组件](font-outlines.md)控制；超过一批时按块调用。这个块边界仍可能重复上传同一字体字节，常驻字体句柄与跨请求缓存尚未实现，不能据此宣称最优性能门禁完成。

空白字形保留实例和布局 advance，路径为空、bounds 为 null。任何缺失轮廓使 `scene` 为 null；已经计算出的局部路径不会发布。存在彩色相关表而未完成具体 glyph 表示选择时，同样返回 `ColorRepresentationRequired`。这是一项保守的未决前提：不把有单色轮廓的彩色字体自动画成黑色。诊断中的 `font` 指向原始请求字体绑定，附有效变化轴，在没有 scene 字体表时仍可解析。

已有 tab、连字符、缺字体等布局前提仍在 `layout` 中表达；没有完整布局就不调用轮廓组件。组件失效和底层错误返回整批错误，不发布布局加部分 scene。真正的业务成功必须检查 `scene` 与所有诊断，不能仅检查 JSON envelope 的 `evaluated`。

## 验证与边界

[证据](../reviews/evidence/2026-09-24-paragraph-paths-verification.json)绑定完整源码、合同、实际产物与验证输入：

- 50 批 Native/WASM 响应一致，覆盖中英、阿拉伯双向/括号、天城文、Emoji、不同字号/基线、轴、二次/三次/复合字形、空行、预算、字体别名和未决表示。
- 160 个字形映射、2250 条路径命令按独立 BigInt 缩放逻辑与独立轮廓入口核对；34 个原点分量实际含小数 EMU。
- 独立 Python Fraction 重算 29 个完整场景的原点；120 位 Decimal 用解析导数求极值，核对 99 份路径资源、2047 条段及 768 项坐标。边界均向外，且超出量在容差内；比较 epsilon 为原始 Q32 单位的 1e-80。
- 自有测试检查非 dyadic 极值、负数奇数中点、大坐标、退化/开放轮廓、共享路径、不同字号、全部取消检查点及非法输入。字体层取消仍按公共 `CANCELLED` 合同处理。
- 真实 WASM 分配失败发生在布局/度量成功后的轮廓调用，整批无结果、模块永久失效，替换模块后恢复。
- 旧 1833 批跨端语义与 38 份已有 Schema 保持不变。当前共 205 项 Rust 测试、1883 批主内核对比和 40 份 Schema；C++ 组件及 TS 适配器字节未变。

实际场景另生成 7 份诊断 SVG，内容只有路径和实例，没有文本节点、系统字体或外部资源。已检查中英/阿拉伯及原创三次曲线的可见预览；初次 Quick Look 缩略图裁剪由诊断 SVG 固定方形视口解决，路径/布局未改。预览的栅格化由系统查看器完成，不能作为本项目 CPU 后端或 Office/WPS 视觉验收证据。

```sh
python3 tools/verification/paragraph-path-fixtures.py
node tools/verification/paragraph-paths-parity.mjs
python3 tools/verification/paragraph-paths-reference.py
python3 tools/verification/paragraph-paths-preview.py
python3 tools/verification/contracts.py --paragraph-paths-report .codex-work/paragraph-paths/parity.json
target/release/mo-cli paragraph-paths .codex-work/paragraph-paths/cli.json .codex-work/paragraph-paths/cli.bin
```

本轮只新增内部几何 crate，无新的第三方版本或字体运行包。产物大小为不完整开发构建，未做新的时间/内存基准。后续继续接入实际绘制后端、通用 Draw IR 的笔刷/图像/clip/group/effect、页面模型编译和目标应用视觉比较。高级可编辑对象、播放与 Musterwork E0–E3 仍未完成。
