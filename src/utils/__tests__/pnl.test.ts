import { describe, it, expect } from 'vitest'
import { computeRealizedPnl, type FilledFill } from '../pnl'

const f = (
  side: 'Buy' | 'Sell',
  price: number,
  quantity: number,
  ts: number,
  symbol = 'BTC-USDT',
): FilledFill => ({ symbol, side, price, quantity, ts })

describe('computeRealizedPnl (FIFO)', () => {
  it('basic buy then sell', () => {
    const fills = [f('Buy', 100, 1, 1), f('Sell', 130, 1, 2)]
    expect(computeRealizedPnl(fills)).toBeCloseTo(30)
  })

  it('FIFO pairs sell against EARLIEST lots first', () => {
    // buy 1@100 (lot A), buy 1@120 (lot B), sell 1.5@130
    // FIFO: 1 of lot A (130-100)=30; 0.5 of lot B (130-120)*0.5=5 → total 35
    const fills = [f('Buy', 100, 1, 1), f('Buy', 120, 1, 2), f('Sell', 130, 1.5, 3)]
    expect(computeRealizedPnl(fills)).toBeCloseTo(35)
  })

  it('sell exceeding held uses remaining qty at full proceeds', () => {
    // buy 1@100, sell 2@130 → 30 (matched) + 130 (excess) = 160
    const fills = [f('Buy', 100, 1, 1), f('Sell', 130, 2, 2)]
    expect(computeRealizedPnl(fills)).toBeCloseTo(160)
  })

  it('respects per-symbol isolation', () => {
    const fills = [f('Buy', 100, 1, 1, 'BTC-USDT'), f('Buy', 2000, 1, 2, 'ETH-USDT'), f('Sell', 110, 1, 3, 'BTC-USDT')]
    expect(computeRealizedPnl(fills)).toBeCloseTo(10)
  })

  it('empty / no sells → zero', () => {
    expect(computeRealizedPnl([])).toBe(0)
    expect(computeRealizedPnl([f('Buy', 100, 1, 1)])).toBe(0)
  })
})
