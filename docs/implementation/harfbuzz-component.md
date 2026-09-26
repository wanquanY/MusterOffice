# 固定字体塑形组件

2026-09-24 · E0 文本计算进展。HarfBuzz 14.5.0 已由同一固定源码构建为 Native C++ 静态库及独立 WASM 模块，能够对显式字体与分段上下文进行真实塑形。本页记录独立组件阶段；随后已接入 [Rust 文本计算与隔离宿主](text-shaping.md)。布局和 Musterwork 链路尚未完成，完整 E0 仍未通过。

## 采用范围

[锁文件](../../components/harfbuzz/lock.json)固定官方发行源码的 URL、长度与 SHA-256。构建不选择系统库，不修改上游源码；内部[窄 ABI](../../components/harfbuzz/README.md)负责输入范围、资源限制、输出所有权、状态和固定 profile。计算使用 OpenType font functions 和明确的 `ot` shaper，没有 CoreText/FreeType/系统字体隐式替补；未开启 `HB_MINI/LEAN/TINY`，未为缩小产物关闭复杂文字、可变字体或竖排。

Rust 主体＋精选 C/C++ 组件方向延续 [ADR 0003](../decisions/0003-language-and-component-strategy.md)。此前 HarfRust 0.13.3 的公开成功状态可见性缺口保留在[字体资源证据](../reviews/evidence/2026-09-24-font-resources-verification.json)。本轮选择 HarfBuzz 是因为可接入分配钩子、显式 shaper 和失败检查，并取得了真实跨端结果；这不意味着它的所有失败路径或全部字体已经验证。

脚本、语言、方向、cluster level、feature 和 variation 均显式给出，不调用猜测段落属性。原始文本不做隐式规范化；允许塑形器执行其内部 Unicode 规范化语义。输出保留字形 ID、字簇、字形 flags 和整数 advance/offset。固定位置精度为 1/64 字体设计单位；字号/行布局转换由后续 Rust 文本层负责。上游生成 UCD 表的头部标记为 Unicode 18.0.0；该文件摘要记入证据。段落分词、双向与换行所用 Unicode profile 尚待统一，不能仅从该标记宣称完整 Unicode 一致性。

## 失败必须隔离

只检查 `hb_shape_full` 和 `hb_buffer_allocation_successful` 不足以证明一个经历分配失败的实例可以继续使用。对固定 Noto Sans 输入 `office affine AV á`，本轮观察器在第 7、16 次成功分配后开始拒绝分配，随后恢复分配并创建新的字体/buffer 重试。重试时两个上游成功值均为 true，但字形序列与未受故障的基线不同；一组返回 19 个缺字 ID，而基线有 14 个正确字形。这是实际复现，不是仅由接口推断。

组件因此把分配失败视为整个实例永久失效，丢弃输出，之后返回 `instance invalidated`。没有清空标记继续运行的路径。Native 需要宿主废弃包含 HarfBuzz 全局状态的工作进程；WASM 需要新建模块实例。Rust 文档状态应留在该实例之外。当前试验验证了拒绝复用和新实例可工作，生产工作进程池、取消、超时与资源租约回收仍待实现。

四个上游分配钩子统一追踪活跃字节并限制为 256 MiB；字体、文本与宿主副本另计。低层调用还检查 buffer 状态、shaper 返回值、输出数量和指针；任何失败都不发布部分 glyphs。300 个分配位置的故障注入只覆盖一个固定拉丁样本，不能推广为全部字体/分配失败路径已经穷尽。未执行线程并发验收；每个实例按串行调用约束使用。

## 能力与宿主边界

构建关闭上游文件打开、环境变量、locale 读取和 buffer 消息接口。当前 WASM 仍含 C 运行库引入的 `fd_close/fd_write/fd_seek` imports，以及内存增长 import。验证宿主将三个 fd 接口替换为立即抛错的拒绝函数；本轮正常及故障用例均未调用它们。不能把“测试期间没有调用”写成“二进制没有 I/O imports”。正式薄宿主必须继续拒绝这些能力，Native 进程还需实现对应权限隔离。

本组件使用 Emscripten 自带 C/C++ 运行时和 ES module 启动代码，不手写 libc，也不让 JS 重新实现塑形。Rust 主模块与 C++ 模块各自管理线性内存，当前测试存在字体和请求复制；实例/字体复用、批量接口和复制预算尚待实际 SDK 与性能实验。没有端到端时间、峰值 RSS 或大文稿性能结论。

## 本轮证据

[组件证据](../reviews/evidence/2026-09-24-harfbuzz-component-verification.json)绑定源码、上游归档、工具链、实际二进制、语料及逐项结果：

