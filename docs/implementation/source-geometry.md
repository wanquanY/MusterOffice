# 原生形状几何声明

2026-09-24：真实 PPTX 来源索引已保存对象的预设与自定义几何，覆盖页、版式、母版中的形状、图片、连接线和组内对象。它与原生变换、线条及文字读取共用关系驱动的来源扫描和物理节点绑定，Native/WASM 返回相同记录。编辑文字不会重建或改写几何。

这一步完成声明读取，为后续公式求值和路径编译提供输入。**187 个预设名称可识别，不表示 187 种形状已经实现绘制。** 公式、路径坐标、调整手柄仍是原生声明；完整来源页面编译、效果、播放和 Musterwork 接入验收尚未完成。[封存证据](../reviews/evidence/2026-09-24-source-geometry-verification.json)绑定本轮源码、真实文件、产物和回归。

## 表示与边界

`SourceObject.geometry` 是可选字段，未声明几何时不伪造矩形。值包含物理 `sourceOrdinal`、`definition` 和未知属性的 `retainedOrdinals`。预设名称来自官方 Transitional XSD 的 `ST_ShapeType`，由脚本生成枚举事实，Rust 校验与 JSON Schema 使用同一列表。

| 原生内容 | 来源记录 |
| --- | --- |
| `prstGeom` | 预设名称、可选调整列表 |
| `custGeom` | 调整、导引公式、手柄、连接点、文字矩形和路径列表 |
| `avLst` / `gdLst` | 声明顺序、重复名称、公式词法和每个源节点 |
| `ahXY` / `ahPolar` | X/Y 或半径/角度引用、可选上下界、必需位置 |
| `cxnLst` / `rect` | 连接角度与位置、文字区域四边 |
| `path` | 可选宽高、填充模式、描边/挤出标记、原生命令顺序 |
| 路径命令 | move、line、arc、quadratic、cubic、close，控制点与端点各自绑定源节点 |

没有列表与显式空列表保持区分；路径宽高省略与显式零保持区分；省略的填充、描边和挤出标记不在读取阶段补默认值。路径填充的 none、norm、lighten、lightenLess、darken、darkenLess 六种声明均保留。这些模式尚未接入实际画笔计算。

`ST_AdjCoordinate` / `ST_AdjAngle` 允许数值和导引名称。原始词法如 `-0007`、`2.5cm`、`wd2`、`cd4` 均保留，不先猜测数值或单位。导引公式的 XSD 类型是字符串；读取成功与 XSD 通过均不证明该公式可求值。未知公式、重复名称及无法解析的引用要由后续求值规则分别诊断，不能在此阶段伪造路径。

路径宽高按原生非负 EMU 范围验证。解析器验证属性必要性、列表顺序、重复属性族和命令控制点数量；未知属性绑定原节点供后续处理。未知原生子元素返回明确错误，扩展载荷中的同名几何不会混入对象声明。

## 流式读取与资源

`mo-pptx::source::geometry` 的类型、预设名称和流式语法分别组织；解析器只保存尚未完成的类型化栈帧，完成的子记录移入父记录，不复制整棵 XML DOM。它只接受对象自身 `p:spPr` 下的声明，不把组容器或扩展中的元素当成对象几何。

MCE 先确定活动内容，几何记录继续使用物理 XML 序号，未选分支不会改变绑定。包级共享预算限制几何元素数及属性值字节数，默认分别为 1,000,000 和 32 MiB；不会每进入一个表面或对象就重新计数。原有 OPC、XML、MCE 输入限制与取消机制继续生效，未选分支也受物理 XML 预算约束。

本轮没有实现公式依赖图、预设展开、路径坐标变换或几何继承，也没有把源几何直接转换成作者模型。来源记录与求值结果保持分离，防止未验证的默认值和近似值污染文件编辑。

## 标准数据核对

标准来源为 [ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/) Part 1 / Part 4 的 2016 年版。开发验证固定官方 XSD 和附带 `presetShapeDefinitions.xml` 的摘要；标准附带形状定义仅作为忽略目录中的验证输入，没有打包进运行时，也没有据此批准模板实现的引入。

