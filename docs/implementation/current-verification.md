# 当前代码门禁

统一入口为 [`tools/verification/current.py`](../../tools/verification/current.py)。它运行当前可维护测试，不通过替换旧脚本源码生成下一阶段脚本。历史驱动、摘要与证据保持原样；本入口也不为旧证据重新背书。

## 执行

从仓库根目录执行；输出目录必须尚不存在。先按 [Skia](skia-component.md)、[HarfBuzz](harfbuzz-component.md) 文档准备固定版本组件，并设置对应目录。需要 Rust 工具链及 WASM target、离线 Cargo 依赖、pnpm/Node 依赖、兼容 wasm-bindgen；MCP 协议检查使用当前 Python 环境的 `jsonschema`。

```sh
python3 tools/verification/current.py \
  --output .codex-work/current-verification-01 \
  --skia-dir "$MO_SKIA_LIB_DIR" \
  --harfbuzz-dir "$MO_HARFBUZZ_LIB_DIR" \
  --bindgen "$MO_WASM_BINDGEN_BIN"
```

`--list` 只打印执行清单；`--groups lint,rust` 可选择局部门禁。默认运行全部组，`mcp-protocol` 会自动加入构建它依赖的 native／MCP 组。局部运行的报告会列出未选择组，不能视为全量通过。

| 组 | 验证内容 |
| --- | --- |
| lint | 当前门禁驱动自身测试；根 workspace fmt、所有 target 的 clippy，warnings 视为失败 |
| rust | 根 workspace 测试 |
| contracts | 当前 Rust 生成 Schema 与仓库产物一致 |
| typescript | 全部 TS 类型检查、operation / playback client 测试 |
| native | 构建 CLI／宿主／渲染与导出 worker；绑定实际可执行文件摘要；显式运行 ignored 渲染／宿主导出测试及导出 worker 测试 |
| mcp | 独立 MCP workspace 的 fmt、clippy、测试和构建 |
| wasm | 纯 operation service WASM check、WASM build/bindgen、editor client 联测、文档 Native/WASM parity（含真实 PPTX 导入、来源身份及统一事务） |
| mcp-protocol | 独立进程协议、transport、lifecycle、cancellation 检查 |

editor client 联测由 wasm 组在 Native/WASM 构建完成后执行，并显式绑定本轮 CLI 与 WASM 路径；不读取本机历史产物。该组也需要 pnpm。

Linux 原生构建通过 `.cargo/config.toml` 将完整调试信息放到 sidecar 文件，避免嵌入 DWARF 的 debug worker 超过既有 128 MiB 可执行文件准入上限；不放宽该上限或降低调试信息级别。WASM 保持其支持的默认配置。

## 结果解释

每步保存命令、日志、退出码、耗时和执行状态；真实 worker 测试记录绑定的可执行文件 SHA-256。`report.json` 通过原子替换更新。先决条件缺失报告 `blocked`，命令失败报告 `failed`，后续步骤为 `not-run`；任一情况均返回非零。已有输出目录拒绝覆盖。

报告记录 HEAD 与实际代码文件集合摘要，包含尚未提交的源码；前后源码摘要变化时不能报告通过。该摘要不包含文档和本地构建产物。执行期间不要修改代码。每步有 30 分钟命令超时，不把超时视为成功跳过。

隔离验证某个提交时，使用 `git archive` 的原样源码，并记录明确的 commit/tree；将 Git 工作目录与私有 index 指向归档目录，保证驱动读取的是该副本的源码集合。Cargo、独立 MCP、WASM 和 TypeScript 输出均使用副本自己的构建目录，不能跨源码副本共享 `target` 或生成输出。可复用锁定的外部开发依赖，以及由 build script 核对摘要的原生组件；实际验证源码保持不变。

根 workspace 默认 ignored 的测试不等于全部漏测：子进程入口由父测试调用，依赖真实 worker 的测试由 native 组单独执行。历史特定阶段的截图、应用互操作和性能证据仍各自保留。

**本入口的全部通过不代表完整 renderer Native/WASM 一致、全部 PresentationML XSD、Office/WPS 编辑往返、Musterwork 产品替换验收或发行性能验收。** 这些需按各自输入、设备、版本和验收标准执行。纯文档 parity 的结果不能代替渲染 parity。
