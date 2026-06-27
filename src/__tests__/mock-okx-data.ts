import type {
  OkxBalance,
  OkxPosition,
  OkxOrder,
  OkxCandle,
  OkxInstrument,
} from '../services/types'

// ---------------------------------------------------------------------------
// OkxBalance factories
// ---------------------------------------------------------------------------

export function mockOkxBalance(
  overrides?: Partial<OkxBalance>,
): OkxBalance {
  return {
    ccy: 'BTC',
    eq: 2.0,
    cashBal: 1.5,
    availEq: 1.5,
    frozenBal: 0.5,
    ...overrides,
  }
}

export function mockOkxBalanceList(count: number = 3): OkxBalance[] {
  const coins = ['BTC', 'ETH', 'USDT', 'SOL', 'DOGE']
  return Array.from({ length: count }, (_, i) =>
    mockOkxBalance({
      ccy: coins[i % coins.length],
      availEq: 1000 - i * 100,
      frozenBal: i * 10,
      eq: 1000 - i * 90,
    }),
  )
}

// ---------------------------------------------------------------------------
// OkxPosition factories
// ---------------------------------------------------------------------------

export function mockOkxPosition(
  overrides?: Partial<OkxPosition>,
): OkxPosition {
  return {
    instId: 'BTC-USDT',
    pos: 0.5,
    availPos: 0.5,
    avgPx: 50000,
    upl: 1000,
    uplRatio: 0.02,
    ...overrides,
  }
}

export function mockOkxPositionList(count: number = 3): OkxPosition[] {
  const pairs = ['BTC-USDT', 'ETH-USDT', 'SOL-USDT']
  return Array.from({ length: count }, (_, i) =>
    mockOkxPosition({
      instId: pairs[i % pairs.length],
      pos: 0.1 * (i + 1),
      avgPx: 50000 + i * 1000,
      upl: 100 * (i + 1),
    }),
  )
}

// ---------------------------------------------------------------------------
// OkxCandle factories
// ---------------------------------------------------------------------------

export function mockOkxCandle(overrides?: Partial<OkxCandle>): OkxCandle {
  const basePrice = 50000
  return {
    ts: String(Date.now()),
    o: basePrice,
    h: basePrice + 500,
    l: basePrice - 500,
    c: basePrice + 100,
    vol: 1000,
    ...overrides,
  }
}

export function mockOkxCandleList(count: number = 60): OkxCandle[] {
  const now = Date.now()
  let price = 50000
  return Array.from({ length: count }, (_, i) => {
    price += (Math.random() - 0.5) * 1000
    return mockOkxCandle({
      ts: String(now - (count - i) * 3_600_000),
      o: price,
      h: price + Math.random() * 500,
      l: price - Math.random() * 500,
      c: price + (Math.random() - 0.5) * 200,
      vol: 500 + Math.random() * 1500,
    })
  })
}

// ---------------------------------------------------------------------------
// OkxOrder factory
// ---------------------------------------------------------------------------

export function mockOkxOrder(overrides?: Partial<OkxOrder>): OkxOrder {
  return {
    ordId: 'ord-12345',
    clOrdId: 'cl-12345',
    instId: 'BTC-USDT',
    side: 'buy',
    ordType: 'limit',
    sz: 0.1,
    px: 50000,
    state: 'filled',
    avgPx: 50000,
    accFillSz: 0.1,
    uTime: String(Date.now()),
    ...overrides,
  }
}

// ---------------------------------------------------------------------------
// OkxInstrument factories
// ---------------------------------------------------------------------------

export function mockOkxInstrument(
  overrides?: Partial<OkxInstrument>,
): OkxInstrument {
  return {
    instId: 'BTC-USDT',
    instType: 'SPOT',
    uly: '',
    baseCcy: 'BTC',
    quoteCcy: 'USDT',
    ctVal: 0,
    tickSz: '0.1',
    lotSz: 0.0001,
    minSz: 0.0001,
    ...overrides,
  }
}

export function mockOkxInstrumentList(count: number = 15): OkxInstrument[] {
  const pairs = [
    'BTC-USDT', 'ETH-USDT', 'SOL-USDT', 'DOGE-USDT', 'XRP-USDT',
    'ADA-USDT', 'DOT-USDT', 'AVAX-USDT', 'LINK-USDT', 'MATIC-USDT',
    'ATOM-USDT', 'UNI-USDT', 'LTC-USDT', 'BCH-USDT', 'FIL-USDT',
  ]
  return Array.from({ length: Math.min(count, pairs.length) }, (_, i) =>
    mockOkxInstrument({
      instId: pairs[i],
      baseCcy: pairs[i].split('-')[0],
      quoteCcy: pairs[i].split('-')[1],
    }),
  )
}

// ---------------------------------------------------------------------------
// Edge-case helpers
// ---------------------------------------------------------------------------

/** Balance with a large numeric value (> 2^53 boundary for precision tests). */
export function mockLargeNumberBalance(): OkxBalance {
  return {
    ccy: 'BTC',
    eq: 1234567890123456,
    cashBal: 1234567890123456,
    availEq: 1234567890123456,
    frozenBal: 0,
  }
}

export function mockEmptyBalanceList(): OkxBalance[] {
  return []
}

export function mockEmptyPositionList(): OkxPosition[] {
  return []
}
