# TogoClient
由于web被浏览器限制，导致功能无法实现，做一个Togo的客户端，方便有更好的操作

## 当前方向

`TogoClient` 当前定位为 `Tauri` 桌面客户端工程。

- 目标：为 `TogoSpace` 提供桌面端入口
- 方式：直接复用上级 `TogoSpace` 仓库，并在桌面端打包时复制一份精简仓库到应用扩展目录
- 范围：当前以当前仓库所在目录的上级 `TogoSpace/frontend` 结构为主要功能基线，并叠加桌面端能力

## 当前进度

已完成首版桌面端工程骨架：

- `apps/desktop`：`Tauri + Vue 3 + TypeScript + Vue Router + Vue I18n` 工程
- `scripts/setup-togospace-submodule.sh`：校验当前上级 `TogoSpace` 仓库路径的兼容脚本
- `scripts/dev-desktop.sh`：本地开发启动脚本
- `scripts/build-desktop.sh`：桌面端一键打包脚本

当前已落地的界面与能力：

- 对齐 `TogoSpace/frontend` 的控制台和设置页结构
- 房间、消息、Agent、任务、团队设置等主要页面能力
- `TogoSpace` HTTP 接口接入
- `WebSocket` 实时事件接入
- 托盘菜单、关闭到托盘、系统通知
- 一键启动/停止本地 `TogoSpace` 后端
- 哈士奇应用图标与 `0.2.0` 版 macOS `.app` / `.dmg` 打包产物

## 开发启动

```bash
./scripts/setup-togospace-submodule.sh
./scripts/dev-desktop.sh
```

源码运行脚本会自动：

- 使用上级 `TogoSpace` 仓库作为默认 `TOGOSPACE_ROOT`
- 补齐 `apps/desktop/.env`
- 执行 `npm install`
- 调用 `tauri dev`
- 仅创建空的 `src-tauri/resources/extensions/TogoSpace` 占位目录，不复制完整仓库
- 将桌面端后端可写数据放到 `frontend/.desktop-dev-storage/`

常用示例：

```bash
# 已安装依赖时跳过 npm install
./scripts/dev-desktop.sh --skip-install

# 透传 tauri dev 参数
./scripts/dev-desktop.sh -- --no-watch
```

## 桌面端打包

直接执行：

```bash
./scripts/build-desktop.sh
```

脚本会自动：

- 使用上级 `TogoSpace` 仓库作为默认 `TOGOSPACE_ROOT`
- 补齐 `apps/desktop/.env`
- 执行 `npm install`
- 调用 `tauri build`
- 在打包时复制一份过滤后的 `TogoSpace` 仓库到应用扩展目录，并排除根目录下的 `frontend/`

常用示例：

```bash
# 只打 dmg
./scripts/build-desktop.sh -- --bundles dmg

# 已安装依赖时跳过 npm install
./scripts/build-desktop.sh --skip-install
```

默认目录结构：

```text
<repo-parent>/TogoSpace
└── frontend
```

## 文档

- [文档索引](./docs/README.md)
- [Tauri 客户端方案](./docs/tauri-client-spec.md)
- [Tauri 客户端检查清单](./docs/tauri-client-checklist.md)
- [Tauri 客户端任务拆解](./docs/tauri-client-tasks.md)
- [桌面端 README](./apps/desktop/README.md)
