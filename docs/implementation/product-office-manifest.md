# Musterwork 原生产物读取合同

2026-09-26：本阶段开始实际修改 Musterwork 产品源码，读取合同为 `presentation-artifact/4`。产品基线为 `836a5cf3eb68de78aebad887cc6002b08a892125`；这是本次观察的提交，不是永久依赖。以下 `MW:` 路径位于产品仓库，本仓库不导入其私有源码。新增读取器尚未接通创作提交和用户界面，不能视为完整接入或替换验收。

## 读取边界

Rust 的 `MW:apps/agent-runtime/crates/runtime/artifact/src/presentation_office_manifest.rs` 及同名目录负责严格反序列化、语义验证、依赖闭包和产物选择；现有 `presentation_file_content` 对新版本分发到该读取器，旧 HTML/SlideSpec 仍走原解释。下载选择显式 `pptx_asset_id`，不会因为模板来源也有 PPTX MIME 而下载错文件。

TS 的 `MW:packages/agent-runtime-product-client/src/office-presentation-artifact.ts` 通过独立 package export 提供 discriminator、metadata parser 和绑定实际字节的异步 decoder。旧 `parseRuntimeEditablePresentationManifest` 继续拒绝新版本，以免下游把原生模型当 HTML/SlideSpec 或把五类声明压成一个 QA pass。新产品消费者应使用原生类型，后续再连接 Viewer/Player。

两个绑定读取器都核对 Artifact 身份/版本、清单实际长度与 SHA-256、根 MIME 和精确依赖集合。依赖按 canonical ContentId 排序，根内容不进入自己的依赖闭包；同一 ContentId 的重复资产映射仅在全部元数据相同时共享。缺失、额外、重排、冲突或重复的 Runtime dependency refs 被拒绝。原生依赖最多 1024 资产加 public bundle 和可选来源 PPTX，产品领域容器上限相应为 1026；各历史格式自己的较小限制保留。

TS 在等待 Web Crypto 前复制清单 bytes 和版本引用，避免异步期间输入被改写。实际字节入口先执行 2 MiB 限额、严格 UTF-8 解码和受限 JSON token 扫描，拒绝重复键（包含不同转义但相同名字）、孤立 surrogate、非有限数和错误整数词法。JSON.parse 后的纯 metadata parser 无法恢复已丢失的重复键和数字词法，因此不能代替实际字节入口。

## Wire 字段与含义

| 字段 | 合同 |
| --- | --- |
| `version` | 固定 `presentation-artifact/4`；产品清单 MIME 仍为 `application/vnd.musterwork.presentation-manifest+json` |
| `artifact_id`、`artifact_version`、`title` | canonical 非 nil UUID、正 u64 十进制字符串、最多 512 Unicode scalar 的非空白标题；不 trim 或强制转换身份 |
| `kernel` | `document_id`、`revision`、`semantic_digest`、`request_sha256`、`settings_sha256`、`renderer_profile`、`renderer_sha256`；文稿和 renderer profile 是有界 ID，五个 digest 是 lowercase SHA-256 |
| `bundle` | public bundle 的不可变 ContentRef，MIME 为 `application/vnd.musteroffice.bundle+json` |
| `assets` | 有界 `{id, role, content}`，content 为产品 `ContentRef`，保留公共资产 ID 到产品内容的映射 |
| `model_asset_id`、`pptx_asset_id`、`quality_asset_id` | 显式选择模型、交付 PPTX、总体质量报告；必须匹配相应角色与 MIME |
| `pages` | `{id, image_asset_id, width, height, sample}`；页和 preview asset ID 分别唯一，当前实际导出 profile 只有 `sample.mode=editor` |
| `claims` | 五类声明完整覆盖，保留公共字段 `kind/status/subjectSha256/basis/profileId/evidenceAssetIds/reason`，没有整体 `native_editable` 或 QA pass |
| `source_pptx` | 可省略的独立来源引用；显式 null 非法，不能替代输出选择 |

