# HarfBuzz 计算组件

这是固定版本的 C++ 塑形组件和私有 C ABI，已通过原生工作进程和独立 WASM 实例接入 [Rust 文本计算](../../docs/implementation/text-shaping.md)。上游源码通过[锁文件](lock.json)核对，解压、编译及测试产物只放入忽略目录；不使用系统安装的 HarfBuzz。组件自身的验证边界和开发命令见[组件说明](../../docs/implementation/harfbuzz-component.md)。

## 所有权与实例

`mo_hb_shape` 接收本次调用期间保持不变的字体字节、对齐的 `uint32_t` 请求数组和 ASCII 语言字节。调用者必须保证所有原始指针及其声明长度有效；这是内部 ABI，不是面向不可信调用者的内存安全接口。请求数组使用本机整数表示；当前开发宿主的二进制文件使用 little-endian，读取后显式转换。尚未验证 big-endian Native。

成功输出由组件分配，调用者只读，且只能调用 `mo_hb_free` 释放。任何非零状态清空输出指针和长度；0 字形的成功输出仍有 8 个头部 words。C++ 异常和 RTTI 在构建时禁用。版本函数返回 `(major << 16) | (minor << 8) | micro`。

每个实例必须串行调用。状态 2 永久使实例失效，后续计算返回 6；释放旧输出仍允许。没有清除失效状态的接口。Native 宿主必须更换进程，WASM 宿主必须更换整个模块实例；重新创建 `hb_font` 或 `hb_buffer` 无法保证恢复。超时、trap、非法输出同样由宿主废弃实例。原生开发宿主已可按批运行并硬超时回收；生产进程池、用户取消和浏览器 Worker 尚未实现。

| 状态 | 含义 |
| --- | --- |
| 0 | 完整成功 |
| 1 | 非法输入或当前字体不接受的轴设置 |
| 2 | 分配或 buffer 失败，实例已失效 |
| 3 | 塑形器失败 |
| 4 | 输出字形数超过请求预算 |
| 5 | 不可用的 face |
| 6 | 拒绝使用已经失效的实例 |

## 请求 v1

头部 13 个 words，随后依次为全文 Unicode scalar、feature 数组、variation 数组。全文保留被塑形 item 的前后文；语言是独立的 1–255 字节 ASCII 字母/数字/连字符串。高层 BCP 47 和 script/language 选择不在此层完成。

| word | 字段与含义 |
| --- | --- |
| 0 | magic `0x4d4f4842` |
| 1 | face index |
| 2 | 方向：4 LTR、5 RTL、6 TTB、7 BTT |
| 3 | ISO 15924 tag，四个 ASCII 字节打包为高位在前的整数 |
| 4 | HarfBuzz buffer flags；允许低 8 位但拒绝 VERIFY `0x20`，preserve/remove ignorables 不能同时设置 |
| 5 | cluster level 0–3 |
| 6 | 全文 Unicode scalar 数，最多 65536 |
| 7 | item 起点，全文 scalar 坐标 |
| 8 | item scalar 长度 |
| 9 | feature 数，最多 1024 |
| 10 | variation 数，最多 64 |
| 11 | 最大输出字形数，最多 262144 |
| 12 | ABI version，固定 1 |

每个 feature 为 4 words：`tag, value, start, end`，范围为全文 scalar 的半开区间，`0xffffffff` 用作全局末尾。重叠项保持输入顺序，语义交由固定 HarfBuzz 版本解释。每个 variation 为 2 words：`tag, IEEE754-f32 bits`。拒绝非有限值、重复轴、未知轴和超出字体轴范围的值，不静默 clamp。字体原始 fvar 16.16 转 f32 的精度策略必须由高层合同明确，当前 ABI 不替高层作隐式转换。

## 输出 v1

头部 8 words：`magic, version, upem, scale, glyphCount, bufferFlags, clusterLevel, reservedZero`。固定 `scale = upem * 64`，表示字形位置整数的单位为 1/64 字体设计单位；尚未转换字号、像素或 EMU。

随后每字形 7 words：`glyphId, cluster, glyphFlags, xAdvance, yAdvance, xOffset, yOffset`。四个位置值是有符号 i32 的位表示。cluster 为全文 Unicode scalar 下标，不是 UTF-8 字节或 JS UTF-16 下标。Rust `mo-text` 已核对 magic/version、头部、总长度、字形及 cluster 范围；后续布局只可消费通过核对的输出。

字体最大 128 MiB。256 MiB 组件分配上限包含经过钩子的 HarfBuzz 活跃分配、保留的全局分配、组件输出及分配头；不包含调用者字体/请求副本、所有 C 运行库分配或整个进程 RSS。WASM 最大线性内存 512 MiB、栈 1 MiB。两个限制不等于完整任务预算。

`MO_HB_FAULT_TEST` 只在测试构建提供确定性分配失败接口。常规构建没有该接口；故障计数和测试探针不进入产品 API。所有测试工具都属于开发宿主，不是标准 Agent 接入面。

## 字体度量 ABI v1

`mo_hb_measure_font` 与塑形共用字体设置、轴范围检查、输出所有权及整个实例的失效状态。它不接受语言或文本，使用 6 个头部 words：`0x4d4f4d54, 1, faceIndex, axisCount, metricCount, reservedZero`，随后为 `axisCount` 个 `[tag, f32bits]` 及 `metricCount` 个公开 OpenType metric tag。最多 64 个轴、28 个互不重复的度量；不允许私有 tag、非有限/重复/未知/越界轴或尾部数据。

成功头部为 `0x4d4f4d54, 1, upem, upem*64, metricCount, reservedZero`，随后每项为 `[tag, available, positionBits]`。`available` 只能为 0 或 1，0 时位置位必须为零；1 时位置是有符号 i32，真实零有效。查询使用公开 `hb_ot_metrics_get_position`，不合成缺失值。任何分配失败丢弃全部输出，并同时禁止该实例继续测量或塑形。

Rust/TS 私有批次传输头为 `0x4d4f4d42, 1, 0x0e0500, instanceCount`，后接 `[wordLength, componentRequest...]`。最多 256 个实例，41732 个请求 words、23298 个响应 words。成功批次为 `[0, count, length, componentReply..., ...]`，失败为 `[status, failedInstanceIndex]`，不包含部分结果。字体上传一次，组件当前仍为每个实例创建字体对象。完整 profile 和边界见[字体实例度量](../../docs/implementation/font-instance-metrics.md)。
