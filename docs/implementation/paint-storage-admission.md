# 画笔共享存储与构建预算

2026-09-25：修复原生页面在最终栅格预算检查之前，按几何路径复制渐变色标的内存放大问题。实际页面、Native/WASM 与公共入口已验证；[阶段证据](../reviews/evidence/2026-09-25-paint-admission-verification.json)记录实现、测量范围及既有行为回归。完整一期和 Musterwork 替换仍未验收。

## 问题与实现

原先 `Gradient.stops` 是 `Vec<GradientStop>`。同一原生形状有多个填充路径时，每个场景 instance 都克隆整份数组；场景降低中的画笔重定位再次克隆。最终渐变编译器虽然限制输入色标总量，但页面构建会在该检查之前消耗内存。通用场景编译原本已经在降低前检查总量，本次核查保留了这一正确行为。

`GradientStops` 现在封装 `Arc<[GradientStop]>`。渐变克隆、场景 instance 和世界原点重定位共享同一份已求值色标；几何参数继续独立保存。修改工作值需显式调用 `make_mut()`，有共享者时复制，独占时原地修改；其他画笔的值保持不变。数组次序、硬色标、浮点工作通道均保留。

共享范围是同一已求值画笔及其克隆，独立解析的相同数组仍可能独立分配。本阶段没有实现跨文稿全局缓存。序列化仍输出完整 JSON 数组，因此本项优化不减少重复序列化数组的字节数。

Rust 未发行的绘制 API 将 `Gradient.stops` 改为 `GradientStops`，调用方用 `vec![...].into()` 构造，用切片读取或显式写时复制；JSON、Schema、TS、C++ 帧和可选能力版本保持不变。80 份 Schema 与 80 份生成 TS 同前阶段逐字节一致，没有新增依赖或 Serde 全局功能开关。

## 提前检查与职责

`mo-raster::PaintBudget` 统一已有的绘制数量与输入色标工作量，失败不提交计数。共享存储也按逻辑绘制次数计费，避免因内存复用而放松 CPU 工作量限制。

| 位置 | 检查时机 |
| --- | --- |
| 来源页面预检 | 原生路径确定填充/线条用途后，在构建派生 instance、图片解码及文字塑形之前累计 |
| 共同 `SceneBuilder` | 写入路径、变换、instance 与来源记录之前检查；作者页面、原生页面、文字装饰共同使用 |
| 通用场景降低 | 保留原有降低前检查，改用共同预算 |
| 最终栅格编译 | 保留已有总量检查，采用共同常量和色标加法检查 |

既有上限继续为 65,536 个绘制、262,144 个逻辑输入色标；单渐变合法性、存储色标上限、坐标与数值验证仍由原有对应层负责。背景填充计入预算，未实际填充且不描边的路径不产生绘制；裁剪节点使用独立预算。文字塑形后生成的实际绘制仍由共同 builder 检查，不能将“来源路径提前检查”解读为所有文字工作都已经提前计数。

这不是整个进程的内存上限。来源 XML/颜色绑定、路径、序列化、图片资源和后端缓冲仍有各自的存储与预算。

## 验证

- 590 项 Rust 测试、严格 Clippy、格式与 80 份同源合同检查通过。新增 6 项测试覆盖共享存储、写时复制、协议一致、预算边界、失败原子性、空绘制和实际原生页面。
- 来源样本包含一张先出现的图片、4096 色标及 64/65 个自定义路径。64 个实际填充路径被接受且只持有一份色标；65 个填充路径在图片解码前拒绝。将第一条路径设为无填充后重新被接受；增加渐变背景的超限样本也在解码前拒绝。
- 四个自有 PPTX 的 28 个部件通过官方 XSD。111 对来源页面 Native/WASM 公共请求通过，89 对成功，其中前一阶段 107 对元数据和像素不变。新超限项解码与栅格调用均为零；两个等价有效样本像素一致。
- 854 对旧图片、裁剪、合成及渐变公共请求，307 对旧页面/文字请求保持一致。CLI 验证成功创建、拒绝覆盖和失败不发布像素文件。
- C++、图片解码与塑形组件沿用前阶段固定产物及验证，没有在本阶段重新声称运行新的 C++ 内存检查。Rust WASM 为 6,214,446 字节，较前阶段增加 8,270；C++ WASM 仍为 2,325,263 字节。

## 局部内存与时间测量

设备为 Apple M4 Max、36 GiB 内存、Darwin 25.2.0 arm64，Rust 1.92.0；使用仓库 release 配置，thin LTO、单 codegen unit、溢出检查开启。每个独立进程保留 64 次绘制的 4096 个色标；无字体或图片。共享项执行实际 `Brush::rebased`，对照项执行独立 `Vec<GradientStop>` 克隆，用于比较原复制机制；它不是历史完整内核二进制的基准。

7 轮交错测量，每轮均启动新进程。文件页缓存未清空，第一次运行后的页缓存通常已热。`/usr/bin/time -l` 在本 macOS profile 中报告整个测量进程的最大 RSS；纳秒值仅覆盖构建操作，不含渲染或进程启动。

| 指标 | 共享画笔 | 复制数组对照模型 |
| --- | --- | --- |
| 保留的色标缓冲 | 1 | 64 |
| 色标有效数据 | 163,840 字节（160 KiB） | 10,485,760 字节（10 MiB） |
| 构建中位数 | 3,375 ns | 740,375 ns |
| 构建范围 | 2,750–4,625 ns | 608,208–1,027,750 ns |
| 进程最大 RSS，七轮同值 | 1,949,696 字节 | 12,451,840 字节 |

色标有效数据减少 **98.4375%**。有效数据不含对象、分配器头、基准输入数组或编译/栅格缓冲；上述 RSS、时间和色标差值不能外推为整个演示文稿、Musterwork 安装包或完整产品性能收益。

## 复现

固定组件仍在 `.codex-work/gradient-field/component`；本阶段的新输出使用 `.codex-work/paint-admission`。此前封存目录不覆盖。

```sh
python3 tools/verification/paint-admission-checks.py
python3 tools/verification/contracts.py
cargo build --release -p mo-raster --example gradient_storage --locked
python3 tools/verification/paint-storage-measure.py
node tools/verification/paint-admission-parity.mjs
node tools/verification/paint-admission-regressions.mjs
node tools/verification/text-page-runtime-regressions.mjs .codex-work/paint-admission .codex-work/gradient-field/component .codex-work/paint-admission/ts-raster/index.js
python3 tools/verification/paint-admission-evidence.py
```

证据脚本直接对四份来源 PPTX 运行官方 XSD 校验，并验证源代码、构建记录、生成合同及运行产物摘要。Python 验证需要 `jsonschema` 和 `lxml`。测量脚本当前明确限定 macOS RSS 单位，其他平台需建立相应测量 profile。
