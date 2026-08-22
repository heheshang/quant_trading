# 移动端截图核对结果

> 无头 Chromium（Playwright）在 **375×812 移动视口**下，对该应用逐页截图并检测横向溢出 / 移动导航结构。
> 通过打桩 `window.__TAURI_INTERNALS__.invoke` + 注入登录态，绕过 Tauri 后端进行**布局**核对（数据为空/占位）。

## 1. 横向溢出检测（关键）

| 页面 | 视口宽 | 页面 scrollWidth | 溢出 |
|------|--------|------------------|------|
| /login | 375 | 375 | ❌ 无 |
| /dashboard | 375 | 375 | ❌ 无 |
| /strategy | 375 | 375 | ❌ 无 |
| /backtest | 375 | 375 | ❌ 无 |
| /trading | 375 | 375 | ❌ 无 |
| /risk | 375 | 375 | ❌ 无 |
| /monitor | 375 | 375 | ❌ 无 |
| /settings | 375 | 375 | ❌ 无 |
| /profile | 375 | 375 | ❌ 无 |
| /binance | 375 | 375 | ❌ 无 |
| /test | 375 | 375 | ❌ 无 |

**结论**：全部 11 页在 375px 下 `scrollWidth == 视口宽`，**无整页横向溢出**。宽表格已在卡片内横向滚动（不撑宽页面）。

### 1b. 暗色模式（theme=dark）

同样在 **375×812** + `html.dark` 生效下复核，**11 页全部无横向溢出**：

| 页面 | viewport | scrollWidth | 溢出 |
|------|----------|-------------|------|
| login / dashboard / … / test（11 页） | 375 | 375 | ❌ 无 |

- `html.dark` 生效：`body` 背景 `rgb(20,20,20)`（#141414），卡片背景同步为暗色。 ✅
- 截图产物：`/tmp/mobile-shots-dark/*.png`。

## 2. 移动导航结构校验（<768px）

| 检查项 | 期望 | 结果 |
|--------|------|------|
| 固定侧边栏（desktop sidebar） | 隐藏 | ✅ hidden |
| 汉堡按钮 → 抽屉 | 弹出 | ✅ 弹出 |
| 抽屉菜单项 | 全部 10 项 | ✅ 仪表盘/策略/回测/交易/风险/监控/设置/账户/币安/测试 |

**结论**：移动端采用「汉堡 + 抽屉导航」，固定侧边栏隐藏，结构正确。

## 3. 截图产物（本地 /tmp，不入库）

```
/mnt/.../mobile-shots_login.png
/mobile-shots_dashboard.png
/mobile-shots_strategy.png
/mobile-shots_backtest.png
/mobile-shots_trading.png
/mobile-shots_risk.png
/mobile-shots_monitor.png
/mobile-shots_settings.png
/mobile-shots_profile.png
/mobile-shots_binance.png
/mobile-shots_test.png
/mobile-shots_drawer.png
```

## 4. 执行方式（复现）

```bash
# 需 dev 服务器 + playwright + chromium
npm run dev                     # 另起 Vite（或复用已有 5176）
npx playwright install chromium # 首次
node scripts/screenshot-mobile.mjs
node scripts/drawer-check.mjs
```

## 5. 局限说明
- 无 Tauri 后端，数据为打桩空值，**核对的是布局/结构**（无重叠、无溢出、导航可用），不含真实数据渲染。
- 真机上建议仍按 `docs/MOBILE_VERIFICATION.md` 清单人工核对一次。
