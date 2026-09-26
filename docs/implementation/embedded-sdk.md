# Rust 嵌入 SDK 与产品开发材料

2026-09-26：在[无状态原生导出](embedded-export.md)与[产品原生清单读取](product-office-manifest.md)之后，提供可独立编译的本地 Rust SDK，并在 Musterwork 中加入固定摘要的材料准备入口。这是开发接入，不是默认 PPT 引擎切换，也不是公开发行。

## 同源库与宿主边界

[`mo-embedded-sdk`](../../crates/mo-embedded-sdk/src/lib.rs)只重导出计算、操作、模型、编辑、交付验证、OPC 与原生导出接口；没有第二套文档语义或产品任务 owner。父进程依赖闭包不包含标准 SQLite 宿主、Tokio、Skia/HarfBuzz FFI 或 WASM。图形与字体计算仍由单独固定摘要的 worker 完成。SDK 没有产品身份、授权和成功提交权。

`NativeExportCandidate::expectation()`保留调用者请求和实际 renderer 所确定的预期，供宿主写入最终存储之后重新执行共享交付检查。它不能代替权限、取消、Invocation/fence 或事务检查。示例将真实输出分块写入自己的新目录，再以同一预期读取最终文件验证；结果明确为 `committed=false`。

[`tools/sdk/build.py`](../../tools/sdk/build.py)从内部依赖闭包构造 22 个生产库、48 个 registry 包的本地 SDK。生产 Rust/data 字节保持不变；生成 Cargo 清单移除开发依赖，禁用自动测试/示例目标，测试仍在源码仓库运行。离线解析以根锁文件为种子，并核对所有外部版本及 checksum 均来自原锁文件。组件数据与已有 notice 保留；完整二进制发行许可材料仍未完成。

清单记录来源与生成文件的相对路径、长度和 SHA-256，不包含机器绝对路径。可信调用方固定清单摘要后，使用[验证器](../../tools/sdk/verify.py)检查有界文件清单、路径、精确库存与文件字节。清单不自行提供真实性；验证器应来自可信源码。归档固定文件顺序、权限、时间和 gzip header，两个最终构建逐字节相同。

## Musterwork 材料准备

以下 `MW:` 路径位于产品仓库，私有产品源码没有迁入本仓库：

- `MW:components/musteroffice/lock.json`分别固定 SDK 清单和实际已构建的平台 worker；当前只有 macOS ARM64。
- `MW:scripts/prepare-musteroffice-development.mjs`验证、分块复制到自己的暂存目录，再完整校验并重命名；不下载或执行外部程序。
- SDK 位于 `components/musteroffice/generated/sdk/<manifest-sha256>`；worker 位于独立的 `generated/workers/<target>/<worker-sha256>`。worker 更新保留 SDK 路径与旧版本。
- 已存在的缓存必须重新通过摘要和库存检查；损坏时失败，不覆盖或自动修复。并发准备核对已完成的胜出目录。生成目录由产品 `.gitignore` 忽略。

真实 SDK/worker 已在产品目录完成准备和无源路径的再次复用。产品主 Cargo workspace 尚未加入 SDK path 依赖，Runtime 也尚未调用原生生产者；当前材料准备不会改变旧产品路由。后续要将准备步骤加入正式开发/构建流程，再接通受现有 owner 管理的原生生成路径。

## 验证与实际范围

根仓库的 facade 检查、原生导出 8 项真实进程测试和两个受影响生产包的严格 all-targets Clippy 通过。Python 验证器 6 项测试、产品准备脚本 13 项测试通过，覆盖固定摘要、修改/缺失/额外文件、路径别名、symlink、损坏缓存、worker 升级和并发准备。

独立消费项目从 SDK 编译并调用真实 release worker，两页文稿的 12 份交付文件及最终文件检查与此前封存基准一致。第一次示例编译使用了不存在的设置摘要方法而失败；修正为读取候选保留的预期后通过，失败日志保留。最终 SDK 与该成功版本只改变本地验证脚本；生产 Rust、数据、Cargo 清单和锁完全相同。随后另从产品实际准备的最终 SDK 编译独立消费项目，执行产品缓存 worker 并核对交付结果，见[阶段证据](../reviews/evidence/2026-09-26-embedded-sdk-verification.json)。这仍不是 Runtime 的 Artifact 提交或产品 UI 验收。

最终 SDK 源码 gzip 归档为 707,542 字节；它不是链接后的运行体积。已固定的 macOS ARM64 worker 为 10,027,936 字节，沿用此前构建。没有新外部组件版本，也没有新的 WASM 构建或安装包/整体性能结论。上一阶段全仓 873 项 Rust 与产品 Rust/TS 回归属于各自封存版本，本阶段不重复累计为新通过数。

## 下一步与未关闭门槛

产品现有 Content Store 接口仍以整块 Vec 为主，需要从实际存储层补齐流式写入、不可变范围读取与配额/清理所有权，不能只在表层包装流式 API。随后把原生草稿和候选接入现有 Runtime 的 Invocation、取消、fence 与原子 Artifact/Ledger/回执提交，对最终存储字节重新执行共享验证。

Viewer/Player、历史迁移、完整动画/转场/媒体/SmartArt/公式能力、Office/WPS 编辑往返、跨平台发行及 E0–E3 验收仍开放。当前 SDK、MCP 和文件验证的通过都不能代替这些独立门槛。
