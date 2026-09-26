# MCP 版本化缓存元数据

本阶段完善[原生 MCP stdio](mcp-stdio.md)的协议响应，不改变文档、任务或资源业务合同。[阶段证据](../reviews/evidence/2026-09-26-mcp-response-metadata-verification.json)保存实际协议响应、最终二进制和与上阶段的差异；原 stdio 记录及失败输入不覆盖。

首次接入的实际消息表明：现代 `tools/list`、`resources/list`、`resources/templates/list` 没有 `ttlMs/cacheScope`，旧版 `resources/read` 则带有现代缓存字段。SDK 会移除旧版不支持的 `resultType`，但不会替应用补齐或删除缓存字段。这是适配器的职责，不能把 SDK 编译或一次调用成功当成完整协议校验。[2026-07-28 缓存规范](https://modelcontextprotocol.io/specification/2026-07-28/server/utilities/caching)要求指定的完整响应包含缓存提示，按授权过滤的列表适合使用私有缓存范围。

现在通过同一请求上下文的已验证协议版本选择响应字段。现代三个列表及资源读取都设置 `ttlMs=0`、`cacheScope=private`；旧档案省略这两个字段。缓存声明不承担权限校验，原有宿主授权仍执行。`server/discover` 的 SDK 元数据也纳入实际检查。未声明支持的 prompts 接口明确返回 method-not-found，避免继承默认成功列表而形成未经定义的额外服务。

更新后的独立检查完成 173 次请求，对两代各 32 个成功可缓存响应核对版本、TTL 和范围，八次未声明的 prompts 请求均被拒绝；同时重跑两代完整开发链路、拒绝权限/跨作用域、重连和真实文件导出。十项适配器测试、严格 Clippy 和格式检查覆盖最终源码。两次下载 PPTX 的原生对象、文字、20 个相关 XSD 部件及四页 PNG 探针再次独立检查，与上一阶段的实际 PPTX 字节/像素保持一致；产物身份见阶段记录。

没有将此修正视为完整 MCP 接入通过。标准 parse/invalid-request 错误恢复、SDK 取消/断开/输出失败组合、HTTP/任务扩展、SDK/Skill/Plugin、完整高级内容和 Musterwork 接入继续实施。本阶段没有新做全仓、Native/WASM 或产品性能验收。
