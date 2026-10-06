# 保调变速：顺序 DSP 与跨端接入

2026-10-06 · 已实现连续有限源保调计算、Native/WASM 接入和工程派生图；最终产品验收仍未完成。

## 职责和选型

`mo-video-audio::StretchStream` 持有采样调度、整数时钟、有限源边界、预热/尾部和错误语义。`StretchEngine` 是窄计算能力；`mo-stretch-sys` 对接原生 C++，`mo-stretch-wasm` 对接单独 WASM 实例。`packages/stretch-component` 只管理组件内存和函数调用，不重写时间轴、速率或内容语义。无业务平台、文件、URL 或音频设备依赖。

实现采用已在方案中列为候选的 Signalsmith，固定版本、MIT 声明和小范围集成变更详见[组件记录](../../components/signalsmith-stretch/README.md)。依据是其公开的相位声码器接口和输入/输出延迟模型；并不代表所有音乐/语音/极端速率的主观音质均已验收。

## 精确调度

当前处理一个连续有限源，输入/输出关系为严格递增的分段有理映射 `f(k)`，每段斜率在 `[1/64,64]`；恒定速率为 `f(k)=k*S/T`。来源重采样、声道解释和反向方向由[工程变速图](video-audio-retime.md)决定，此层不把输入采样率变化误当成保调。

1. 首次读取 `[0,L_in)` 并执行一次 `begin/seek`。来源短于输入延迟时只对声明范围之外补零。
2. DSP 的处理输出时钟为整数 `k`。输入边界严格为 `L_in + floor(f(k))`，使用有界精确有理数运算，不累计浮点速度误差。传给 DSP 的浮点速率由 Q32 下取整生成，只用于初始估计。
3. 内部输出量子最多 128 帧，在中间锚点的上取整边界切开；最后一块截断至 `T+L_out`。输入不足的尾部明确补零，处理整个有限源及尾部影响。
4. 去掉前 `L_out` 个输出，只发表恰好 `T` 个样本，首块可以小于 128。读取/播放器/声卡回调不改变 DSP 分块边界。
5. 不调用上游 `exact()` 的过短素材静音分支。1、16、128 帧短素材均实际产生非零结果，但极短素材不等于良好感知音质。

这种零扩展定义是本 profile 的有限源语义；没有宣称与上游 `outputSeek/flush/exact` 的首尾窗算法逐 bit 相同。只因输出长度正确不能证明源事件对齐或主观音质；后续完整媒体测试仍需检查语音瞬态、节拍和首尾可听内容。

## 连续状态、重试与发布

DSP 状态跨量子保留。宿主的源窗口尚未就绪时，读取返回错误；该次对应的 DSP 还没有执行，可以填好素材后重试，不重复 `begin` 或跳过输入。已经完成的预热步骤可保留，尚未有可发表的音频块。

DSP 调用失败、DSP 输出非有限，或状态推进后输出因取消/分配失败而丢失时，流永久失效。不能把下次输出当成丢失块继续拼接。宿主应丢弃该派生任务的未提交候选，从有效连续起点重建；最终 UI 取消无需自动重试。

每个成功块携带从零开始的确切 first frame。宿主逐块写入并原子发表派生资源，不能将一次纯计算成功视为持久化成功。此前的标准源 PCM 瓦片解决的是解码历史；保调后的派生瓦片通过工程层绑定完整变速图、数值实现 profile、上游资源和工程效果依赖，不能冒用原始 PCM 身份。

## 预算与故障边界

- Rust 预先准入 scratch、候选输出和完整预热工作量，最多 128 MiB scratch 额度。返回后由调用者持有的旧块另计。
- DSP 使用实例分配器，vector capacity 纳入预算；没有全进程 allocator 覆盖。所有 C++ 异常在 C ABI 边界内处理。
- 原生 safe wrapper 独占对象；WASM Rust 借用宿主组件，宿主只给一个流使用并负责释放。浏览器设备/AudioWorklet 不应直接执行该同步计算，应消费 Worker 已准备的数据。
- WASM 无文件/网络导入，只允许内存增长与 abort。TS adapter 使用固定 scratch，计算后重取 heap view，防止内存增长后使用失效 view。
- 真实取消逐检查点、构造分配故障、宿主引擎 panic 后失效和 ASan/UBSan 单独测试；这些不等于完整进程/浏览器资源隔离。

## 当前实测

| 检查 | 结果 | 范围 |
| --- | --- | --- |
| Rust 实际原生行为 | 7 项通过 | 保调/重复、极短/尾部、非法输入、取消每个观察点、预准入预算、panic 后失效、最大整数时长 |
| 跨端真实 DSP | 12 组 Native/WASM 全 PCM 最大误差和 RMS 均为 0 | macOS arm64 Apple Clang 17；Emscripten 6.0.10；Node v23.5.0 |
| 跨端语料 | 0.5×、2/3×、0.75×、1.5×、2×、1/3×中的对应长度档案，双声道、44.1kHz、静音、尾部、1/16/128 帧短音频 | 实际 fixture 固定输入/输出长度；不是完整随机速率/全部格式覆盖 |
| 音高/能量 | 独立正向过零计数距 440/990Hz 小于 3Hz；中央 RMS 在 0.15–0.26 | 输入为幅度 0.3 的合成音，不是主观试听评分 |
| 暂时缺源 | 每次源读取前故意失败一次再重试，两端均与无失败输出逐样本一致 | 不包含持久 Host 断电恢复 |
| 内存/未定义行为 | 原创边界及所用头文件 ASan/UBSan 实际执行；35 个构造分配点逐个故障；192,000 输出帧，峰值组件计数 912,064 bytes | 48k 双声道测试配置；不是总 RSS 或所有配置最大值 |
| 非有限计算 | 最大有限 f32 输入触发 DSP 失败，后续计算拒绝 | 不伪造零输出成功 |
| 开发构建体积 | 本机静态原生归档约 92 KB；DSP WASM 67,682 bytes + JS 7,660 bytes | 不含 Rust 模块、C++ 系统运行库、Host 和整个应用，不是最终发布体积 |

跨端门槛事先设为最大误差 `2e-4`、RMS `2e-5`，当前语料实际达到 bit 等价的数值输出。不同编译器/CPU/标准库尚未验收；派生缓存必须包含实际数值构建 profile，不将该实测推广为跨平台任意声音逐 bit 一致的承诺。

复现：

```sh
python3 tools/verification/current.py --groups stretch \
  --output .codex-work/stretch-gate-final \
  --stretch-dir .codex-work/stretch-native \
  --stretch-wasm-dir .codex-work/stretch-wasm \
  --bindgen .codex-work/toolchain/bin/wasm-bindgen

python3 tools/components/build-stretch.py --target native --sanitize --fault-tests \
  --stretch-source .codex-work/signalsmith-stretch-1.4.0 \
  --linear-source .codex-work/signalsmith-linear-0.6.4 \
  --output .codex-work/stretch-asan --archiver .codex-work/emsdk/upstream/bin/llvm-ar
.codex-work/stretch-asan/stretch-probe
```

## 工程接入与剩余验收

`AudioMixer` 已通过连续派生资源调用保调结果，支持同向分段速率、嵌套政策、裁切/分割上下文、派生 PCM 随机读取和依赖失效，见[工程保调图](video-audio-retime.md)。缺少该能力的旧 Host 继续返回明确的 Unsupported；已接入 Host 的待准备窗口返回 PendingDerived。

生产后台准备、临时/磁盘缓存总额度、设备播放、真实语音音乐/瞬态和极端速率听感仍未验收。算法及测试 Host 通过不代表用户已经可在工作台使用“保持音高”。
