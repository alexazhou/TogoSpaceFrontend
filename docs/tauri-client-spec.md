# TogoClient Tauri 客户端方案

## 1. 背景

`TogoSpace` 目前已经具备：

- Tornado 后端
- 基于 Vue 的 Web 控制台
- 基于 Textual 的 TUI 观察端
- WebSocket 实时事件流

当前 `TogoClient` 的目标不是重做一套新的业务后端，而是构建一个桌面客户端，解决浏览器环境下的限制问题，并提供更稳定的桌面交互入口。

## 2. 项目目标

本项目第一阶段目标如下：

- 在 `TogoClient` 中构建一个基于 `Tauri` 的桌面客户端
- 通过“子仓库”方式重新拉取并固定 `TogoSpace`
- 客户端功能与 `TogoSpace` 当前 `tui/` 的能力保持一致
- 保持与 `TogoSpace` 现有 HTTP 和 WebSocket 接口兼容
- 为后续扩展桌面能力预留壳层能力，例如窗口、系统通知、文件选择、剪贴板

## 3. 关键约束

- 第一阶段只要求“与 TUI 一致”，不要求立即覆盖 `TogoSpace` Web 控制台全部能力
- 业务数据仍以 `TogoSpace` 后端为准，不在 `TogoClient` 内重复实现调度、房间、消息和 Agent 核心逻辑
- 桌面端优先走“连接现有后端”的模式，避免在第一阶段就把后端打包、安装、升级问题混在一起
- 子仓库采用 `git submodule` 作为默认方案，除非后续明确切换为 `git subtree`

## 4. 总体方案

### 4.1 技术选型

- 桌面壳层：`Tauri 2`
- 前端界面：`Vue 3 + TypeScript + Vite`
- 壳层语言：`Rust`
- 网络通信：`fetch/axios + WebSocket`
- 状态管理：`Pinia`
- UI 组织方式：组件化三栏布局

选择 `Tauri` 而不是 `PyQt5` 的主要原因：

- 更适合承载现代前端交互
- 可直接复用 Web 技术栈，开发效率更高
- 后续接入系统能力时比浏览器更自由
- 安装包体积、跨平台能力和桌面集成体验较均衡

### 4.2 仓库结构建议

建议将 `TogoClient` 组织为如下结构：

```text
TogoClient/
├── README.md
├── docs/
├── external/
│   └── TogoSpace/              # git submodule
├── apps/
│   └── desktop/
│       ├── src/               # 前端页面与状态管理
│       ├── src-tauri/         # Tauri Rust 壳层
│       ├── public/
│       ├── package.json
│       └── tauri.conf.json
└── scripts/
```

说明：

- `external/TogoSpace` 作为子仓库，负责同步上游代码
- `apps/desktop` 是实际可交付的桌面客户端工程
- `docs/` 只承载设计、计划和实施文档

## 5. 子仓库策略

### 5.1 推荐方式

使用 `git submodule` 将 `TogoSpace` 引入到：

```text
external/TogoSpace
```

推荐原因：

- 明确区分上游源码与本仓客户端代码
- 便于固定 `TogoSpace` 版本
- 便于后续跟进上游更新
- 不会把外部项目历史直接混入当前仓库主线

### 5.2 约束规则

- `TogoClient` 对 `external/TogoSpace` 的修改应尽量最小化
- 若需改动 `TogoSpace`，优先通过上游 PR 或补丁层处理
- `TogoClient` 内部文档与脚本必须明确依赖的 `TogoSpace` 版本

## 6. 功能对齐范围

第一阶段以 `TogoSpace` 当前 `tui/` 的真实能力为基线，而不是以 README 中过时描述为准。

### 6.1 必须覆盖的功能

- 房间列表展示
- 房间上下切换
- 当前房间消息列表加载
- 发送消息到当前可发送房间
- 团队成员列表展示
- Agent 状态展示
- 房间未读数展示
- 房间最后一条消息预览
- 系统连接状态展示
- WebSocket 实时刷新消息
- WebSocket 实时刷新 Agent 状态
- WebSocket 实时刷新房间状态
- 后端不可达时的错误提示
- 断线后的重连机制

### 6.2 可以延后到第二阶段的能力

- 多窗口
- 系统托盘
- 系统通知
- 文件拖拽上传
- 本地日志查看器
- 内置后端启动与停止
- 与 `TogoSpace` Web 控制台能力对齐

## 7. 架构设计

### 7.1 分层结构

客户端分为四层：

1. 展示层
   - 页面、组件、布局、交互反馈
2. 状态层
   - 房间列表、当前房间、消息、成员、连接状态、未读计数
