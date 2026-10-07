# 视频媒体运行时 SDK

2026-10-07 · 不可变开发档案，未完成生产发行与产品媒体验收。

## 公共入口与依赖

源码 `bfc939886f6ace222dc4a1759a6a5a2827bbf216` 新增 `mo-video-runtime`，profile 为 `musteroffice.video.runtime/1-draft`。入口重导出现有 authoring、compile、source、render、audio、PCM、export 类型，保持一个 Rust 类型图，不新增文档/任务/存储系统。只做 metadata 编辑的用户继续使用 `mo-video-operations`。

默认 features 为空；`native` 显式启用 worker adapter 和原生源采样；`gpu` 显式启用 GPU adapter。codec 不链接进 SDK，由独立固定 worker 接收 ReadAt/WriteAt 字节能力。宿主负责验证 release、资源权限、持久化、设备和候选发布。默认 facade 已通过 WASM 编译，这不是 WebCodecs/WebGPU 的实际运行证据。

该次 `tools/video-sdk/rust.py --profile runtime` 对 19 个自有 crate 构建独立 registry 包，保留所有可选和平台生产依赖；不纳入 tests/bin/build script/codec binary。既有 `operations` 档案保留原 6 包闭包。验证器要求精确 profile/root、完整且无重复的包清单、源码 clean、实际文件/index 和每个 SHA-256。后续源码 `3c95c60` 因受限容器索引把 runtime 审核闭包增至 20 包并要求 worker profile 3；`3c0cf1a` 增加精确呈现映射及 raw decoder 时钟后进一步升为 profile 4；`e639172` 将索引映射接入实际采样与两种顺序的访问候选，升级到 profile 5，并公开 runtime 的纯计算 isobmff 模块。下文记录的已发行/已安装 19 包事实不被改写，新发行及产品 pin 升级另行验证，见[容器索引记录](video-container-index.md)、[呈现时间记录](video-presentation-time.md)、[索引采样记录](video-indexed-source.md)和[新 SDK 合成记录](video-indexed-composition.md)。

独立消费者只复制 registry，不引用工作区路径。其 Cargo.lock 中自有包的版本/source/checksum 全部匹配 manifest；完整外部依赖图必须与源码锁图一致。产品消费仍需核对其自身更大的 Cargo 图，不能直接覆盖产品锁文件。

## 本次固定材料

| 材料 | 身份 |
| --- | --- |
| SDK 版本 | `0.1.0-video.20261007.1` |
| SDK manifest SHA-256 | `d26836cfaa20f356de1d87917cc5b3d6435fb6255bcbc7cda75e44667afe9227` |
| 本次 release worker manifest | `0dbfaf167806b329118ac681a13f3c518bcdcca2f9493834f1dbd5cffaee043a` |
| codec component record SHA-256 | `a73285ab54fc310110b4e8b53cd318ef86fca20f84d51bdd67c4d9f0bfdd54dc` |
| B-frame/VFR/AAC 源素材 SHA-256 | `ca7211a9b5e1bbdb66c7bd98e515484d7b6bc5ee8bbeeadb6cabb06d90bd73a6` |
| 输出 YUV SHA-256 | `acca4d419c9ad689cc5393099ee0e8e683429aad0f55fc45e9620510f0a676a9` |

包为 `releaseCleared: false`。上述 worker 为 macOS arm64，从同一源码提交以 `tools/media-worker/build.py --release` 和经核验的固定 FFmpeg 组件构建。7 个二进制合计 8,152,280 bytes，不包括宿主、字体、GPU 或模型，不作为应用体积结论。此前 debug worker 的检查仍有独立报告，不能混用摘要。

## 实际闭环证据与限制

`tools/video-sdk/check-runtime.py` 从独立发行包执行真实工程：导入固定视频资源，添加半透明红色图形层，对照编辑前后画面，前后 seek 并验证返回原帧，导出 128×96、25 fps、3 秒的 FFV1 审计 MP4。编码候选共 75 帧、408,469 bytes；与独立 FFmpeg 7.1.1 解码所得 1,382,400 字节 I420 逐字节一致。保留源缓存最高 313,344 bytes，这是该语料 active source cache，不是进程峰值内存。

同一消费者另解码 144,000 个 AAC 样本；成片没有音轨，不能据此宣称音画混合输出已验收。CPU 参考负责这次合成，GPU feature 实际编译但未在此消费者执行 GPU。连续源区间 `[0,3)` 来自已知语料证明，不能当成任意素材的完整 probe/edit-list 支持。

同一测试先用原 debug worker 通过，再用产品安装器安装的新 release worker 通过。后者报告为 `.codex-work/video-runtime-installed-worker-consumer/report.json`，前者为 `.codex-work/video-runtime-consumer-bfc9398/report.json`。完整消费者源码摘要和 Cargo 锁图摘要都在报告中。

工具测试 3 passed，facade all-features check、strict Clippy 和 default WASM check 通过。Musterwork 提交 `89c9c1d7e` 固定这次 SDK，安装器保留旧 authoring 档案并新增 runtime 闭包；12 项安装/版本测试、4 项真实编辑/权限集成、1 项持久重开测试和产品适配器 strict Clippy 通过。PPT 发行身份保持原值，6 个视频编辑领域 crate 相比旧 pin 源码无差异；这些是各自明确范围的证据，不代替全应用升级验收。

## 继续落地

产品现已另加受信 worker 安装和授权 NativeVideoMediaHost 探测入口，详见产品 `docs/architecture/native-video-media-runtime.md`。通用素材区间索引、图片/字体读取、标准/派生 PCM 持久化、设备播放、输出验证发布、正式 RPC/技能及 UI 连接仍未全部完成。完整产品目标继续以[实施记录](video-editing-progress.md)和[验收矩阵](../design/video-editing/acceptance.md)为准。

## 当前索引合成升级

SDK `.20261007.2` 从干净 `76a1621` 构建 20 包，已通过新版独立 registry 消费者；Musterwork `dd5e1c7b6` 安装并固定同源 profile 5 worker。精确帧/空白、统一预览导出适配器、五组真实素材及新版 pin/验证边界见[索引合成记录](video-indexed-composition.md)。上文旧版 19 包及消费证据保留为历史，不代表当前产品 pin。
