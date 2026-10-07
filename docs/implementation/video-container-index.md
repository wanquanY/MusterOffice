# ISO BMFF 原始样本索引

状态：2026-10-07 开发实现。`mo-media-isobmff` 负责受预算约束的非分片容器索引，已有受限 worker 的有界分页查询；尚未接入产品导入或 Browser 媒体 Host。本记录不代表连续呈现区间已证明。

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

## 受限 worker 分页查询

源码提交 `3c95c603ea3d80754d0a514afe57669eb7433059` 新增 `NativeIndex`。计算发生在核实 bundle 的独立子进程内；应用资源上限后才读取和解析容器。进程保留一份不可变索引，使用同一 `ReadAt` capability 往返取原文件范围，不获得输入路径。索引会话不允许解码帧、执行编码或切换为其他状态。

读取端口和 decoder 共用 `Owner`，统一调用序号、递增读取序号、全会话累计读取预算、整次调用 deadline、取消边界、回调 panic 处理、停止及进程回收。没有复制第二套进程生命周期。调用方 callback 仍须及时返回：父进程无法抢占它自己的阻塞 callback；返回后会检查 deadline/cancel 并停止子进程，不能称为硬实时中断。

公开查询值包含：movie 概要、单轨概要、样本页、edit 页、mdat 范围页，以及 sample-description/辅助表的分段原始字节。每页最多 128 条记录，初始化/辅助字节每段最多 4096 bytes；无需提升既有 64 KiB 控制 header 上限。双方验证查询范围及页面数量，宿主复核字节范围和各类预算。静态非法查询在发送前拒绝；worker/传输/响应校验失败会永久停止 owner，不能继续返回旧索引。查询本身不重新解析或重复读取来源。

私有 worker 协议升级为 `musteroffice.native-media-worker/3`。新宿主明确拒绝旧协议 bundle，不自动兼容或降级。runtime 源码打包闭包相应从 19 增至 20 个自有 crate，新增 `mo-media-isobmff`；6 包 authoring 闭包不变。产品当前固定的 19 包 SDK、profile 2 worker 继续作为匹配的一组使用，本次没有替换产品 pin，也未公开发行新的 SDK。

从上述干净提交构建的 macOS arm64 release bundle 为 `.codex-work/video-index-worker-final/`，manifest SHA-256 `e4ca639950ec5bb51c1cbc04e5ba55ab092030efa873e5185a6b7d34c22376dd`。7 个二进制总计 8,270,552 bytes，包含 worker、窄 FFI 和 5 个固定 FFmpeg 库，不包含应用、GPU、字体、模型或安装包；此数值不是全产品体积验收。

实际三份素材通过该 bundle 的短读取和逐页查询，所有概要、样本、edit、初始化数据、辅助表和 mdat 范围与原始解析一致，查询期间来源读取计数保持不变。390 个压缩样本仍与独立 demux 对照一致。实际故障测试验证样本/读取预算、越界回调、panic、取消、超时、非法范围及 drop；正常关闭、取消后调用和 drop 均检查物理子进程已回收。证据为 `.codex-work/video-index-worker-verified/`，其中所有实际输入用例显式执行，0 ignored。

原有 bundle 校验、decoder 与 encoder 的 10 项实际集成检查通过；新 bundle 的 3 种工程导出通过独立 FFmpeg 检查，报告在 `.codex-work/video-index-worker-encode-verified/`。普通 Rust 检查共 22 项通过；包含新页面边界/最大数字序列化和原传输故障测试。外部输入测试在普通命令中仍显式 ignored，实际证据由上面的专门脚本取得。相关库和 worker strict Clippy、runtime `native,gpu` 编译、parser WASM 编译、2 项打包闭包与 6 项验证驱动测试通过。没有扩大为全媒体 group、远程 CI、其他操作系统或浏览器验收。

```sh
python3 tools/verification/video-container-check.py \
  --input /absolute/path/to/nonfragmented.mp4 \
  --worker-bundle /absolute/path/to/verified-current-profile-bundle \
  --ffprobe /absolute/path/to/independent/ffprobe \
  --output .codex-work/new-worker-index-evidence
```

## 后续集成

当前 `e639172` 已进一步接入真实索引采样和访问候选，worker 协议为 profile 5，见[索引采样记录](video-indexed-source.md)。下面 profile 4 是前一阶段记录，下一次产品升级须使用审核配对的当前 SDK/worker。

后续提交 `3c0cf1a` 已实现精确 edit mapping、明确的 raw decoder 模式及容器呈现范围/gap，并将私有 worker 协议升级为 profile 4，见[呈现时间记录](video-presentation-time.md)。上述 profile 3 数值是历史核验事实；当前检查脚本要求当前 profile。通用 codec 样本/画面准入与 Native/Browser 素材采样仍须接入。分片文件、其他容器、更多 timing 特征仍需独立实现和媒体语料验证。下一次产品升级须同时审查新的 20 包 SDK 和 profile 4 worker，经过独立发行消费及原权限/持久化集成验证，不能直接链接相邻源码。现有 Browser SDK 不包含该索引 Host。
