# 单次解码的 PCM 分块与完成记录

2026-10-07。内核和真实文件夹 fixture 已验证；产品持久任务、授权缓存和音频设备接线继续实施。此增量不表示有声播放或完整本地验收完成。

## 目的与边界

长素材不能先解码一遍猜长度，再解码一次写 PCM，也不能把整段声音保留在内存。`PcmStreamBuilder` 从同一个新解码器连续接收真实帧，边检查边交出固定宽度的 canonical f32le 分块；成功 EOF 才确定完整范围和最终源身份。它与已知范围的 `PcmTileBuilder` 共用编码算法，产出的字节、SHA-256 和最终 `PcmTileRef` 完全兼容。

内核不接受文件路径，不落盘，也不持有租户、任务或登录权限。宿主负责原素材授权、固定 worker、隔离任务尝试、写入、取消、原子完成、恢复与回收。完成记录不是访问凭据；历史计算完成也不能使已删除的素材重新可读。

## 流式计算

1. `PcmStreamOrigin` 绑定原内容摘要、稳定流 ID 和 decoder profile。profile 必须包含固定 worker 身份、raw/demuxer 时间轴模式以及所有影响输出的选项。资源 ID 可随工程变化，不作为跨工程访问许可。
2. 每次 `push` 复用 `PcmInspection` 的 PTS、精确采样格点、实际格式、generation、连续性和有限数值检查，只标准化一次。当前帧与不足一个分块的尾部拼接，完整分块立即交给宿主。
3. `UnsealedPcmTileRef` 是临时引用，绑定 provisional origin、序号、起点、长度和实际字节摘要。它不表示整个源 ready，不能被工程播放或导出提前接受。
4. 宿主确认同一解码器成功 EOF 后调用消耗 builder 的 `finish_at_eof`。它生成 `CompletedPcmStream`，含真实 `InspectedPcm`、最终 `PcmSource` 和可能存在的最后短块。
5. `seal_ref` 验证 provisional origin 及最终几何范围，转换为原 reader 支持的引用。内核不能证明宿主已写入字节，宿主必须核对完整连续清单并原子提交。

任何解码/格式/预算错误、取消或检查回调 panic 都永久污染本次 builder，不能把此前前缀当成成功资源。宿主写入失败也必须放弃整个尝试，不能仅忽略失败块继续封口。索引/读取只引用原子发布后的完成清单；临时文件存在本身不构成完成。

### 内存与工作量

默认分块 4,096 个采样帧；单块允许 256～65,536 帧。默认最多 1,000,000 块、一次调用最多交出 1,024 块。分块数上限同时计入未完成尾块，避免最后 EOF 才发现超额。

默认 64 MiB 工作额度覆盖当前标准化块、保留尾部的 capacity 和当前调用交出的编码字节。分配标准化块前先扣除已有尾部额度。解码器输入、宿主 I/O、调用方保留的前次输出和清单元数据另计；这不是进程 RSS 承诺。宿主应立即写出并释放返回块，不能把所有块累积到 Vec 后宣称流式内存。

## 完成记录与重开

`PcmStreamRecord` 是有版本、拒绝未知字段的紧凑元数据，包含原内容、流、decoder profile、分块宽度、实际采样率、命名声道、精确样本零点、真实帧数、解码帧数和最终源摘要。它没有路径和业务权限。

宿主只可从自身**已提交、内容身份已验证且重新授权**的完成记录调用 `restore`。恢复时重新校验配置、范围、精确时钟、声道和数量预算，并重建/核对最终源摘要。任意 JSON、未提交临时记录或磁盘文件存在都不证明历史 EOF。正常 `PcmTileReader` 仍验证每次读取的几何、长度、SHA-256 和数值；损坏/缺块不会转换成静音。

恢复出来的 inspection 是经宿主确认来源的历史计算记录，不表示本次调用解码过内容。跨工程相同内容可重新绑定 resource ID，但产品不能用相同内容摘要绕过原工程/文件权限。

## 验证证据

