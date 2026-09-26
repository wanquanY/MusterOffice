# 原生预设形状展开

2026-09-25：真实 PPTX 的 187 种 `prstGeom` 已接入导引求值、路径编译和几何绘制验证。CLI、范围读取接口与 WASM 共用实现；源文档仍保留原生预设和调整值，没有改写成图片或自定义路径。源码、实际产物和全部回归绑定[本轮证据](../reviews/evidence/2026-09-25-preset-expansion-verification.json)。

这一步补齐了普通原生形状进入绘制链路的入口，**尚不等于完整来源页面或 Office/WPS 兼容验收通过**。验证绘图使用明确的诊断颜色和线宽，没有冒充原文稿样式。

## 定义与数值

[固定来源](../../components/drawingml-presets/source.json)是 ECMA-376 Part 1 第五版附件中的几何定义，原始 XML 按 SHA-256 原样保存。[生成器](../../tools/contracts/generate-preset-catalog.py)输出确定性的紧凑 XML 和索引，并生成[逐形状清单](../../components/drawingml-presets/catalog.json)。运行数据共 350386 字节，复用已有流式 XML 语法与导引解释器，不引入浏览器、第三方办公库或运行时文件访问。

原附件存在可复现的问题，修订只作用于实现派生数据，不覆盖或冒称修订了标准：

- 两份完全相同的 `upDownArrow` 取第一份；缺失的 `upArrow` 用 `downArrow` 的垂直镜像构造，保持调整域、文字区域、手柄和连接方向。
- 三种循环箭头共八条 `+-` 公式多出尾部零。生成器逐条核对原公式并移除多余操作数，清单保存修改前后内容。
- 附件使用的 `wd12`、`wd32`、`hd10`、`cd3` 在预设定义内部解释为明确比例；不扩大自定义几何及文档调整公式的内建名称集合。

以上属于显式 draft 实现规则，尚未宣称各目标应用采用完全相同规则。用户 PPTX 中的错误公式仍返回定位诊断，不套用附件修订。许可依据及全文位于[组件说明](../../components/drawingml-presets/README.md)和[版权文本](../../components/drawingml-presets/COPYRIGHT.txt)。

每次查询维护一个有界生命周期的模板缓存，同种预设只解析一次；无全局可变缓存。模板解析、属性字节、导引计算及重复对象求值都消耗共享预算，解析和计算接受取消。模板不复制文档调整值；每个对象先计算默认调整，再按文档顺序计算覆盖，最后计算模板导引。重复调整名保留记录，并绑定最近值；未触及的默认值继续有效。

`NativeShapeType` 内部改为经过验证的紧凑索引，公开名称仍为原字符串。来源类型可复制，路径与依赖记录不再反复分配预设名字符串。

## 来源合同

几何 profile 升为 `ecma376-2016-ms-presets-draft-v2`。开发中的 v1 请求明确拒绝；Rust、四份受影响 Schema 和生成 TS 类型同步更新。

派生记录、诊断、导引依赖、手柄引用、路径及命令来源映射使用 `origin`：

```json
{"kind":"document","sourceOrdinal":23}
```

```json
{"kind":"preset","preset":"roundRect","definitionOrdinal":7}
```

文档序号属于原物理 XML 部件；预设序号属于该 profile 固定运行定义的零起点元素先序遍历。二者不会混用。`EvaluatedGeometry.sourceOrdinal` 仍标识文档中的几何声明本身。文档覆盖值保持文档来源；引用它的模板导引、手柄可跨来源指向该值。原始来源声明合同不变，只有派生结果改为显式来源类型。

路径仍保留原始填充、描边及挤出修饰和省略状态；多路径不合并。Q96 弧线计算、Q32 量化和误差合同沿用[原生路径编译](native-paths.md)，其中误差界仍相对于 binary64 导引输出，不是完整页面的视觉误差证明。

## 实际验证

- 363 项 Rust 测试、严格 Clippy、rustfmt、61 份 Schema 和 TS 类型检查通过。新增测试覆盖全部 187 种形状的三种尺寸、覆盖和依赖来源、重复对象缓存、预算、取消及自定义几何规则隔离。
- 571 个真实 PPTX 输入：561 个默认形状/纵横比组合、7 个调整值探针和 3 个错误探针。568 个输入成功求值并编译，3 个错误有文档来源诊断；全部幻灯片 XML 通过官方 Transitional XSD。
- 新增 1329 批 Native/WASM 对照：571 批几何查询、571 批路径编译、187 批实际绘制。结构化结果及绘制像素逐字节一致。所有 187 种形状的实际像素已生成图集并查看。
- 独立 80 位十进制参考核对 43832 个导引/几何值和 52091 处来源。参考读取原始附件而非运行定义数值；向上箭头另以求值后的向下箭头作几何镜像检查。预设绝对/相对容差为 `1e-7 + 2e-11 × max(abs(expected), abs(actual))`，最大占比约 0.002167。
- 独立路径检查核对 967 条路径、75481 个控制点、22852 段弧及 68556 个椭圆样点。误差不超过既有局部区间与余项界。
- 旧 7738 批全部重跑。5541 批记录保持原样，2197 批逐字段验证仅有 profile 和显式来源迁移；旧自定义几何数值、路径和文件编辑结果未改变。旧独立几何/弧线参考也重跑通过。累计为 **9067 批主内核跨端检查**，不等于用户场景或目标应用验收数。

本轮未重新构建未修改的 C++ 组件，也没有重复声称新增 sanitizer 或 Office/WPS 验收。

## 开发产物体积

未压缩、未完成的开发产物：

| 产物 | 当前字节 | 相对上一阶段 |
| --- | ---: | ---: |
| Native CLI | 5491104 | +377328 |
| Rust WASM | 4986960 | +373145 |
| Native 绘制工作进程 | 3703184 | 长度相同，摘要改变 |
| Native 文本工作进程 | 2375360 | 长度相同，摘要改变 |
| 绘制 C++ WASM | 1808300 | 原字节 |
| 字体 C++ WASM | 841468 | 原字节 |

两个 WASM glue 和两个 TS 组件适配器保持原字节。Cargo/pnpm 锁文件未改变。体积增加主要来自完整预设数据和来源处理；350386 字节运行表已经计入 CLI/WASM，不能再次相加。没有测量本阶段 RSS、冷启动或完整页面延迟，也不能据此计算 Musterwork 安装包。

## 复现与下一步

按[开发说明](development.md)构建 Native/WASM，并准备既有官方来源几何语料，然后运行：

```sh
python3 tools/contracts/generate-preset-catalog.py --check
python3 tools/verification/preset-expansion-fixtures.py
node tools/verification/preset-expansion-parity.mjs
python3 tools/verification/preset-expansion-independent.py
python3 tools/verification/native-path-independent.py --presets
python3 tools/verification/contracts.py --preset-expansion-report .codex-work/preset-expansion/parity.json
```

全覆盖声明限于已测默认纵横比和列明的调整探针，不代表全部调整边界、退化尺寸或目标软件编辑手柄交互均已通过。接下来继续来源几何继承/页面变换、原生画笔空间计算、文字/图片/效果的共享页面编译，再完成高级对象、播放、Agent 服务、Office/WPS 往返和 Musterwork E0–E3。完整原生预设创建与调整编辑也仍需接入作者模型及原子操作，不能把本次查询/编译能力当成完整编辑器。
