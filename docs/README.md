# TogoClient 文档索引

本文档目录用于承载 `TogoClient` 的方案、实施清单和任务拆解。

## 当前文档

- [Tauri 客户端方案](./tauri-client-spec.md)
- [Tauri 客户端检查清单](./tauri-client-checklist.md)
- [Tauri 客户端任务拆解](./tauri-client-tasks.md)
- [桌面端 README](../apps/desktop/README.md)

## 文档范围

当前这组文档聚焦以下目标：

- 在 `TogoClient` 内新增一个桌面客户端工程，技术栈使用 `Tauri`
- 以子仓库方式重新引入 `TogoSpace`
- 第一阶段功能与 `TogoSpace` 当前 `tui/` 实现保持一致
- 为后续进入编码阶段提供明确的模块边界和开发顺序

## 当前实现

目前仓库中已经创建：

- `external/TogoSpace` 子仓库
- `apps/desktop` 首版桌面客户端骨架
- `scripts/setup-togospace-submodule.sh` 子仓库初始化脚本
- `scripts/dev-desktop.sh` 本地开发启动脚本
