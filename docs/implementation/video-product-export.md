# 产品冻结版本导出候选

2026-10-07。产品提交 `276e2ab26904d0379604572101b9e1cb41cfb99d` 接通原 SQLite 工程、固定 SDK/worker、共享媒体资源、工程合成/混音到本地 MP4 候选。Office SDK `.20261007.5`、worker profile 5、PPT 与 v0.1.0 稳定标签未改变。

## 实际实现

Musterwork 的原 Runtime 持有用户/工作区、不可变 revision、当前 writer epoch 和全部资源的存活检查。导出固定历史版本，可以与后继编辑并行；素材隐私删除与 writer handover 仍阻止候选读取和提交。PCM 沿原内容账户增加精确历史来源授权，复用原缓存身份，不复制数据库或绕过删除。

预览中的资源绑定、视觉源与声音 bank 已提取为共同媒体适配器。导出直接调用 Office 的有限 `ExportPlan/ExportSession`、真实索引采样、CPU 合成、mixer 和编码 worker，产品没有另写时间映射、GOP 定位或 PCM/DSP 算法。

候选采用有界匿名本地文件，完成后收回唯一写句柄，检查实际长度并流式计算 SHA-256。异步调用被抛弃会取消物理工作，编码器及各源进程退出后才释放计算 permit；交互编辑仍有独立额度。候选只提供绑定原权限的流式读端口，没有公开路径或虚构文件回执。

真实输出重新打开检查 codec/尺寸/声道/采样率、精确 movie 终点与完整 CFR 呈现网格；逐页检查允许 B 帧解码顺序，再用既有 SDK 取样器抽检首、中、尾三帧和实际解码一块声音。声音格式直接复用 SDK 归一化，兼容真实 decoder 的 packed/planar 表达，不把传输 f32le 名称误当 codec 帧格式。

## 验证

- 3 项显式真实导出集成通过：硬件 H.264/AAC；开始导出后，在当前 head 删除画面/原声片段及资源，仍输出原 revision 2；取消/容量/磁盘输出额度拒绝、丢弃调用后的物理退出、成片候选产生后删除源仍拒绝读出。最终冻结版本用例另在最终代码上复跑通过。
- 2 项新增 SQLite 授权检查通过，覆盖固定版本、错误用户/digest/未来版本、writer handover 和旧内容删除；9 项 PCM 持久化回归包含其中一项新增 PCM 检查。
- 5 项原预览、1 项真实 VFR 画面和 4 项真实 AAC 混音回归通过；生产库 strict Clippy、整个 Device 装配编译通过。
- 独立 FFmpeg/ffprobe 7.1.1 完整解码 3 秒、128×96、75 帧及 48 kHz 双声道音频，逐帧呈现为 i/25；画面具有真实内容和变化，原声对照 SNR 约 41.7/42.0 dB。原工程长度为 144,000 可听采样帧，原始 AAC 解码另带尾 padding，报告区分两者，不声称无损、响度或物理音画验收。

产品入口 `apps/agent-runtime/scripts/verify-video-export.py`，设计 `docs/architecture/native-video-export.md`，本地报告 `.codex-work/video-export-report.json`。报告绑定提交、文件/日志摘要、实际输出和独立解码证据；产品原 210 个 WIP 文件、完整 diff 和展开状态保留。

## 当前边界

计算状态 Candidate 不等于产品任务 Completed。持久导出意图、原 ProductCommand/Invocation 内容预留、原子 Artifact 发布、工具结果文件卡片、播放器/下载尚待接通。匿名候选不能跨重启恢复；重启须由原 Runtime 在同一导出意图下重试或恢复已持久化的已验证输出。实际编码字节不可假定每次重算相同，输出幂等依赖原任务/尝试/回执，不能依赖重新编码得到同一 hash。

Browser 输出、完整页面、多容器/图片、保调准备、长时/4K/平台与全量功能验收仍在目标内。此增量没有完成用户全部本地验收目标，也未公开发行组件或部署产品。
