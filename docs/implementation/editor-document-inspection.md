# 编辑文稿的页面发现

状态：计算接口实现，产品编辑与放映验收尚未完成。

## 输入和责任

`EditorPageRequest.inspect` 返回 `EditorDocumentInfo`，不创建、替换或清理当前渲染页。
它与 `prepare` 使用同一个 AuthorPlan/SourcePlan/source reader，因此产品不计算私有计划摘要，
不根据页序拼接原生路径，也不通过临时导出和再次导入获取编辑后的页面。

| 输入 | 必需材料 | 校验与含义 |
|---|---|---|
| `author` | Document、显式 ExportDefaults；材料通道必须为空 | 建立语义计划；资源声明纳入计划摘要，实际图片字节在 prepare 时校验 |
| `retained` | Document、原始 OPC 字节 | 校验来源摘要、原始绑定及 profile 允许的字段覆盖；不自动迁移 profile |
| `pptx` | 原始 OPC 字节 | 从原生 presentation 页序和页面声明发现页面；没有模型 ID |

文稿查询不接受字体字节，不调用图片、字体或光栅组件。它不是素材可用性、全部页面可渲染、
保留内容可编辑或正式交付成功的证明。沿用 32 MiB JSON、128 MiB 材料及模型/包解析预算；
Rust 计算接受取消检查，失败不返回部分目录。宿主持有权限、资源加载、Worker 调度和迟到结果淘汰。

## 身份和页序

- `sourceSha256` 直接填入页面准备的 `expectedSourceSha256`。原始 PPTX 使用包摘要；Author/Retained 使用领域分隔的原生计划摘要。
- `model.id` 与 `model.semanticDigest` 标明对应 Document；后者等于 SnapshotRecord.semanticDigest，不能代替 CAS revision。原始 PPTX 的 model 为 null。
- `pageSize` 来自已校验声明。原生文件没有页面尺寸时返回 null，不猜测默认尺寸。
- `slides` 按实际演示顺序包含隐藏页，提供 opaque `slide` 地址、原生 nativeId、稳定 slideId、原生名称和 hidden。
- Author/Retained 的每项必须准确对应模型 slideOrder；原始 PPTX 的 slideId 为 null，不生成没有模型归属的虚假 ID。

自建文稿的 NativeBindings 统一持有页序、页面路径和原生 ID，直接渲染、文稿查询与 PPTX 写出共用。
移动页面后稳定 SlideId 保持不变，原生路径可以改变；宿主必须用新的查询结果按 SlideId 重新选择页面。
Retained 使用原始包经验证的 bindings，不根据自建文稿命名规则推算。

默认字体、颜色等 ExportDefaults 变化会改变 Author 计划摘要，但不改变 Document 的 semanticDigest。
Viewport、字体材料和采样属于页面 view 身份，不属于本接口的文稿目录。页面准备重新校验计划身份；
旧计划摘要配合已编辑 Document 会拒绝，原有页面仍可查询。成功准备新页面后旧 view 失效。

## 公开调用

Native `--editor-page-session` 使用既有 JSON/材料/字体帧通道，`inspect` 回复没有像素。
WASM `EditorPageSession.inspect(requestJson, material)` 不需要 component 参数。
TypeScript `PresentationEditorPage.inspect(input, material?)` 返回生成合同的 EditorDocumentInfo，
保持与 prepare/query/pick 相同的 owner 关闭、重入和错误规则。

```ts
const info = page.inspect({kind: 'author', document: snapshot.document, defaults});
const selected = info.slides.find(slide => slide.slideId === selectedSlideId);
if (!selected) throw new Error('Selected slide no longer exists');
const rendered = page.prepare({
  input: {kind: 'author', document: snapshot.document, defaults, resources},
  page: {
    page: {...pageOptions, expectedSourceSha256: info.sourceSha256, slide: selected.slide},
    imageSource: 'embeddedSnapshot', sampling: 'nearest',
  },
  fonts: fontManifest,
}, verifiedInputs);
```

示例中的 pageOptions、材料范围、字体清单与实际字节由已授权宿主提供。宿主将查询/渲染结果绑定到
发起计算时的 Snapshot 与 generation；内核摘要不授予发布权限，也不自动持久化候选编辑。

## 验证入口

Kernel 单元测试：`editor_page/inspect/tests.rs`；真实跨端及编辑往返：
`tools/verification/editor-document-parity.mjs`；TS 所有权测试：`editor-page-client-tests.mjs`。
详细执行证据见 [文稿查询验证](../reviews/evidence/2026-10-03-editor-document-inspection.json)。
能力矩阵、高层表格/保留文字操作、IME、固定 SDK 集成与双端完整产品验收继续推进。
本接口重用有界计划计算，尚无跨查询计划缓存或延迟目标资格结论。