- `mo-video-audio`、`mo-video-pcm`、`mo-video-runtime/native` 常规检查 65 项通过，4 项专用检查在该命令中 ignored；其中真实 AAC 用例另行显式执行，不能将其余 ignored 计为通过。
- 流式分块与已知范围 builder 在单/双声道、packed/planar、不规则输入分割、负数及分数采样相位下逐字节一致。覆盖短尾、整尾、非法引用、取消/panic、预算失败后禁止完成、丢块/篡改读取及完成记录重建。
- 真实 H.264/AAC fixture 经 raw worker 只解码一次，145,024 个原始采样帧写成 36 个文件，保存完成记录后释放原内核状态再读回；映射后的 144,000 个采样帧与新 demuxer 解码逐个比较 `f32::to_bits()`，包括 signed zero。5 个随机窗口一致。
- 实际原始 PCM 580,096 bytes；读取器缓存上限 2 块，测试期间实际读取 41 次。此处是文件 I/O fixture 的同进程重开，没有产品数据库、权限撤销或进程崩溃恢复，不能扩大结论。
- 三 crate all-targets strict Clippy、格式检查和 runtime 默认 WASM 编译通过；没有执行浏览器音频数值或实际音频设备测试。

固定 fixture 摘要 `ca7211a9b5e1bbdb66c7bd98e515484d7b6bc5ee8bbeeadb6cabb06d90bd73a6`，worker manifest `98c173a7934275de3fea8aa13c604c734fddc694029aab87c8470fc45b586a27`；映射后 PCM 摘要 `40b3b2bf3dd2790d5cb63752401e1b7a1559f1ce5280d6df1ff90bce65e46def`。细节见忽略的 `.codex-work/video-stream-record-bits.json` 和相邻日志。

## 复现与产品接入

```sh
cargo test -p mo-video-audio -p mo-video-pcm -p mo-video-runtime \
  --features mo-video-runtime/native --offline --locked
cargo clippy -p mo-video-audio -p mo-video-pcm -p mo-video-runtime \
  --features mo-video-runtime/native --all-targets --offline --locked -- -D warnings
cargo check -p mo-video-runtime --target wasm32-unknown-unknown --offline --locked
```

显式原生用例参数与[音频 movie 映射](video-audio-movie.md)一致。`MO_AUDIO_MOVIE_REPORT` 需为新路径，同名 `.tiles` 目录由用例独占创建。

`tools/video-sdk/check-audio-movie.py --streamed` 新增独立 registry 消费方式，以公开 SDK 完成单次解码、封口、记录序列化/恢复、movie 映射与 32 个随机窗口检查。它的分块宿主是内存 fixture，仅验证发行消费，不作为内存性能证据。固定发行身份和执行结果在真正打包验证后记录，不以工具存在作为通过证据。

干净源码 `f98c99b2a17eb0831ada73da353e14d6700f5614` 已构建本地 `0.1.0-video.20261007.5`，20 crate，manifest `58c31c3d95c67c1943c3a4c53fc2b43c7ea6c0035bede3f7c76bd179542eeff8`，`sourceDirty=false`、`releaseCleared=false`。独立消费者只复制 registry archives、index 和固定外部锁图，真实完成 36 块封口、record 恢复、144,000 个映射样本的逐位检查，以及 32 个倒序随机窗口；没有源码路径依赖或额外消费者依赖。报告 `.codex-work/video-pcm-stream-consumer-01/report.json`。没有公开发布、升级公共频道或重建 codec worker。

Musterwork 已在 `7e50a2f7f` 增加原有 SQLite/CAS owner 下的源授权、分块预写/原子完成、旧 writer 未完成尝试恢复和源删除关联；`c0d2b52e3` 固定消费 `.20261007.5` 并接通 `NativeVideoPcmHost`。原项目权限和任务存储仍由产品拥有，没有把这些职责迁入本内核。

产品真实检查新增 3 项、全部显式执行通过：AAC 物化的 145,024 个原始采样帧采用 Host 选择的 16,384 帧块宽，写成 9 块、580,096 bytes；与独立 raw decoder 的全部 f32 字节一致。关闭整个真实产品数据库后重开，复用原 completion/attempt 和全部字节；删除原源后拒绝访问。另验证调用方中断时保持物理额度到 worker 回收/清理完成，以及首块已落盘后取消不会发布前缀。产品 58 项不同存储检查、27 项安装检查、26 项 adapter 常规检查和生产库 strict Clippy 通过，专用 ignored 检查单独标明，没有混计。

