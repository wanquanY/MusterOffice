# AVC 帧解释与随机访问参数恢复

2026-10-08 · indexed movie 路径增量，不是完整工作台验收。

## 共享语义

新增纯 Rust `mo-media-avc`，接受 avcC、access unit 或显式 `ReadAt`，不依赖解码器、路径、网络。原创实现参考 [ITU-T H.264 (08/2024)](https://www.itu.int/rec/T-REC-H.264-202408-S/en) 的 6.2、7.3、7.4、Annex E。该版已被后续版本替代，本组件不宣称支持后来扩展；没有复制第三方解析器。

每个 slice 按 PPS ID 查实际 SPS，保留参数 revision、profile/level、位深、色度格式、编码/可见尺寸、场标志、VUI 颜色声明及 decoder buffer 限制。多个 slice 必须属于同一 picture；未定义引用和跨 picture 混装拒绝。参数替换按 access unit 原子提交，尾部损坏、取消、读失败不污染后续状态。

缺省颜色保持 unspecified 与 present 标志，不按分辨率推断 BT.709。420 的缺省位置遵守标准，六种显式位置均保存；422 按格式语义处理，444/单色不伪造偏移。场图像、分离颜色平面、不同上下场位置不能当成同一 progressive 解释。

这不是完整 bitstream conformance 验证器。SPS 解析至 VUI/HRD 和结尾；PPS 只解析引用与 picture boundary 所需前缀，熵编码、FMO、slice payload 和 SEI 效果仍由解码器或专门语义实现负责。头部解析成功不能代替真实解码。

## 索引与预算

`PortableIndex` 按 sample decode order 延迟扫描 avc1/avc3，缓存结果连续段和参数变化 checkpoint；description 切换从对应 avcC 初始化。保留最初状态，向前/向后/重复访问不会把当前参数套给历史帧。

| 公共方法 | 语义 |
| --- | --- |
| `scanAvc(json, readAt)` | `{track,through,count}`，最多 2048 samples，返回 `{scanned,ready}` |
| `avcSample(json)` | `{track,sample}`，返回 description、picture facts、可用的 progressive `chromaLocation`；未扫描失败 |
| `avcBootstrap(json)` | 同上，返回目标 sample **之前**的长度前缀 SPS/PPS `Uint8Array`，目标包内更新仍按原序生效 |

`track` 是 zero-based ordinal。readAt 必须对应 open 的同一不可变源；不保留 JS 回调。所有 track 共用累计 32 MiB 帧头读取/4,000,000 NAL 扫描额度。浏览器每批最多 128 samples，批间让出事件循环；同步卡死仍由宿主 terminate Worker。

单个 access unit 最大 64 MiB/4096 NAL，参数集最多 65535 bytes，活动参数合计 1 MiB。slice 仅读最多 64-byte 前缀，图像载荷跳过。读取额度失败不退还。缓存另受 `maxMetadataBytes` 限制，和容器元数据额度独立，默认各 32 MiB；snapshot 对共享 Arc 字节保守全额计账，Vec 扩容和固定状态另留余量。它们不是进程 RSS 保证。metadata/bootstrap 查询不再次读取源。

## 实际浏览器接入

`IndexedVideoPackets` 使用共享 metadata 判断 IDR，移除该路径独立的 TS NAL 类型循环。启动 decoder 时在已验证 IDR 前补入 checkpoint 参数，所以 IDR 未重复 SPS/PPS 也能恢复历史替换。前缀及合并缓冲在分配前申请共享媒体预算；没有扩大默认预算掩盖复制。

`movieVideoAt` 按精确 sample ordinal 返回只读 `codecMetadata`，核对解码可见尺寸并拒绝未支持的场解释。原有 raw PTS 游标承担 B-frame 输出对应，不把输出次序当成 decode order。旧 packet-clock API 不编造 metadata。

GPU gate 已改用 `sample.codecMetadata.chromaLocation`，取消写死的 `left`。`VideoColorSpace` 仍来自实际帧；VUI/container/SEI/用户解释的完整冲突和优先级策略待接入共享解释层。当前 media adapter 仍仅接纳已建立容量模型的 progressive AVC、单 description；索引支持 description 切换不等于平台已支持动态容量扩大。其他 codec、场重建、ICC 和跨端颜色策略分别验收。

## 验证

- 8 项头部测试：六种位置、缺省/显式颜色、非首个 SPS、PPS→SPS、原子更新、bootstrap、picture 边界、截断、短读、超额、取消/回调失败。
- 3 项索引状态测试：分批、反向、description 切换、损坏后早期结果与重试、缓存/请求额度。
- 既有 ISO BMFF 精确时间检查、严格 TS/Clippy、10 项浏览器媒体单测继续通过。
- Chrome 154/macOS arm64 的 Worker/FileReaderSync：原生与 WASM metadata/bootstrap 完全一致，独立 FFprobe 帧/包验证；77 帧 B/VFR，加 7 个 24 帧素材，覆盖全部位置及中途 SPS/PPS 更新。
- `avc-metadata-fixtures.py` 生成测试画面，第二段同时改变位置和 CABAC PPS，后续 IDR 删除重复参数。正式 gate 增加直接晚起始、回退和中间位置的全新 decoder，逐帧摘要必须与顺序解码相同。
- 真实 GPU gate 对照已有独立验证的 native linear RGB，77 帧加 3 次回退最大线性误差 `3.577e-7`。不据此宣称速度、零拷贝、正常产品页音画播放或整体验收。

```sh
cargo test --offline --locked -p mo-media-avc -p mo-media-isobmff
cargo clippy --offline --locked -p mo-media-avc -p mo-media-isobmff -p mo-media-index-wasm --all-targets -- -D warnings
pnpm run test:video-browser-media
python3 tools/verification/avc-metadata-fixtures.py FFMPEG NEW_CORPUS
node tools/verification/media-index-browser-check.mjs BROWSER INDEX PIN NATIVE_QUERY NEW_REPORT FIXTURES...
node tools/verification/video-browser-movie-check.mjs BROWSER MEDIA MEDIA_PIN INDEX INDEX_PIN NATIVE_QUERY NEW_REPORT FIXTURES...
```

最终干净组件 pin、复验及产品安装单独登记。Stable v0.1.0 不变，无新公共发行。

## 固定开发交付

干净源码 `a28ca4c55511f66d6c71a456f1cfa80237e317ea`：

| 组件 | 清单 SHA-256 | 体积 |
| --- | --- | --- |
| media | `c1a953b6f1c833fa147cdf008897edc20790ef8c42124c089fc9770d3a305984` | 92 files，2,293,669 bytes，包含未修改 Mediabunny 对应源码/许可证 |
| index | `6c88864cc1cdf4a2b45243e0246a6bd00d7a3e55c41c1e475da42cc56872e313` | 11 files，615,081 bytes；WASM 572,552 bytes |

index WASM SHA-256 `53b4e36861698bbf90f6ab2b52e710e614a4cfee59ae8d50a911235fe95cf7fe`。独立 build/verify 均通过，`releaseCleared: false`。

最终实际报告：`.codex-work/avc-index-final-check/report.json`（8 素材、586 查询、原生/WASM/独立 metadata 一致）；`.codex-work/avc-movie-final-check/report.json`（4 素材、1,044 影片查询、1,020 普通画面核验，另 12 次全新 decoder 定位，16 MiB 受控预算内峰值 10,096,821 bytes，结束归零）；`.codex-work/video-decoded-avc-final-check/report.json`（80 次实际 GPU 颜色对比，最大线性差 `3.5762786865234375e-7`，受控媒体/GPU 账本结束归零）。

Musterwork 已通过固定五组件安装器更新 media/index，PCM/mix/visual 保持原 pin；Product Host 类型检查、16 项持久化/PCM 测试与五组件重新安装/篡改/符号链接检查通过。未据此宣称正式页面的联合音画体验已经完成。
