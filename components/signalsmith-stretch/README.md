# Signalsmith 保调计算组件

2026-10-06：已接入有界 Native/WASM 开发组件与 Rust 顺序调度；尚未完成工程变速图、随机访问派生缓存和产品播放器接入。

- Stretch 使用上游 `1.4.0` 标签对应提交 `a670068d9aeb64913331d5cc29337b19a457a7df`。
- Linear 使用该标签 CMake 文件声明的 `0.6.4`，提交 `de55e6a50ffcf6f8f43f649692d94691c7025151`。
- [锁文件](lock.json)逐个固定实际使用的三个头文件和两个 MIT 声明的长度/SHA-256。上游源码保留在构建输入目录，不迁入原创源码树。
- Stretch 标签内的 `SignalsmithStretch::version` 仍为 `{1,3,2}`；不能用该静态字段冒充标签或构建身份。发行/缓存身份使用源码摘要、集成 profile 和实际构建档案。
- 来源：[Stretch 仓库](https://github.com/Signalsmith-Audio/signalsmith-stretch/tree/a670068d9aeb64913331d5cc29337b19a457a7df)、[Linear 仓库](https://github.com/Signalsmith-Audio/linear/tree/de55e6a50ffcf6f8f43f649692d94691c7025151)。算法属于上游，MusterOffice 原创部分为调度、受限接入和产品工程语义。

## 集成 profile

`musteroffice.stretch.portable-f32/1-draft` 使用上游浮点 FFT/STFT，不启用 Accelerate、IPP、PFFFT、XSIMD 或平台自动探测。禁止 fast-math 和 FMA contraction。默认窗口为采样率的 120 ms，间隔 30 ms；关闭 split computation。在 48kHz 下输入/输出延迟均为 2880 样本。

构建时仅执行两类逐字核对的源码替换：

1. 将三个头文件中的 `std::vector` 换为使用实例预算的同一标准容器。每个文件替换数量固定；不拦截全进程 `new`，不改变 FFT/相位算法。
2. 将一处 `uniform_real_distribution` 换为固定数值映射；随机引擎选用标准规定序列的 `minstd_rand`，种子固定。避免不同 C++ 标准库产生不同随机映射。

准备后的每个文件、原创 wrapper、构建脚本、编译器、命令和产物摘要进入 `build.json`。Rust 原生绑定和 WASM 验证在消费前核对记录；不得接受修改后的旧二进制记录。

## 所有权与失败

[私有 C ABI](mo_stretch.h)只接收有限长度的交错 f32 PCM，不访问路径、网络或设备。调用者保证指针有效、输入/输出不重叠、实例串行使用。C++ 异常被捕获；分配失败或非有限 DSP 输出使实例永久失效，销毁后重建。非法长度、未初始化、重复 begin、非有限输入在状态推进前拒绝。

实例预算最多 128 MiB，包含 instance/engine 对象和全部 vendor vector **容量**；不包含系统分配器元数据、异常运行库、调用者 PCM、线程栈和整个进程 RSS。Native 对象不得在多个线程共享；WASM 每个组件实例独占一块线性内存，最大 256 MiB，栈 1 MiB。TS adapter 另分配固定输入/输出暂存区并独立报告，当前是复制接入，不宣称零拷贝。进程/Worker 的总任务限制仍由 Host 承担。

`begin` 只允许一次，先提供 inputLatency 长度的输入历史；`process` 每次最多 65536 输入帧和 4096 输出帧。此 ABI 没有“随意 seek 后继续输出与连续计算完全相同”的承诺，也不自行决定剪辑时长。

## 本地构建

先取得上述固定提交，以下命令只读取并核对已有源码，不下载或静默换版本：

```sh
python3 tools/components/build-stretch.py --target native \
  --stretch-source .codex-work/signalsmith-stretch-1.4.0 \
  --linear-source .codex-work/signalsmith-linear-0.6.4 \
  --output .codex-work/stretch-native \
  --archiver .codex-work/emsdk/upstream/bin/llvm-ar
python3 tools/components/build-stretch.py --target wasm \
  --stretch-source .codex-work/signalsmith-stretch-1.4.0 \
  --linear-source .codex-work/signalsmith-linear-0.6.4 \
  --output .codex-work/stretch-wasm --emsdk .codex-work/emsdk
```

原生开发绑定默认读取 `.codex-work/stretch-native`，可用 `MO_STRETCH_LIB_DIR` 指定。WASM 使用已选 Emscripten 6.0.10 开发工具链，相关供应链/二进制发行声明仍遵守[既有工具链限制](../../docs/implementation/dependencies.md)。本组件不把这些开发产物当作已发布 SDK。

完整边界、调度算法与实测见[保调实施记录](../../docs/implementation/video-pitch-stretch.md)。
