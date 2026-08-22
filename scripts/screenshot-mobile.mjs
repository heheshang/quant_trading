import { chromium } from 'playwright'
import fs from 'node:fs'

const THEME = process.env.THEME || 'light'
const BASE = 'http://localhost:5176'
const OUT = `/tmp/mobile-shots-${THEME}`
fs.mkdirSync(OUT, { recursive: true })

const routes = [
  '/login', '/dashboard', '/strategy', '/backtest', '/trading',
  '/risk', '/monitor', '/settings', '/profile', '/binance', '/test',
]

const mockCommands = {
  verify_token: () => true,
  get_user_profile: () => ({ id: 0, username: '管理员', role: 'admin' }),
  get_account_info: () => ({ account_id: 1, total_assets: 123456.78, available_cash: 65432.1, frozen_cash: 0, market_value: 58024.68, total_pnl: 1234.56, daily_pnl: 12.34, margin: 0, margin_ratio: 0, updated_at: '2026-01-01T00:00:00Z' }),
  get_positions: () => [],
  get_active_orders: () => [],
  get_strategies: () => [],
  list_strategy_types: () => [],
  get_metrics: () => ({}),
  get_alerts: () => [],
  get_logs: () => [],
  get_risk_metrics: () => ({}),
  get_okx_balance: () => [],
  get_binance_balance: () => [],
  get_okx_positions: () => [],
  get_okx_instruments: () => [],
  get_okx_announcements: () => [],
  get_okx_candles: () => [],
  get_binance_candles: () => [],
  get_okx_realtime_data: () => ({}),
  get_binance_order_book: () => ({ bids: [], asks: [], symbol: '' }),
  get_backtest_results: () => [],
  get_binance_subscriptions: () => [],
  get_subscriptions: () => [],
  check_okx_status: () => ({ connected: false }),
  check_binance_status: () => ({ connected: false }),
  get_config: () => ({}),
}

const browser = await chromium.launch()
const context = await browser.newContext({
  viewport: { width: 375, height: 812 },
  deviceScaleFactor: 1,
})
await context.addInitScript((cmds) => {
  localStorage.setItem('authToken', 'mock-token')
  localStorage.setItem('username', '管理员')
  localStorage.setItem('isAuthenticated', 'true')
  localStorage.setItem('theme', cmds.theme)

  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => (cmds[cmd] || (() => ({})))(args),
    transformCallback: (cb) => (cb ? 1 : 0),
    unregisterCallback: () => {},
    convertFileSrc: (p) => p,
    listen: async () => () => {},
    emit: async () => {},
  }
}, { ...mockCommands, theme: THEME })

console.log('route\tviewportWidth\tscrollWidth\toverflow')
for (const route of routes) {
  const page = await context.newPage()
  try {
    await page.goto(BASE + route, { waitUntil: 'networkidle', timeout: 15000 })
    await page.waitForTimeout(700) // let lazy UI settle
    const { vw, sw, sw2 } = await page.evaluate(() => ({
      vw: window.innerWidth,
      sw: document.documentElement.scrollWidth,
      sw2: document.body.scrollWidth,
    }))
    const overflow = Math.max(sw, sw2) > vw
    console.log(`${route}\t${vw}\t${Math.max(sw, sw2)}\t${overflow ? 'YES' : 'no'}`)
    await page.screenshot({ path: `${OUT}/${route.replace(/\//g, '_') || '_root'}.png` })
  } catch (e) {
    console.log(`${route}\tERROR\t${e.message.slice(0, 60)}`)
  }
  await page.close()
}

await browser.close()
console.log('done -> screenshots in', OUT)