这里返回的仍是 raw PCM，产品声音读取必须继续使用 `MovieAudioPlan`；后台调度、音频设备时钟、画面同步和冻结工程导出尚未接通。产品尚未执行本轮真实浏览器或登录 Electron 的声音验收。

## 产品按 movie 时间读取的集成

产品提交 `e4488c29b` 在相同固定 `.20261007.5` SDK 上接入原 CAS 到 `PcmTileStore` / `CanonicalAudioResource`，再对同一 verified source 建立实际 `NativeIndex` 和 `MovieAudioPlan`。`NativeVideoAudioResource` 在阻塞渲染 actor 使用，不把存储 I/O 放进实时音频回调；读取完成后索引 worker 已退出。

产品在每次逻辑读取前后重新检查当前源权限及精确完成记录，包含 SDK 内存缓存命中和 empty edit。SQLite v200 在任何派生块删除时原子停用整次缓存并回收其余块；迁移也处理 v199 遗留的已删块记录，失败时状态和 tombstone 一起回滚。该逻辑归产品所有，内核仍不依赖权限、数据库或业务 epoch。

5 项实际 worker/SQLite 用例显式通过：完整 144,000 个 movie 样本与独立 demuxer 的 f32le 字节一致，32 个随机窗口一致；预算拒绝先于字节读取；取消清空缓存；删除其他块使 warm cache 失效；读取前验证通过、读取后验证时源被删也不返回候选样本。原持久化整库重开和两种取消用例同时通过。51 项不同产品存储检查、26 项 adapter lib 检查和生产库 strict Clippy 通过，专用 ignored 未计入。证据为产品 `.codex-work/video-pcm-read-report.json`。

仍没有声卡输出、工程混音会话或音画同步验收；此接口也不代替工程 pinned revision 的校验。首次读取当前需完成原 PCM 物化，长素材可听延迟、后台准备/恢复/空间策略、冻结工程导出和浏览器声音继续实施。内核当前 unit-rate movie 路径仍可能保留未必必要的 sinc filter；性能不得以此结果宣称最优或全部达标。

## 产品工程混音会话

产品提交 `801f1726d` 将上述源接入生产 resident 的预览组合。画面与声音两个有界 blocking actor 共享同一不可变 `VideoProject` 和同一物理会话额度，使用原 scope/pin/authority；没有复制另一份可变时间线。音频通过内核 `AudioMixer` 处理，产品负责有界源缓存、明确的单声道到双声道矩阵、准备与回收。首次全源 PCM 准备改用进程级独立 1 个额度，因此不占据唯一编辑命令额度；关闭时等待两条队列和实际准备任务结束。

4 项实际 AAC 混音检查通过：单轨、双轨叠加、定位及取消恢复；48 kHz 素材到 24 kHz 工程的 4096 帧输出与分块结果一致；慢准备时画面和编辑仍可前进，旧 pin 结果被后校验拒绝；丢弃调用者后关闭仍持有额度至实际回收。另有 5 项保存工程预览、1 项实际 VFR 画面、2 项实际 Runtime 组件加载和 29 项安装检查通过，生产库 strict Clippy 通过。未将上述内部计算与真实组件加载扩大为已登录声音播放证据。

启动验证还发现并修正了产品 SDK/worker 配对漂移。新的产品 worker lock `/2` 分别记录原 worker 构建提交 `636ad723…` 与当前 SDK 提交 `f98c99b2…`，精确绑定 `.20261007.5` manifest；实际 worker 字节和 manifest `98c173…6a27` 不变，未伪造重新构建或公开发行。实际仓库 pin 检查已加入，避免仅测合成安装 fixture。

声音 protobuf/Electron 传输、设备采样率转换/播放时钟、音画同步、完整保持音高派生调度和多声道播放策略仍未完成。产品内部输出是工程采样率，不能靠前端每块独立转换声称已经保证连续播放质量。证据：产品 `.codex-work/video-preview-audio-report.json`，详细边界见其 `docs/architecture/video-preview-runtime.md`。
