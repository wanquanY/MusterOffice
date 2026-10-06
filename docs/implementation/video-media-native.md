# 原生媒体执行边界

2026-10-06 · 开发中。已完成固定 C 解码组件、Rust 安全接入及可终止 worker；工程采样执行和产品集成尚未完成。

## 组件和职责

`components/ffmpeg` 固定源码、构建档案、许可与窄 C ABI；[组件记录](../../components/ffmpeg/README.md)说明 codec/格式边界、真实解码检查及分发要求。`mo-media-ffmpeg` 是 Rust 原生叶子适配器，不依赖 PPT/OPC，也不接受文件路径、URL、业务模型或 Agent 凭证。

宿主提供固定长度、内容不可变的 `ReadAt` 能力及取消检查；长素材不要求整文件放入内存。Rust 包装只拥有这些能力和当前 demux/decoder 状态，不拥有用户存储。浏览器实现不会加载此原生 crate。

## 生命周期和失败语义

- `Container::open` 保存 box 稳定地址的回调状态，读取有界流信息；头信息未知字段为 `None`，不按平均 FPS 合成时间戳。
- `Container::select` 消耗容器并返回 `Decoder`。一个会话只能选择一个流；音视频使用各自会话。类型系统拒绝重复使用已选择的容器。
- `Decoder` 明确为 `!Send + !Sync`，所有 C 调用和回调在拥有线程同步执行。
- `next_frame` 返回 `Some(DecodedFrame)`、排空后的 `None` 或明确错误。帧有独立拥有的 active plane 字节，平面连续行距等于 `row_bytes`；不暴露 decoder 指针或 padding。旧的已返回帧不会因下一帧、seek、关闭而失效。
- 复制前复查各平面乘法、聚合字节预算和地址空间转换，使用 fallible reserve；任何平面失败时丢弃整个候选帧。音视频没有隐式颜色、像素、采样率、声道或音量转换。
- 成功 seek 由 C 层递增 generation 并清空解码状态；它返回此前随机访问点之后的帧，不承诺第一个返回帧就是目标呈现帧。上层仍须使用精确 before/after 采样与 VFR 呈现区间选择。
- 任一操作错误立即关闭 native context。之后 next/seek 返回 `State`，不能将失败混淆为正常 EOF。Drop 再次执行时不会 double close；回调状态保持到 native close 完成后才释放。

`ReadAt` 可以短读，不能在声明的资源末端之前返回零或超出目标容量。回调返回异常数值或 I/O 错误都会失败。C 分配的 AVIO 缓冲可能未初始化，因此包装在形成安全 Rust `&mut [u8]` 前初始化目标缓冲；安全回调允许读取它，不能把未初始化内存交给 safe Rust。

read/cancel 回调用 `catch_unwind` 拦截 Rust unwind，记录首次失败并阻止后续回调；I/O 错误与 panic 不穿过 C ABI。panic payload 的析构也可能 panic，因此首次 panic payload 被有意遗忘，每个失败会话至多一份，最终进程退出回收；这不是长期复用失败会话的理由。`panic=abort`、内存耗尽或本机崩溃仍可能终止进程，由独立 worker 层处理。宿主回调必须及时返回；包装不能抢占一个不返回的同线程回调。

## ABI 与固定构建

`MO_FFMPEG_LIB_DIR` 指向组件根目录（含 `native-build.json`，不是 `install/lib`）。构建脚本核对当前 source lock、C/header 清单及摘要、独立/内嵌上游构建档案、平台、非 sanitizer 档案、库和安装头摘要。还核对实际 unversioned/major loader alias 的目标属于已验证库，避免正确文件旁的错误软链接被链接器选中。

构建期间 C 编译器测量固定头文件的每一个字段 offset、结构体 size/alignment，生成 Rust 编译期断言。运行时再检查 wrapper ABI 1 与 FFmpeg 9.0.2 版本。当前不接受未核实的交叉编译目标；没有系统 pkg-config/Homebrew 回退。

本 crate 自身测试使用开发目录 rpath。下游可执行程序须消费 `DEP_MO_MEDIA_LIBRARY_DIR` 构建元数据并设置自己的开发/发行 rpath；Cargo 的 `rustc-link-arg` 不会自动传递给其他 crate 的可执行程序。最终发行仍须提供固定依赖闭包、来源/许可与可替换动态库方案，不把个人开发路径发布给用户。

## 已实际验证

- 8 项 Rust 所有权/错误测试：真实构造 PCM WAV 逐字节往返、13 字节短读、拥有帧跨 seek/close 存活、source 恰好 drop 一次、非法回调长度/提前 EOF/I/O 错误、read/cancel panic（包括 C 内部调用点）、会 panic 的 panic payload、逐个解码取消检查点及帧字节预算。
- 3 项 compile-fail 文档测试：不能 Send、不能 Sync、不能重复选择同一容器。
- runner 生成真实 H.264 B-frame/VFR + AAC 素材，另执行显式 ignored 的 Rust 测试。77 帧的 active pixel SHA-256/PTS 与独立 CLI 一致；前后 seek generation 与 suffix/完整序列一致；BT.709 元数据保留。
- AAC 共 144,000 样本，PTS 连续；与独立参考 PCM 最大绝对差 `2.24e-8`、RMS `3.32e-9`。参考旧版 CLI 的尾 padding 处理沿用组件记录中已知源长度的明确规则。
- `current.py --groups media` 顺序执行 C、Rust、worker 构建及跨进程实际检查，Rust 检查绑定本次 C evidence 的素材、组件摘要，不用预录媒体输出替代执行。

```sh
MO_FFMPEG_LIB_DIR="$PWD/.codex-work/ffmpeg-native" cargo test --locked --offline -p mo-media-ffmpeg
python3 tools/verification/current.py --groups media \
  --ffmpeg-dir .codex-work/ffmpeg-native --output .codex-work/new-media-check
```

解码真实素材测试默认 ignored；上面的 media 分组必须显式执行它，缺少真实素材/组件时失败，不以 skipped 计作通过。本机证据 `.codex-work/media-rust-gate-final/report.json` 及其 `media-rust/report.json`，macOS arm64；没有新增 Linux/Windows、浏览器或性能验收结论。

## 进程限制基础

后续已实现独立 `mo-native-limits`，用于 disposable worker 启动时安装并核验进程 CPU、地址增长、文件描述符和文件增长上限；实际子进程故障测试已通过，详见[限制语义与证据](native-worker-limits.md)。现已接入[视频 worker](video-media-worker.md)，在接收素材前安装并由宿主核验；它不是文件/网络沙箱或 RSS 硬限额，不能据此把完整产品执行链标为完成。

## 编码能力

后续新增了类型化 `Encoder` 与候选 `WriteAt`，构建身份也校验固定上游修改。编码、封装与错误路径已有实机证据；详见[原生编码记录](video-native-encoding.md)。编码的真实文件测试另由 `encode` 分组及显式硬件检查执行，仍不代表已接入 worker 或完整工程导出。

## 下一层必须完成

可终止进程、双向资源读取、有界 IPC、总调用时限和进程额度已在 worker 层实现并实测。下一层需要使用实际帧建立呈现索引并执行工程采样；原生/浏览器共享渲染与音频处理、编码导出、产品 SDK 和界面。当前 Rust 安全 API 不等于敌意输入隔离，不能把它直接放到 UI 进程里当成已完成的产品边界。
