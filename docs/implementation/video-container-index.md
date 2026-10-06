# ISO BMFF 原始样本索引

状态：2026-10-07 开发实现。`mo-media-isobmff` 负责受预算约束的非分片容器索引；尚未接入产品导入、受限 worker 查询或 Browser 媒体 Host。本记录不代表连续呈现区间已证明。

## 时间与数据合同

独立保存 movie、track-media 两种时基及 decode、composition 时间。每个压缩样本保留原始 DTS、PTS、解码增量、文件范围、sample-description 索引和 stss 标记。stss 不是无条件闭合 GOP 承诺。

保留 mvhd/mdhd/tkhd 声明时长、track flags、显示矩阵与 16.16 显示尺寸；样本表求和的 decode duration 单独保存。elst 的 movie 起点、时长、media time 和有符号 16.16 rate 原样保存，包括 empty、dwell 与非单位速率声明，不在此层应用或舍入。sample description 的原始初始化字节及依赖/roll/recovery 相关辅助表交给后续 decoder admission，尚不解释这些辅助表。

设计参考 [Apple sample atoms](https://developer.apple.com/documentation/quicktime-file-format/sample_atoms) 和 [W3C ISO BMFF byte stream format](https://www.w3.org/TR/mse-byte-stream-format-isobmff/) 对样本时间及编辑映射的区分。W3C 文档针对 MSE，不将其局部条件冒充通用文件格式的完整要求。实现为原创 Rust 有界解析器，未复制 FFmpeg 或 mp4parse 实现。

## 边界与预算

输入为宿主提供的 `ReadAt`、已核实长度及取消回调，无路径、网络、权限或持久化。顶层只读 box header，按核实长度跳过 mdat；只加载受限 moov。单次回调最多 64 KiB，支持短读取，对提前 EOF、越界返回及回调 panic 明确失败。

默认 metadata 32 MiB、全文件最多 2,000,000 样本、64 轨、8192 boxes；样本、编辑项和 descriptions 采用跨轨共享预算。硬上限、算术溢出检查及 fallible allocation 独立生效。这些是逻辑预算，不是物理 RSS 实测结果。所有 chunk/sample 范围必须属于实际 mdat；stsc、stsz/stz2、stts、ctts、stss 的数量与引用保持一致，重复单例拒绝。

支持 stco/co64、普通/64 位 box header、stz2 4/8/16 bit、ctts v0 无符号及 v1 有符号偏移、v0/v1 时钟和编辑列表。明确拒绝 fragmented/compressed movie、受保护样本、外部数据引用、cslg、shadow samples 等未实现语义。未知辅助信息不构成可解码证明。没有把 unsupported 文件转为猜测时间轴。

## 实际验证

7 项 Rust 检查通过，覆盖：精确时钟与编辑项、紧凑/固定大小表、畸形引用、预算和逐点取消、回调失败、逐字节截断/结构位翻转，以及 256 组确定性生成的 chunk 布局与独立展开结果比较。超过 8 GiB 的稀疏虚拟来源读取不足 1 KiB；此项证明跳过媒体区的算法，不是产品大素材存储验收。

`video-container-check.py` 显式运行真实文件测试，与独立 FFmpeg 7.1.1 的 raw demux 结果比较 DTS、PTS、位置、大小、同步标记及每个压缩样本 SHA-256：

| 实际输入 | 轨/样本数 | 索引读取 / 文件字节 | 结果 |
|---|---:|---:|---|
| B 帧/VFR MP4 | 2 / 219 | 4,403 / 75,885 | 一致 |
| 既有 SDK 消费者导出的 MP4 | 1 / 75 | 1,157 / 408,469 | 一致 |
| 5.1 声道 AAC M4A | 1 / 96 | 1,170 / 33,379 | 一致 |

参考程序存在 12 个 packet duration 与原始 stts 不同的情况。检查使用相邻实际 DTS 差和独立 demux 总解码区间核对 stts，同时记录差异；没有把 codec 推算的 packet duration 写回原始索引。该 VFR 视频的 mdhd 声明为 90,090 media ticks，stts 总和为 89,089，elst 声明为 3,003 / 1,000 movie seconds，三者不能互换为连续画面终点。

本地证据目录 `.codex-work/video-container-real-verified/` 保存三份实际索引、独立输出、测试日志和输入摘要；三次真实文件测试均实际执行，0 ignored。常规 cargo test 中该外部输入测试显式 ignored，不能替代脚本证据。strict Clippy、WASM 编译及 6 项验证驱动测试通过；日志为 `.codex-work/video-container-{tests,clippy,wasm}-verified.log` 和 `video-container-driver-tests.log`。只验证了 WASM 编译，未声称真实浏览器读取或 worker 隔离。

新索引检查已接到 verification media group，依赖其生成的真实媒体。本轮没有重跑整个 media group，也未取得远程 CI 证据。

```sh
cargo test --offline --locked -p mo-media-isobmff
cargo clippy --offline --locked -p mo-media-isobmff --all-targets -- -D warnings
python3 tools/verification/video-container-check.py \
  --input /absolute/path/to/nonfragmented.mp4 \
  --ffprobe /absolute/path/to/independent/ffprobe \
  --output .codex-work/new-container-evidence
```

## 后续集成

将解析放入现有受限 worker，以有界分页返回原始索引；实现精确 edit mapping、明确的 decoder 时钟和有证据的呈现区间/gap，再接 Native/Browser 的真实素材采样。分片文件、其他容器、更多 timing 特征仍需独立实现和媒体语料验证。当前 runtime SDK 的 19 包发行、产品固定 manifest、worker 和 Browser SDK 没有被此新 crate 隐式替换。