此次文件实际有 187 条定义，但只有 186 个不同名称：`upDownArrow` 两条，`upArrow` 缺失。语料按原始序号分别读取两条同名定义，没有自行重命名或推断修复。与此独立，XSD 的 187 个合法预设名称全部有真实 PPTX 识别用例。因此必须分别跟踪名称覆盖、定义读取和将来的几何求值/渲染覆盖。

公式及预设的公开说明可参见 [ShapeGuide.Formula](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.shapeguide.formula?view=openxml-3.0.1) 和 [PresetGeometry](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.presetgeometry?view=openxml-3.0.1)。这两项是后续求值设计的参考，不作为当前已实现的能力。

## 验证结果

全仓 **285 项 Rust 测试**通过，本阶段新增 8 项，覆盖完整记录、词法保留、缺省/空值、非法语法、未知内容绑定、真实编辑、取消和包级累计预算。严格 Clippy、rustfmt、TS 和 53 份 Schema 检查通过；只有 `pptx-source-response` 及其生成 TS 增加几何类型，其余 52 份合同不变。无新增外部运行依赖或锁文件变化。

新增手写模块最长 307 行；生成的 `pptx-source-response.ts` 达到 1209 行，已评估拆分：当前仍是一份自包含来源响应合同，不手工切开生成文件；后续继续扩展前应在合同生成器层引入公共类型模块，并保持 Rust/Schema/TS 同源。所有源码仍低于 2000 行上限。

423 份实际 PPTX 包含全部 187 个预设名称、标准附带的 187 条自定义几何及边界、错误和对象作用域用例。新增 **823 批 Native/WASM 检查**，其中 400 份成功输入执行了真实文字编辑及候选重读，两端索引和生成文件字节一致。

独立 lxml DOM 实现从原始 ZIP/XML 重建字段和物理节点位置，核对 5,601 条几何声明，覆盖六类路径命令、两类手柄和六种路径填充模式。2399 个表面部件通过官方 XSD；23 个非法几何探针也由 XSD 确认非法。另一个故意带未知属性的正向保留用例单独排除，不计入正向 XSD 数量。公式操作符在验证中只按词法清点，包含故意未知公式，不能作为求值覆盖率。

400 份编辑候选除授权的文字叶节点外，其余 XML 和部件保持不变。旧 3216 批主内核检查在新二进制上重新执行，累计 **4039 批**。旧来源响应只允许新增对象 geometry 字段，移除新增字段后核对原响应/封存摘要；已有字段、文字编辑候选、颜色/线条查询和渲染帧保持不变。

既有光栅参考按相同帧摘要复用；197 批 Skia 组件/sanitizer、61445 个参考像素的证据按未变源码和产物复用，本轮没有重跑组件测试。没有新增 Office/WPS/LibreOffice 运行、页面像素对照或应用编辑往返，原有兼容差异仍开放。

未压缩 CLI 为 4,571,088 字节（增加 56,592），Rust WASM 为 4,065,961 字节（增加 54,555）。其余九项工作进程、C/C++ WASM 和薄层产物字节不变。这些是未完成内核的开发产物，尚不能推算完整内核、Musterwork 安装包、峰值内存或端到端性能。

## 复现与后续

按[开发说明](development.md)准备 Native/WASM、已有基础 PPTX、官方 XSD 和几何定义验证文件；定义文件摘要由生成器强校验。Python 的 lxml 和 jsonschema 仅用于开发验证。

```sh
python3 tools/verification/geometry-shape-names.py check
python3 tools/verification/source-geometry-fixtures.py
node tools/verification/source-geometry-parity.mjs
python3 tools/verification/source-geometry-independent.py
python3 tools/verification/contracts.py --source-geometry-report .codex-work/source-geometry/parity.json
```

`source-geometry-evidence.py` 核对完整回归、真实产物和历史摘要；默认只验证，`--seal` 独占创建新证据。复现旧回归需要执行各原始入口，不得只替换报告里的产物摘要。

后续已实现有预算、显式数值规则及来源诊断的[导引与坐标求值](geometry-evaluation.md)，继续连接预设/自定义路径、原生填充和已有线条颜色至页面编译。完整文字/图片、效果、高级对象、播放、Agent 接入和 Musterwork E0–E3 仍属于原目标，未因本阶段范围缩减。
