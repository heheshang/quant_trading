import { chromium } from 'playwright'
const browser = await chromium.launch()
const ctx = await browser.newContext({ viewport:{width:375,height:812} })
await ctx.addInitScript(() => {
  localStorage.setItem('authToken','mock-token'); localStorage.setItem('username','管理员')
  localStorage.setItem('isAuthenticated','true'); localStorage.setItem('theme','dark')
  window.__TAURI_INTERNALS__ = { invoke:async(cmd)=>({verify_token:true})[cmd]||{}, transformCallback:(cb)=>(cb?1:0), unregisterCallback:()=>{}, convertFileSrc:(p)=>p, listen:async()=>()=>{}, emit:async()=>{} }
})
const p = await ctx.newPage()
await p.goto('http://localhost:5176/dashboard', { waitUntil:'networkidle' })
await p.waitForTimeout(500)
const res = await p.evaluate(() => {
  const htmlDark = document.documentElement.classList.contains('dark')
  const bodyBg = getComputedStyle(document.body).backgroundColor
  const cardBg = getComputedStyle(document.querySelector('.el-card') || document.body).backgroundColor
  return { htmlDark, bodyBg, cardBg }
})
console.log('html.dark active:', res.htmlDark)
console.log('body bg:', res.bodyBg)
console.log('card bg:', res.cardBg)
await browser.close()
