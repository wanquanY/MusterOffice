# MusterOffice 文档

本仓库是 MusterOffice 产品与内核设计的唯一维护位置。Musterwork 保留项目导航和自身的接入设计，不维护另一份持续演进的内核方案。

## 建议阅读顺序

1. [产品愿景与范围](product/vision.md)：已确认的产品定位和扩展方向。
2. [整体架构](architecture/overview.md)：共享核心与办公领域模块的边界。
3. [决策清单](decisions/README.md)：已确认内容、未决事项和实现准入。
4. [演示文稿详细设计](design/presentations.md)：首个模块的完整评审稿。
5. [来源说明](references/musterwork-baseline.md)：现有平台的历史依据和代码迁移边界。
6. [许可证与分发状态](governance/licensing.md)：开源之前需要完成的事项。

## 文档责任

| 内容                         | 维护位置        |
| ---------------------------- | --------------- |
| 产品名称、愿景、场景和边界   | `product/`      |
| 跨领域架构与宿主职责         | `architecture/` |
| 各领域详细设计与验收         | `design/`       |
| 重要决定和未决问题           | `decisions/`    |
| 外部项目依据、研究和历史事实 | `references/`   |
| 许可、发行与贡献治理         | `governance/`   |

当前不存在已经实现的内核。设计中的目录、协议、性能预算和候选语言，只有对应决定与验收完成后才成为正式产品承诺。
