# MusterOffice 整体架构

状态：架构评审稿。产品名称、独立项目、可扩展办公定位和语言/组件策略已确认；具体组件、协议档案及实现仍待验证。

## 分层建议

```text
Agent / Automation / Application
              │
       Skill / platform Plugin
              │
       MCP / SDK / CLI adapters
              │
         Operation Service
              │
       Host execution adapter
              │
   ┌──────────┼────────────┐
Presentations Documents Spreadsheets …
   └──────────┼────────────┘
   Shared primitives and contracts
              │
   Native / WASM execution bindings
```

该图表达逻辑分层，箭头不代表核心可调用任意宿主能力。每一领域通过受限接口访问已授权资源。

## 共享与独立

| 层        | 适合共享的内容                     | 应当保持独立的内容                    |
| --------- | ---------------------------------- | ------------------------------------- |
| 协议      | 能力发现、版本、事务封装、诊断结构 | 各领域的具体操作和约束                |
| 资源      | 字体、图片、摘要、字节句柄、限额   | 领域特有资源依赖                      |
| 文字/绘图 | 字体塑形、字形、二维几何、颜色     | 演示文稿固定画布、文档分页规则        |
| 计算      | 取消点、资源预算、增量依赖基础设施 | 表格公式语义、分页、图表数据语义      |
| 格式      | ZIP/OPC/XML 等经过验证的底层工具   | PPTX、DOCX、XLSX 的各自读写与兼容规则 |
| 宿主      | 文件、网络、缓存、任务持久化、隔离 | 产品权限、账户、计费与发布策略        |

不把所有办公领域塞进演示文稿的页面/Scene 模型。电子表格需要公式依赖和重算；文档需要流式段落与分页；演示文稿采用固定页面。这些语义分别拥有自己的模型与验收。

## 按需装配

接入演示文稿的产品不应被迫下载未来的电子表格公式引擎。公共发行清单声明模块、版本和资源闭包，宿主按需要安装或预置。

模块不是任意脚本插件。首版采用可审查、版本明确的实现边界，不让输入文档动态执行代码。跨领域转换作为显式适配，报告损失与验证范围，不承诺天然无损。

## 技术路线状态

用户已确认 Rust 主体＋WASM/原生绑定＋TypeScript 薄接入，并允许经验证的 C/C++ 底层组件，见 [ADR 0003](../decisions/0003-language-and-component-strategy.md)。“纯计算”描述无宿主副作用，不代表依赖必须纯 Rust。具体图形/字体/媒体组件和跨语言边界仍待实验；改变主体方向应另记依据和决定。

文件、网络、时钟和存储由宿主注入；跨端共用布局、格式和诊断算法。CPU 标准绘制、字体选择和资源身份保持可复现。性能必须对完整流程测量。

## 标准 Agent 接入

通用调用采用 MCP，任务方法采用 Agent Skill，平台安装采用 Plugin，嵌入和实时播放采用 SDK/CLI，共用版本化 Operation Contract。标准宿主支持独立本地/自托管；产品宿主可接入已有 Runtime 和存储。内核不直接依赖这些协议和身份系统。

Musterwork 的接入形式为 Skill＋MCP 注册＋宿主/Artifact 适配＋Viewer/Player SDK；其他产品无需引入 Musterwork。大文件和逐帧数据通过宿主数据通道，模型侧只接收有界工具结果与资源引用。详见[接入总设计](agent-integration.md)及[接口规格](../design/agent-interfaces.md)。

## 首个模块

接口、排版与格式方案见[演示文稿详细设计 v0.4](../design/presentations.md)及[实施规格](../design/implementation/README.md)。一期完整演示目标已由 [ADR 0002](../decisions/0002-phase-one-complete-presentations.md)确认；现行设计已纳入时间/事件、媒体、SmartArt、公式和原生互操作，修订依据见[v0.2 评审](../reviews/2026-09-24-presentations-design-review.md)。

演示文稿领域核心拥有文档与来源绑定、编译计划、布局、媒体时间关系与播放语义；时钟、输入事件、解码设备和实际呈现由宿主适配。原生 writer 使用编译计划内的对象语义与布局，Scene 只承担绘制；实际输出回读检查后由宿主提交。具体接口及绘制后端仍待评审和验证。PPTX 要求只约束 Presentations 模块，不构成 Documents/Spreadsheets 已经支持的声明。
