import { chromium } from 'playwright'
const BASE = 'http://localhost:5176'
const browser = await chromium.launch()
const ctx = await browser.newContext({ viewport: { width: 375, height: 812 } })
await ctx.addInitScript(() => {
  localStorage.setItem('authToken','mock-token'); localStorage.setItem('username','管理员')
  localStorage.setItem('isAuthenticated','true'); localStorage.setItem('theme','light')
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd) => (cmd==='verify_token'?true:{ verify_token:true }[cmd]===undefined?{}:true),
    transformCallback: (cb)=>(cb?1:0), unregisterCallback: ()=>{}, convertFileSrc:(p)=>p,
    listen: async ()=>()=>{}, emit: async ()=>{},
  }
})
const page = await ctx.newPage()
await page.goto('http://localhost:5176/dashboard', { waitUntil:'networkidle' })
await page.waitForTimeout(500)
// Fixed sidebar should be HIDDEN on mobile (<768px)
const asideVisible = await page.$eval('.sidebar', el => getComputedStyle(el).display !== 'none').catch(()=>false)
// Click hamburger (menu-toggle) and check drawer appears
const hasToggle = await page.$('.menu-toggle')
if (hasToggle) await hasToggle.click()
await page.waitForTimeout(500)
const drawerVisible = await page.$eval('.el-drawer', el => {
  const r = el.getBoundingClientRect(); return r.width > 0 && r.left < window.innerWidth
}).catch(()=>false)
const menuItems = await page.$$eval('.el-drawer .el-menu-item', els => els.map(e=>e.textContent.trim()).filter(Boolean))
console.log('desktop sidebar visible (should be no):', asideVisible)
console.log('mobile drawer visible after hamburger (should be yes):', drawerVisible)
console.log('drawer menu items:', menuItems.join(', '))
await page.screenshot({ path: '/tmp/mobile-shots_drawer.png' })
await browser.close()