3. 服务层
   - HTTP API 调用、WebSocket 事件消费、错误处理、重连策略
4. 壳层能力层
   - Tauri 窗口、系统通知、文件系统、剪贴板、未来的托盘能力

### 7.2 数据来源

第一阶段直接对接 `TogoSpace` 已有接口：

- `/system/status.json`
- `/teams/list.json`
- `/agents/list.json`
- `/rooms/list.json`
- `/rooms/{room_id}/messages/list.json`
- `/rooms/{room_id}/messages/send.json`
- `/teams/{team_id}/rooms/{room_id}/agents/list.json`
- `/ws/events.json`

### 7.3 页面布局

桌面端建议采用三栏布局：

- 左栏：房间列表
- 中栏：消息列表 + 输入框
- 右栏：房间成员与 Agent 状态

底部保留状态栏，展示：

- 连接状态
- 当前房间消息计数
- 当前同步状态

### 7.4 状态模型

建议最少维护以下状态：

- `systemStatus`
- `rooms`
- `currentRoomId`
- `currentRoomKey`
- `messagesByRoom`
- `agents`
- `roomMembersByRoom`
- `unreadByRoom`
- `connectionState`
- `wsReconnectState`

### 7.5 事件模型

需要消费的 WebSocket 事件包括：

- `message`
- `agent_status`
- `room_status`

客户端收到事件后需要：

- 更新当前房间消息
- 更新目标房间未读数
- 更新 Agent 状态
- 更新房间状态和当前轮次 Agent
- 在连接恢复后执行一次全量同步

## 8. 模块拆分

建议 `apps/desktop/src` 采用如下模块划分：

```text
src/
├── app/
├── pages/
├── components/
├── stores/
├── services/
├── models/
├── composables/
├── utils/
└── styles/
```

模块职责如下：

- `pages/`
  - 页面容器，例如主工作台
- `components/`
  - 房间列表、消息流、输入框、成员面板、状态栏
- `stores/`
  - 维护全局状态和领域状态
- `services/`
  - HTTP 客户端、WebSocket 客户端、重连、错误适配
- `models/`
  - Team、Room、Message、Agent、WsEvent 类型定义
- `composables/`
  - UI 交互复用逻辑
- `utils/`
  - 时间、字符串、错误信息、节流等基础工具

## 9. 迭代计划

### 9.1 Phase 0：工程初始化

- 初始化 `Tauri` 桌面工程
- 建立 `external/TogoSpace` 子仓库
- 补齐本地开发脚本
- 明确 `.env` 或桌面配置的后端地址来源

### 9.2 Phase 1：TUI 功能等价

- 实现房间列表、消息流、成员面板、状态栏
- 完成 HTTP 拉取和 WebSocket 同步
- 完成未读、预览、连接状态和发送消息
- 完成断线重连和首次全量同步

### 9.3 Phase 2：桌面能力增强

- 系统通知
- 托盘
- 快捷键
- 剪贴板
- 文件系统接入

### 9.4 Phase 3：深度集成

- 一键启动本地 `TogoSpace` 后端
- 检查依赖版本和运行状态
- 提供诊断页和日志页

## 10. 风险与应对

### 10.1 上游接口波动

风险：

- `TogoSpace` 的接口返回结构可能演进

应对：

- 在 `services/` 层建立统一适配
- 为关键接口建立最小契约测试

### 10.2 TUI 与真实业务并非严格等同

风险：

- TUI 是观察端视角，未来用户可能要求补齐更多管理能力

应对：

- 第一阶段只承诺 TUI 等价
- 文档中单独维护“后续增强范围”

### 10.3 桌面端与后端版本错配

风险：

- `TogoClient` 与子仓库中 `TogoSpace` 版本不匹配

应对：

- 固定 submodule commit
- 在客户端启动时显示兼容版本信息

## 11. 交付物

本阶段交付物应包含：

- `TogoClient` 文档
- `Tauri` 桌面工程骨架
- `TogoSpace` 子仓库
- TUI 等价功能的首版桌面界面
- 本地开发和构建说明

## 12. 当前决策

当前文档先采用以下决策：

- UI 技术栈改为 `Tauri`
- 前端框架采用 `Vue 3 + TypeScript`
- 子仓库方案采用 `git submodule`
- 第一阶段范围对齐 `TogoSpace` 当前 `tui/`
- 后端先以“外部已启动服务”方式接入

若后续你希望把“子仓库”改成 `subtree`，或者希望第一阶段直接内置启动 `TogoSpace` 后端，可以在此文档基础上再收敛一版实施方案。
