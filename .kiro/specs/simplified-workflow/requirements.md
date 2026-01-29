# 需求文档

## 简介

本功能对 anyFAST 应用进行简化改造，将复杂的手动操作流程简化为单一的"启动/停止"切换按钮。启动后自动执行测速并应用最优结果，停止时自动清除 hosts 绑定。同时增加开机自动启动功能和工作状态视觉反馈。

## 术语表

- **System**: anyFAST 应用系统
- **Dashboard**: 仪表盘界面组件
- **Hosts_Manager**: hosts 文件管理模块
- **Speed_Tester**: 端点测速模块
- **Toggle_Button**: 启动/停止切换按钮
- **Working_Indicator**: 工作状态指示器

## 需求

### 需求 1：开机自动启动

**用户故事：** 作为用户，我希望应用能在系统启动时自动运行，这样我不需要每次手动启动应用。

#### 验收标准

1. THE System SHALL 在 Windows 安装程序中注册开机自动启动项
2. WHEN 系统启动时，THE System SHALL 自动以最小化到托盘的方式启动
3. THE System SHALL 提供设置选项允许用户启用或禁用开机自动启动功能

### 需求 2：简化界面操作

**用户故事：** 作为用户，我希望界面更加简洁，只保留核心功能按钮，这样操作更加直观。

#### 验收标准

1. THE Dashboard SHALL 移除"一键全部应用"按钮
2. THE Dashboard SHALL 移除"清除绑定"按钮
3. THE Dashboard SHALL 将原有的"开始测速"和"停止"按钮替换为单一的"启动/停止"切换按钮
4. WHEN Toggle_Button 处于停止状态时，THE Dashboard SHALL 显示"启动"文字和启动图标
5. WHEN Toggle_Button 处于启动状态时，THE Dashboard SHALL 显示"停止"文字和停止图标

### 需求 3：启动按钮行为

**用户故事：** 作为用户，我希望点击启动按钮后系统自动完成测速和应用最优结果，这样我不需要手动操作多个步骤。

#### 验收标准

1. WHEN 用户点击启动按钮，THE Speed_Tester SHALL 开始对所有启用的端点进行测速
2. WHEN 测速完成后，THE Hosts_Manager SHALL 自动将所有成功测试的端点的最优 IP 应用到 hosts 文件
3. WHEN 应用完成后，THE System SHALL 启动后台健康检查任务持续监控端点状态
4. IF 测速过程中发生错误，THEN THE System SHALL 显示错误提示并保持停止状态

### 需求 4：停止按钮行为

**用户故事：** 作为用户，我希望点击停止按钮后系统立即清除所有 hosts 绑定，这样我可以快速恢复到原始网络状态。

#### 验收标准

1. WHEN 用户点击停止按钮，THE System SHALL 立即停止后台健康检查任务
2. WHEN 停止按钮被点击，THE Hosts_Manager SHALL 清除所有 anyFAST 管理的 hosts 绑定
3. WHEN hosts 绑定清除后，THE System SHALL 刷新 DNS 缓存
4. THE System SHALL 在清除完成后更新界面状态显示

### 需求 5：工作状态视觉反馈

**用户故事：** 作为用户，我希望能够清楚地看到应用当前是否处于工作状态，这样我可以确认优化是否生效。

#### 验收标准

1. WHEN System 处于工作状态时，THE Working_Indicator SHALL 显示脉冲动画效果
2. WHEN System 处于工作状态时，THE Toggle_Button SHALL 显示醒目的活跃状态样式
3. WHEN System 处于停止状态时，THE Working_Indicator SHALL 停止动画并显示静态样式
4. THE Dashboard SHALL 在状态栏区域显示当前工作状态文字提示