ContentRef 保留 `content_id/byte_length/media_type/sha256` 四个字段，`byte_length` 必须是 canonical u64 字符串。一般资源可为零字节；bundle、模型、输出、质量报告、来源 PPTX 和预览必须非空。单资产至多 128 MiB，模型与总体质量报告各 32 MiB，bundle 2 MiB，资产合计 512 MiB；读取容器至多 256 页，单边至多 8192。256 是容器容量，不提升 40 页创作或 80 页来源预览的产品操作准入，也不表示 256 页性能已通过。

资产角色保留 editable-document、pptx、preview、quality-report、image、font、audio、video、model3d、embedded、source、other。角色能被读取不等于媒体或高级对象已实现。未来增加实际 preview sample、播放包或字段时必须同步演进公共合同、产品 owner 和消费者，不在当前版本内猜测未知结构。

声明分别为 structure、layout、native-editability、playback、target-application；状态分别为 passed、failed、not_proven、not_applicable。所有 subject 指向实际选择的输出摘要。passed/failed 必须有 quality-report 角色的证据引用及非 none basis，目标应用 passed 还要求 application-test；未证明和不适用必须有解释。这些只是引用和声明的合法性检查，不验证证据内容是否足以证明结论。

## 真实验证与限制

[自有夹具生成器](../../tools/verification/product-office-fixture.py)从[已封存原生导出](embedded-export.md)读取并核对 SHA，生成稳定 UUID 的产品映射。12 个实际交付文件和 public bundle 共 13 个引用逐字节一致，未复制第三方文稿或私有产品实现。产品夹具为其 `tests/fixtures/office-native`，包含针对默认忽略 PPTX 的局部例外，确保真实文件会随测试源码保留。

Rust 与 TS 共用 62 组元数据案例和 14 组原始 JSON 案例，另分别验证 1024/1025 资产和 256/257 页边界，包含可选来源后的完整 1026 引用。目标应用 application-test 的正例只证明声明语法可接受，绝不计作实际 Office/WPS 验收。真实导出夹具仍只有 structure passed，其余四项 not_proven。

产品两个 Rust 模块最终 71 项测试通过；产品客户端完整回归 624 项通过、76 项按原配置跳过，随后新增容量用例后的原生专项 96 项通过。新增 TS 源码严格类型检查通过。Clippy 的完整 all-targets 检查未通过：首先在既有 capability 代码发现 collapsible_if，限定所选包后又在既有 domain 测试发现 unwrap_used。随后仅检查所选生产库，发现并修正本次新增 validator 的 collapsible_if，最终所选生产库严格 Clippy 与格式检查通过；各次日志均保留，不能将局部检查替代产品全仓质量门禁。

本阶段没有新增外部依赖，也没有改变内核、WASM 或渲染执行代码，因此没有重新构建或宣称新的体积/性能结果。验证来源和原始记录由[阶段证据](../reviews/evidence/2026-09-26-product-office-manifest-verification.json)绑定；其中产品部分仅保留相对路径和摘要，私有源码留在产品仓库。

## 接下来接通生产者与消费者

产品 Runtime 仍需在既有 Invocation/fence/generation 所有权下运行 NativeExporter，把私有结果写入 Content Store，并对最终存储字节重新执行共享交付验证。只有这些验证成功且取消/CAS 条件仍成立，才能在现有事务提交 Artifact、资源、Ledger 和成功回执。读取器不能自行授权或发布资源，也不能用这份 metadata 代替实际 bundle/model/PPTX/preview 的检查。

随后连接原生草稿与候选、Web/Desktop/Managed 路由、Viewer/Player、历史转换和外部编辑。当前完成的是新读取合同及 Rust 文件解析入口；实际默认工具、产品展示和创作提交尚未切换。[MW01–MW16](../design/implementation/musterwork-adapter-spec.md)、完整高级内容、Office/WPS 往返、跨平台/安装包及 E0–E3 门禁继续开放。
