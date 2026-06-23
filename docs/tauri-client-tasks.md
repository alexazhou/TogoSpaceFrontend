# TogoClient Tauri 客户端任务拆解

## 0. 当前状态

已完成的基础开发项：

- `external/TogoSpace` 已以 `git submodule` 方式接入
- `apps/desktop` 已创建 `Tauri + Vue 3 + TypeScript + Vue Router + Vue I18n` 工程骨架
- 已按当前仓库所在目录的上级 `TogoSpace/frontend` 结构同步控制台与设置页主结构
- `TogoSpace` HTTP 接口与 `WebSocket` 事件处理已接入
- 已支持托盘、通知和一键启动本地 `TogoSpace` 后端

仍需继续推进的重点：

- 锁定 `TogoSpace` 依赖版本策略
- 进一步优化桌面端交互体验
- 继续补齐桌面增强能力，如剪贴板和文件选择
- 持续验证与上游 frontend 的功能一致性

## 1. 里程碑

### M1：仓库初始化

目标：

- 搭建 `Tauri` 工程骨架
- 重新拉取 `TogoSpace` 子仓库
- 跑通本地开发链路

任务：

- [x] 在 `external/TogoSpace` 建立 `git submodule`
- [x] 在 `apps/desktop` 初始化 `Tauri 2`
- [x] 建立开发环境说明
- [x] 增加启动脚本和环境变量说明

### M2：基础数据接入

目标：

- 拉通 HTTP 接口
- 完成系统状态、房间、消息、成员数据读取

任务：

- [x] 接入 `/system/status.json`
- [x] 接入 `/teams/list.json`
- [x] 接入 `/agents/list.json`
- [x] 接入 `/rooms/list.json`
- [x] 接入 `/rooms/{room_id}/messages/list.json`
- [x] 接入 `/teams/{team_id}/rooms/{room_id}/agents/list.json`

### M3：前端主界面迁移

目标：

- 实现对齐 `frontend` 的控制台与设置页主界面
- 达到主要浏览、发送与管理能力

任务：

- [x] 对齐控制台聊天视图
- [x] 对齐控制台任务视图
- [x] 对齐设置页导航与主要分区
- [x] 对齐消息输入与发送
- [x] 对齐房间、成员与 Agent 详情主流程

### M4：实时同步

目标：

- 接入 WebSocket
- 支持实时消息和状态刷新

任务：

- [x] 接入 `/ws/events.json`
- [x] 处理 `message` 事件
- [x] 处理 `agent_status` 事件
- [x] 处理 `room_status` 事件
- [x] 实现断线重连
- [x] 实现重连后的全量同步

### M5：桌面可用性完善

目标：

- 把客户端从“能跑”提升到“可日常使用”

任务：

- [x] 补齐加载态和空态
- [x] 补齐异常提示
- [x] 优化窗口缩放适配
- [ ] 增加调试日志
- [x] 验证 macOS 基础打包链路
- [x] 接入托盘与关闭到托盘
- [x] 接入系统通知
- [x] 接入一键启动本地后端
- [x] 更新应用版本号与图标

## 2. 推荐开发顺序

建议按照以下顺序实施：

1. 子仓库接入
2. Tauri 工程初始化
3. API 类型与适配层
4. 三栏布局与静态页面
5. 房间与消息读取
6. 发送消息
7. WebSocket 实时同步
8. 重连与错误处理
9. 打包与文档补齐

## 3. 完成标准

满足以下条件即可认为第一阶段完成：

- 可以从桌面端连接到 `TogoSpace` 后端
- 可以看到房间列表、消息流和成员面板
- 可以发送消息
- 可以接收实时消息和状态更新
- 房间未读、消息预览和连接状态可用
- 功能体验与 `TogoSpace` 当前 `tui/` 基本一致
