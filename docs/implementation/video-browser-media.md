# 浏览器本地媒体读取

2026-10-08。对应 W2 / SYS-03 的增量实现，完整 Web 工作台尚未验收。

## 责任与接口

`packages/video-browser-media` 是 MusterOffice 的独立媒体薄接入层。它只接受宿主已授权的不可变 Blob，在宿主持有的 Worker 内执行容器探测和浏览器实际解码。它不接受 URL、任意文件路径、登录信息、工程写入或服务端上传。Rust 编辑与采样计划仍是唯一语义来源；此组件不实现第二套时间线 reducer。

- `open`：解析实际流，返回容器时间网格、首包/尾部时间、画幅、变换矩阵、像素比例、色彩、音轨及 codec 配置支持情况。配置探测不等于实际解码成功。
- `videoAt`：直接接受 Rust `TimeSample` 的有理数时间及 `before/after`。仅在精确命中容器时间网格且方向为 before 时查询前一 tick；不添加 epsilon。返回样本后再次验证精确覆盖区间，不把最后一帧延长到空白或 EOF 后。
- `videoRange` / `audioRange`：逐个回调并等待消费，返回完整源样本及有理区间。Rust DSP/计划执行器仍负责裁剪、变速、重采样和声道映射。音频的 `first` 是首包时间，可能包含负时间预热；不能直接当作影片可播放起点。解码尾包可能包含 padding，影片 edit list/有效区间需要独立应用。
- 样本是借用对象，回调完成立即释放；保留后的方法调用拒绝。GPU 消费可显式 `cloneFrame`，调用方承担该平台帧的关闭责任。`copyRgba` 明确转换成 SDR sRGB，不标成原始 YUV。
- 单个输入只允许一个活动操作，避免无界请求队列。取消/超时关闭整个输入，关闭借用帧并直接 return 上游迭代器；即使消费回调一直不结束，也不等待它推进循环后才清理解码队列。重新定位的替代 generation 使用宿主提供的同一不可变 Blob 创建新输入。

## 实际发现与修正

直接从目标时间调用通用音频 sink，会把 AAC 的 key 标记误当成“无前置状态即可正确恢复”。真实检查中，全片 RMS 差异为约 0.0053，明显超过门槛。修正为从真实首包恢复解码状态，再按精确样本区间选择输出。没有猜测固定编码延迟或调宽测试容差。音频 sink 已恢复 sample-rate 时间网格，因此保持该网格，不再次量化为微秒。

取消也不能只调用 input.dispose：当消费者不返回时，for-await 的清理可能尚未执行。当前由组件保存上游迭代器，取消时主动 return，并关闭当前借用帧；另有真实浏览器“回调永不完成→超时→资源回收”检查。

这些策略保障当前正确性，但**随机音频 seek 目前会从首包重建状态**。长素材应增加经过容器/codec 证明的恢复点或持久顺序 decoder cursor，不能把当前随机 seek 性能当作已完成。

## 固定依赖与交付

Mediabunny 1.61.3 / MPL-2.0，固定到研究时同一 upstream commit。容器与样本行为依据[官方 API](https://mediabunny.dev/guide/media-sinks)和[固定源码](https://github.com/Vanilagy/mediabunny/blob/9d36fc5779b254904bc609e66f2cd80ed8390b4b/src/media-sink.ts)检查。

构建独立 Browser SDK，不重新构建 PPT 或 authoring WASM。产物包含运行时 ESM、独立类型、源码记录、依赖原始完整 `src/` 与 `shared/`、许可和全量文件摘要。宿主只消费外部固定 manifest pin。构建与验证入口见[组件记录](../../components/video-browser-media/README.md)。

## 已验证

真实 Chrome 154.0.8037.98 / macOS arm64 / 专用 Worker，使用与原生检查相同的 H.264 B-frame/VFR + AAC 素材，SHA-256 `ca7211a9b5e1bbdb66c7bd98e515484d7b6bc5ee8bbeeadb6cabb06d90bd73a6`。FFmpeg 独立解码作为参照，不以浏览器自己的 seek 结果作为真值。

| 项目 | 实际结果 |
| --- | --- |
| 视频 | 77 帧；全部 PTS 与独立 ffprobe 时间基一致 |
| 定位 | 16 次正/反向边界查询；起点 before 无样本；EOF 不延长 |
| 像素 | 全部帧 sRGB RGBA 对比；平均通道误差 0.60354，最差单帧平均误差 0.65143（8-bit） |
| 音频 | 144,384 个解码采样值；RMS 约 `3.31e-9`，最大误差约 `2.24e-8`；包含编码尾部 padding，并不等于可播放区间样本数 |
| 随机音频窗口 | 非包边界的 1.011–1.121 秒窗口与完整解码对应位置相同 |
| 生命周期 | early break、消费异常、忙碌拒绝、关闭后调用、预取消、执行中取消、回调永不完成后的 deadline、像素额度拒绝；最终 decoder/frame/audio-data 活跃计数为 0 |
| 单元/交付 | 5 项检查；含 107,892 帧时间网格、负时间/非法时间与字节/额外文件/符号链接篡改拒绝 |

最终本地证据：`.codex-work/video-browser-media-final/report.json`；实际消费干净源码 `a7d306400a9e4e3d69ba96d49761653555590974` 构建的 SDK，manifest `5c41d9a3ac7bb8e6bee686f48227a488b7143e5214e3da04def2f061e3ef5cb1`，目录 `.codex-work/video-browser-sdk-final/`。运行时 ESM 308,619 bytes，gzip 82,003 bytes；包含完整对应源码/许可/类型的交付目录为 2,252,091 bytes、88 个 inventory 文件。这是文件体积，不是运行内存、启动耗时或全工作台下载量。

复现：

```sh
pnpm run test:video-browser-media
node tools/verification/video-browser-media-check.mjs BROWSER_EXECUTABLE BFRAME_VFR_FIXTURE NEW_OUTPUT_DIRECTORY SDK_DIRECTORY MANIFEST_SHA256
```

## 尚不能据此关闭的验收

1. 浏览器合成、影片 edit list/priming/padding 映射、音频 DSP、持久导出与正式 Web 页面/Agent 接线仍待接通。当前输入接口不能直接替代已验收原生 MovieTimeline。
2. 本次背压与截止检查真实观察到最高 **42 个平台 VideoFrame**，全部最终释放。上游按数量限制预解码，尚不是字节预算；4K、高位深、多素材同时解码的总内存上限不能宣布通过。需要按 codec 最低重排深度及宿主预算设计 byte-weighted 队列，并测实际峰值。
3. Blob 读取缓存、文件大小、轨道数、画幅、音频块及时间有限制；浏览器底层 codec 内存和容器元数据解析 CPU 仍需宿主 Worker 隔离/终止及真实恶意素材压力验证。操作内计时器不能替代宿主对 Worker 卡死的截止控制。
4. 只有上述 Chromium 环境证据，没有 Safari/Firefox/Edge、完整 HDR/alpha、所有采样率/格式或长片 A/V 性能结论。公开发行/SBOM 清关仍独立执行。
