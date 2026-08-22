import { chromium } from 'playwright'

/**
 * Mobile ergonomics check (375px): touch-target size, readable font sizes,
 * spacing, and horizontal overflow — quantifies "human visual experience".
 */
const BASE = 'http://localhost:5176'
const routes = ['/dashboard', '/strategy', '/trading', '/binance', '/settings']

const browser = await chromium.launch()
const ctx = await browser.newContext({ viewport: { width: 375, height: 812 } })
await ctx.addInitScript(() => {
  localStorage.setItem('authToken', 'mock-token')
  localStorage.setItem('username', '管理员')
  localStorage.setItem('isAuthenticated', 'true')
  localStorage.setItem('theme', 'light')
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd) => ({ verify_token: true })[cmd] || {},
    transformCallback: (cb) => (cb ? 1 : 0),
    unregisterCallback: () => {},
    convertFileSrc: (p) => p,
    listen: async () => () => {},
    emit: async () => {},
  }
})

console.log('route\toverflow\tbtnH\tinputH\tfontSize\tcardGap')
for (const route of routes) {
  const page = await ctx.newPage()
  await page.goto(BASE + route, { waitUntil: 'networkidle' }).catch(() => {})
  await page.waitForTimeout(600)
  const m = await page.evaluate(() => {
    const vw = window.innerWidth
    const sw = Math.max(document.documentElement.scrollWidth, document.body.scrollWidth)
    const btn = document.querySelector('.el-button')
    const input = document.querySelector('.el-input__inner, .el-select__wrapper')
    const card = document.querySelector('.el-card:not(:first-child)')
    const bodyFont = getComputedStyle(document.body).fontSize
    const cardGap = card ? getComputedStyle(card).marginTop : '0'
    const btnH = btn ? Math.round(btn.getBoundingClientRect().height) : 0
    const inputH = input ? Math.round(input.getBoundingClientRect().height) : 0
    return { overflow: sw > vw, btnH, inputH, bodyFont, cardGap }
  })
  console.log(`${route}\t${m.overflow ? 'YES' : 'no'}\t${m.btnH}\t${m.inputH}\t${m.bodyFont}\t${m.cardGap}`)
  await page.close()
}
await browser.close()