- 45 个 Native/WASM 用例：33 个成功、12 个拒绝，303 个输出字形的 ID、字簇、flags、advance/offset 完全相同。覆盖拉丁 ligature/kerning、组合符、阿拉伯连接与前后文、天城文重排、中文双向竖排、单色 Emoji 序列、可变轴、UVS、TTC/CFF 和错误请求。
- 31 个成功用例的 301 个字形与同版、未修改的上游 `hb-shape` CLI 相同。这是独立入口对照，仍共享 HarfBuzz 算法，不是 Office/WPS 排版正确性证明。空文本没有 CLI 输出行；禁止 dotted circle 的 flag 无对应 CLI 参数，另用 FontTools cmap 核对其有/无。
- 300 个故障位置在 Native/WASM 分别触发 151 次实际失败；失败输出为空，随后复用全部拒绝。未触发失败的其余位置与正常基线一致。
- Native ASan＋UBSan 下重复 45 个用例及 300 个故障位置，无 sanitizer 报告。没有 TSan 或穷尽字体模糊测试结论。
- LLVM deterministic archive 模式消除时间戳/uid/gid 差异；同机连续两次完整 Native 测试构建的四个产物 SHA-256 相同。这不是跨机器或全发行工具链可复现构建证明。

上述 45 组是独立组件阶段的测试。当时 Rust 内核源码及开发 CLI/WASM 与字体资源证据相比未变，因此没有合并成主内核的 389 组。后续真正接入 Rust 的 57 组验证独立记录于[文本塑形](text-shaping.md)，不能把两阶段产物或测试数混用。

本轮同时构建不含故障注入接口的 profile 并独立执行同一 45 组对比。其 WASM 为 828917 字节、启动 JS 为 9257 字节；Native 静态库为 1601832 字节。静态库不是最终链接后的可执行增量；这些数字不含字体、Rust 主内核、完整宿主、图形、媒体等，不能据此推算 Musterwork 安装包。

## 可重复的开发命令

先按锁文件取得官方源码归档到 `.codex-work/harfbuzz/harfbuzz-14.5.0.tar.xz`，准备本地 Emscripten 6.0.10 SDK 和 LLVM `llvm-ar`。构建脚本在解压前验证归档长度和 SHA-256，不自行联网。测试字体按[字体资源步骤](font-resources.md)获取，Python 语料生成器需要 FontTools 4.61.1。

```sh
python3 tools/components/build-harfbuzz.py --target native --fault-tests --reference --archiver .codex-work/emsdk/upstream/bin/llvm-ar
python3 tools/components/build-harfbuzz.py --target wasm --fault-tests
.codex-work/font-tools-venv/bin/python tools/verification/harfbuzz-cases.py
node --expose-gc tools/verification/harfbuzz-parity.mjs
```

`--reference` 只用于开发，需 `pkg-config` 与 GLib；本轮版本为 GLib 2.86.4。组件本身不链接 GLib。ASan/UBSan 使用独立目录并复制同一已校验归档：

```sh
mkdir -p .codex-work/harfbuzz/asan
cp .codex-work/harfbuzz/harfbuzz-14.5.0.tar.xz .codex-work/harfbuzz/asan/
python3 tools/components/build-harfbuzz.py --target native --directory .codex-work/harfbuzz/asan --fault-tests --sanitize --archiver .codex-work/emsdk/upstream/bin/llvm-ar
python3 tools/verification/harfbuzz-safety.py
```

不含注入代码的构建去掉 `--fault-tests`，Native/WASM 使用同一 `--directory .codex-work/harfbuzz/release` 并先放入归档；对比使用 `--directory .codex-work/harfbuzz/release --skip-faults`。测试脚本会检查该模块没有注入接口。

## 下一步

Rust 类型化合同、字体绑定、轴精度、字簇核对和开发隔离宿主已见[后续实现](text-shaping.md)。继续推进字体选择、主题别名、script/itemization、bidi、断行、字形绘制、字号和页面布局。字体净化、彩色字形、完整 AAT/复杂字体覆盖、外部软件数值和视觉验收、硬取消与完整资源预算仍未完成。每一步都需要实际闭环，不能用本组件成功替代 E0/E1/E2/E3。

上游依据：[固定发行](https://github.com/harfbuzz/harfbuzz/releases/tag/14.5.0)、[构建配置](https://github.com/harfbuzz/harfbuzz/blob/14.5.0/CONFIG.md)、[分配钩子](https://github.com/harfbuzz/harfbuzz/blob/14.5.0/src/hb.hh)、[buffer API](https://harfbuzz.github.io/harfbuzz-hb-buffer.html)。来源许可与工具链限制见[依赖清单](dependencies.md)。
