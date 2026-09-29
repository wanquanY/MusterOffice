# 现有模板来源兼容与本地原生启用

2026-09-29。状态：局部内核修复和产品适配入口已实现，完整模板迁移尚未完成。
用户要求在当前 Musterwork 开发环境停用旧 PPT，并开始适配原有模板。
内核继续只承担文稿计算；产品拥有目录、版本、任务、字体选择和本地启用设置。

## 内核修复

真实模板揭示了两处来源读取与页面计算缺口：

- 母版／版式的 `p:hf` 进入显式来源结构，保留日期、页脚、页眉和页码四项可选声明。
  页面选择使用最近的声明，声明内遗漏属性按启用处理；明确禁用的继承占位符不绘制，
  来源对象仍保留。独立写在幻灯片上的形状不受这项继承筛选影响。
  默认值和适用父元素依据 [Open XML HeaderFooter 定义](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.headerfooter?view=openxml-3.0.1)。
  启用的特殊占位符仍返回尚未实现的诊断，不能据此声称完整页脚功能已完成。
- `a:t` 上正确 XML 命名空间中的 `xml:space="preserve"` 复用原文字读取器，
  保留解码后的全部空格及文字，并保留可编辑来源绑定。`default`、未知值、
  其他命名空间与其他未支持属性仍保留诊断，不作宽泛放行。

解析器拒绝非法布尔值和重复声明，页面预检拒绝未知属性／子内容。
公共来源 Schema 与 TS 类型来自同一结构生成；原生导出和 WASM 共用计算实现。
测试素材为自有 XML 构造，未把真实用户模板迁入源码或固定测试语料。

## 产品启用与模板边界

Musterwork 当前开发工作区已配置 `native-office-preview`，对应新的桌面会话。
启动器将同一选择传递给 Rust 构建、Electron 和播放器，原生模式不再准备旧转换器。
后端拒绝旧渲染调用，并将活动模板计算限定为原生策略；冻结的旧任务和版本保留历史身份。
已运行的 Agent Runtime 仍需重启以加载新环境；本次没有代替用户中断其正在运行的进程。

产品目录根据当前作者格式返回 `requires_conversion`。旧 HTML 模板在完成转换前
不能选入原生会话；个人转换入口与平台维护命令重新读取精确原始 PPTX，
按同一模板 ID 创建后续原生版本，继续经过 Define、Stress、独立 Verify 及原发布校验。
导入／渲染成功不等于模板已发布，也不替代真实效果验收。

## 实际验证及剩余工作

一个现有单页中文模板使用显式 Liberation／Noto Sans SC 字体包，在候选 Worker
完成原始来源渲染及未修改 PPTX 导出。1280×720 原生预览与公共 WASM 播放器初始帧
的全部 RGBA 字节一致。字体替换清单由宿主显式提供，不宣称与原商业字体的字宽一致。
用户原文件和诊断材料仅留在忽略的本地目录，未进入本仓库。

另外两个 14 页平台模板均有 13 个指向不存在母版的 content-type Override，
被现有 OPC 结构校验拒绝。原文件保留，需要可追溯的来源修复；没有关闭校验、
暗中删除部件或将失败标为通过。三个模板都尚未作为原生 v3 目录版本发布。

修复后的 SDK、原生 Worker 和播放包目前仅为本地候选构建，尚未升级 Musterwork
固定交付物。单页像素对照不证明编辑后往返、动画、Office/WPS 一致性、
完整模型维护链路、大文稿性能或最终安装包体积。

验证命令和明确范围：

```sh
cargo test -p mo-presentation-compile --test source_header_footer \
  -p mo-pptx --test source --test text_cascade
cargo test -p mo-presentation-compile --test source_visibility \
  --test source_resource_page --test source_playback
cargo clippy -p mo-presentation-source -p mo-presentation-compile --lib -- -D warnings
npm run check:types
```

第一组 39 项测试覆盖来源读取、特殊占位符声明和文字继承；原有层级／资源／播放
另有 14 项回归通过，共 53 项。原生和 WASM 均重新构建，并通过真实单页像素对照。
产品侧另有 88 项后端模板测试、39 项模板 UI 测试、20 项启动／组件准备测试，
以及桌面类型和共享前端边界检查通过；这些结果不扩大上述验收范围。

[本阶段证据](../reviews/evidence/2026-09-29-existing-template-compatibility.json)固定源码和三个候选构建的摘要。
本机旧 Corepack 的签名密钥不能识别项目要求的 pnpm 下载签名；类型验证改由 npm
执行同一已安装的本地脚本，没有关闭签名校验或改变依赖版本。

## 2026-09-30 主工作区激活检查点

后续已在用户授权下将上述修复及[显式包类型声明修复](opc-content-type-repair.md)同步到
Musterwork 主工作区提交 `5004a9b2a`。固定 SDK 摘要以 `92160037` 开头，Worker 为
`56c03b70`，播放包为 `7b6df404`；完整摘要在产品组件 lock 与验收文档中。
开发桌面完成重新构建并加载该交付物，认证 readiness 为 2，持久化、端点与产品上下文均就绪；
后端正常启动全部开发服务且 `/health/live` 返回 200。开发旧引擎继续禁用。

18 项组件准备测试、一次实际工具循环的两个版本导出和重新导入、同一单页真实模板的
原生／WASM 全像素比较通过。原生 Worker 是完整 release 构建，12,372,160 字节；
这个单文件数值不代表安装包体积、内存或渲染性能，也不代表实际模型对话验收已通过。
两份平台原件的修复产物完整保留 79 个实际部件后仍暴露排版缺口；
后续[悬挂标点实现](hanging-punctuation.md)另行构建验证，不能把本检查点视为已加载后续所有能力。
三个历史模板的原生 v3 目录发布和完整效果验收仍待完成。
