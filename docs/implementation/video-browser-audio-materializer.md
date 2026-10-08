# 浏览器影片音频物化接入

2026-10-08 · `BrowserMediaInput.materializeMovieAudio` 已实现并经过实际 Worker / WebCodecs / 独立 WASM 检查。产品 OPFS 保存、播放与导出尚未完成；本记录不代表整个工作台已可验收。

## 正式组件接口

调用方注入独立固定交付的 index runtime、PCM runtime、已经验证内容摘要的不可变 Blob 和同步暂存能力。`materializeMovieAudio(trackId, pcmRuntime, origin, writeTile, signal)` 从同一素材原始包连续解码到 EOF，返回独立拥有资源的 `BrowserPcmCompletion`。`origin` 只包含素材、内容摘要、流及宿主 decoder 指纹；track、格式和原始时钟由实际输入获得，额外客户端字段不能覆盖它们。

`BrowserMovieIndex` 共用原有有界包分页，并将音轨原始 edit/时长元数据交给 Rust。没有在 TypeScript 重写影片时间算法，也不再使用会重新对齐首个输出时间的第三方音频 wrapper。AAC 的实际 priming/padding 输出时间保持原样。

完成对象包含 `options`、`clock`、`record`、`movieInfo`、实际包/块/tile 数和 `seal` / `close`。它只证明本次计算成功经过同一个 decoder 的 EOF，并通过 Rust 影片覆盖校验。宿主仍须密封全部引用、确认暂存数据耐久、原子发布记录和引用，才能把它当作已保存资源。输入关闭后，成功交付的完成对象继续有效；调用方必须最终 `close`。

同步写回调必须返回 `true`。异常、Promise、false、尾部写入失败或取消使整个 attempt 失效；原始宿主异常保留。取消从写回调重入时，不在仍借用中的 Rust 栈上 free；在调用退出后释放。完成对象刚生成、尚未交到调用方时发生的取消，也会释放对象和预算，不能遗留完成资源。

## 容量与生命周期

`BrowserMediaBudget` 是音视频共用的原子准入账本；旧 `BrowserVideoBudget` 是同一个构造器的兼容导出。输入禁止同时注入两个不同账本。物化前退役该输入已有的音视频游标。

当前平台档案为 AAC-LC、正常频率索引、单/双声道、8–192 kHz。使用固定 Mediabunny 对应源码中的 ASC 解析器，核对 core object、SBR/PS、声道、实际输出频率和 GASpecific 的 960/1024 帧标志；未支持配置明确拒绝。不是任意 AAC 码流容量证明，其他格式仍在完整目标内。

- PCM 计算工作权重为 4 MiB 加元数据/carry；影片完成校验另预留 12 MiB，校验后释放。
- codec 状态采用明确应用权重，加有界 pending 包对应的输出与复制权重。默认 16 包，允许 2–256；由实际后续输出推进队列，不把 `decodeQueueSize` 视为尚存解码资源的总量。
- 原始包在 Blob 读取前预留输入和浏览器复制额度。首包必须成功准入及读取才创建/配置 decoder。
- 只在真正 EOF `flush`。中途 flush 会改变已观察到的实际 AAC 波形，不能用来实现反压。
- AudioData 复制后立即 close；实际输出格式和帧数在复制前检查。完整时间连续性与 PCM 值继续由 Rust 校验。
- Input.close / cancel / timeout 立即关闭 decoder，但尚未返回的 Blob 读取继续占有额度直到真正退出。成功完成对象持续占有计算额度，直到其所有者 close。

账本是应用准入，不是浏览器内部、驱动或 WASM 线性内存的硬上限。素材/索引既有基线、宿主持久存储、调用方保存的 tile 和旧 `audioRange` 不因新增接口自动纳入此额度。同步 Worker 调用的硬终止与跨任务总调度仍由产品负责。

## 验证口径

实际 Chrome 154 / macOS arm64，使用交付包的公开声明文件先编译跨组件消费者，再运行 Worker 内真实 AudioDecoder、AudioData、原始索引 WASM、PCM WASM。测量代理只记录真实调用，不替换解码器。

48 kHz 单声道及 44.1 kHz 双声道三秒素材分别产生 142 / 131 个原始包。正常操作均只创建一个 decoder、一次 EOF flush；实际 AudioData 峰值 1。原始与影片 PCM 对独立 FFmpeg 7.1.1 的误差分别约为 RMS 3.32×10⁻⁹ / 3.84×10⁻⁹，峰值 2.24×10⁻⁸ / 2.98×10⁻⁸。五个随机窗与顺序读取逐位相同。

真实故障检查包含输入关闭后完成对象仍可密封、持有完成对象时共享预算竞争者被拒绝、嵌套操作 busy、写异常及原输入重试、Promise 写拒绝、写入内重入取消、完成交付竞态取消、首包超限在 configure 前拒绝、未完成 Blob 读取期间关闭/超时仍计账、故障后新源恢复。所有最终 decoder、AudioData、PCM handle 和准入额度均归零。测试宿主暂存使用内存，不代表产品 OPFS 提交。

原有影片视频游标回归核对 5 个实际素材、1,402 次影片查询和 1,360 次像素查询；正向语料共 6 次 decoder 创建 / 361 包，结束无保留资源。原音频窗口和视频接口也经过真实解码回归。配置/声明/交付边界单测共 10 项通过。

```sh
pnpm run test:video-browser-media
node tools/video-browser-sdk/build.mjs build NEW_MEDIA_SDK
node tools/verification/video-audio-materializer-check.mjs BROWSER MEDIA_SDK MEDIA_PIN PCM_SDK PCM_PIN INDEX_SDK INDEX_PIN NEW_EVIDENCE THREE_SECOND_AAC_FIXTURE
```

PCM 和索引固定组件见[共享 PCM](video-browser-pcm.md)。尚须完成产品不可变消费、Worker OPFS 暂存/原子提交/恢复、设备播放及音画同步、浏览器合成与导出、其他格式、长素材和浏览器矩阵验收。
