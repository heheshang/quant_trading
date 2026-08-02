import { describe, it, expect } from 'vitest'
import type {
  OkxBalance,
  OkxPosition,
  OkxCandle,
  OkxOrder,
  OkxInstrument,
  OkxPlaceOrderRequest,
  WsTicker,
  WsTrade,
  WsOrderBook,
} from '@/services/types'
import {
  mockOkxBalance,
  mockOkxBalanceList,
  mockOkxPosition,
  mockOkxCandle,
  mockOkxCandleList,
  mockOkxOrder,
  mockOkxInstrument,
  mockOkxInstrumentList,
  mockLargeNumberBalance,
  mockEmptyBalanceList,
  mockEmptyPositionList,
} from './mock-okx-data'

// ---------------------------------------------------------------------------
// The serialisation pipeline under test:
//
//   Rust type (Decimal / String) → serde_json (JSON string)
//     → Tauri IPC → JSON.parse → TypeScript `number` / `string`
//
// Rust OKX types use `String` for numeric fields (eq, px, etc.) and are
// serialised with #[serde(rename_all = "camelCase")].  TypeScript receives
// the value after JSON.parse coerces it — our interfaces currently declare
// most numeric fields as `number`.
//
// These tests verify runtime behaviour of the TS types, NOT the actual IPC
// layer (covered by integration tests elsewhere).
// ---------------------------------------------------------------------------

// ===========================================================================
// Number precision  (Rust Decimal → JSON string → TS number)
// ===========================================================================

describe('Number precision from Rust Decimal / JSON', () => {
  it('stores values within safe integer range (≤ 2^53)', () => {
    const safe: number = 9007199254740991 // Number.MAX_SAFE_INTEGER
    const balance: OkxBalance = {
      ccy: 'BTC',
      cashBal: safe,
      availEq: safe,
      frozenBal: 0,
      eq: safe,
    }
    expect(balance.eq).toBe(9007199254740991)
    expect(Number.isSafeInteger(balance.eq)).toBe(true)
  })

  it('loses precision for values that exceed 2^53 - 1', () => {
    // Values > Number.MAX_SAFE_INTEGER (~9 007 199 254 740 991) lose
    // integer precision when stored in a double-precision IEEE 754 float.
    // Rust uses Decimal / String to preserve these; TS `number` does NOT.
    const unsafe = 12345678901234567 // > 2^53
    const balance: OkxBalance = {
      ccy: 'BTC',
      cashBal: unsafe,
      availEq: unsafe,
      frozenBal: 0,
      eq: unsafe,
    }
    // IEEE 754 rounds 12345678901234567 → 12345678901234568 when stored:
    expect(Number.isSafeInteger(balance.eq)).toBe(false)
    // The value rounds to the nearest representable float:
    expect(balance.eq).toBe(12345678901234568)
  })

  it('reports isSafeInteger false for MAX_SAFE_INTEGER + 1', () => {
    // The boundary itself — MAX_SAFE_INTEGER + 1 is NOT safe
    const boundary = Number.MAX_SAFE_INTEGER + 1
    expect(Number.isSafeInteger(boundary)).toBe(false)
  })

  it('handles very small decimal values (sub-satoshi)', () => {
    // SHIB-type tokens in Rust use Decimal with high precision;
    // TS number can represent these but may accumulate error.
    const small = 0.00000001
    const balance: OkxBalance = {
      ccy: 'SHIB',
      cashBal: small,
      availEq: small,
      frozenBal: 0,
      eq: small,
    }
    expect(balance.eq).toBeCloseTo(0.00000001, 8)
  })

  it('preserves whole-number decimals (2.0 eq)', () => {
    // Rust JSON may produce 2.0 for a Decimal(2.00); JSON.parse gives 2.
    const rawJson = '{"ccy":"BTC","availEq":2.0,"frozenBal":0,"eq":2.0}'
    const parsed: OkxBalance = JSON.parse(rawJson)
    expect(parsed.eq).toBe(2)
    expect(parsed.availEq).toBe(2)
  })

  it('preserves zero values correctly', () => {
    const balance: OkxBalance = {
      ccy: 'USDT',
      cashBal: 0,
      availEq: 0,
      frozenBal: 0,
      eq: 0,
    }
    expect(balance.availEq).toBe(0)
    expect(Object.is(balance.availEq, 0)).toBe(true) // not -0
  })

  it('handles negative values', () => {
    // PnL / margin fields can be negative
    const pos: OkxPosition = {
      instId: 'BTC-USDT',
      pos: -0.5, // short / negative position
      availPos: 0,
      avgPx: 50000,
      upl: -1500, // unrealised loss
      uplRatio: -0.03,
    }
    expect(pos.pos).toBe(-0.5)
    expect(pos.upl).toBe(-1500)
    expect(pos.upl).toBeLessThan(0)
  })

  it('handles 0.1 + 0.2 floating-point summation realistically', () => {
    // Classic floating-point edge case — our interfaces use `number`,
    // so consumers must handle this when doing arithmetic.
    const feeA = 0.1
    const feeB = 0.2
    const balance: OkxBalance = {
      ccy: 'BTC',
      cashBal: feeA + feeB,
      availEq: feeA + feeB,
      frozenBal: 0,
      eq: 3.0,
    }
    // NOTE: 0.1 + 0.2 = 0.30000000000000004 in IEEE 754
    expect(balance.availEq).toBeCloseTo(0.3, 14)
    // not strictly equal:
    expect(balance.availEq).not.toBe(0.3)
  })

  it('uses mockLargeNumberBalance for precision edge case', () => {
    const balance = mockLargeNumberBalance()
    // The mock factory uses eq: 1234567890123456 which is ~1.23e15,
    // below Number.MAX_SAFE_INTEGER (~9e15), so it IS safe.
    expect(Number.isSafeInteger(balance.eq)).toBe(true)
    expect(balance.eq).toBe(1234567890123456)
  })
})

// ===========================================================================
// Null / undefined handling  (Rust Option::None → JSON null → TS)
// ===========================================================================
// NOTE: All OkxBalance fields are required (`number` not `number | null`).
// If Rust sends `null` for a required field, JSON.parse coerces it to `null`
// at runtime even though TypeScript thinks the type is `number`.

describe('Optional / nullable field handling', () => {
  it('accepts undefined for fields via Partial spread', () => {
    // Partial<OkxBalance> allows any field to be omitted.
    // The factory default fills in the rest.
    const balance: OkxBalance = mockOkxBalance({ ccy: 'ETH' })
    expect(balance.ccy).toBe('ETH')
    expect(balance.availEq).toBe(1.5)
    expect(balance.frozenBal).toBe(0.5)
  })

  it('coerces JSON null to value on required number fields', () => {
    // Simulates Rust sending null on what TS types as `number`.
    // This does NOT throw at runtime — null is a valid JS value.
    const order: OkxOrder = {
      ...mockOkxOrder(),
      px: null as unknown as number, // worst case: Rust Option<Decimal> → null
      avgPx: null as unknown as number,
    }
    expect(order.px).toBeNull()
    expect(order.avgPx).toBeNull()
    // JS coerces null → 0 in arithmetic: null + 1 = 1
    expect(order.px + 1).toBe(1)
  })

  it('coerces JSON null to null on Optional<number> types', () => {
    // The mock factories always set all fields, but real API responses
    // may omit fields that are not applicable (e.g. px for market orders).
    // OkxPlaceOrderRequest.px is optional.
    const req: OkxPlaceOrderRequest = {
      instId: 'BTC-USDT',
      tdMode: 'cash',
      side: 'Buy',
      ordType: 'Market',
      sz: '0.1',
      // px deliberately omitted — TS allows this
    }
    expect(req.px).toBeUndefined()
  })

  it('handles null on string fields', () => {
    // Similar null-coercion hazard on string fields.
    const order: OkxOrder = {
      ...mockOkxOrder(),
      ordId: null as unknown as string,
    }
    expect(order.ordId).toBeNull()
  })
})

// ===========================================================================
// Empty string / whitespace handling  (Rust String vs Option<String>)
// ===========================================================================
// Rust uses String (not Option<String>) for many fields, so empty strings
// can arrive from the API even when the value is conceptually absent.

describe('Empty string handling', () => {
  it('preserves empty string ccy', () => {
    // Edge case: Rust might send an empty ccy under error conditions.
    const balance: OkxBalance = {
      ccy: '',
      cashBal: 0,
      availEq: 0,
      frozenBal: 0,
      eq: 0,
    }
    expect(balance.ccy).toBe('')
    expect(balance.ccy.length).toBe(0)
  })

  it('preserves empty uly on instrument', () => {
    // Spot instruments have no underlying asset; uly is "" from Rust.
    const inst: OkxInstrument = mockOkxInstrument({ uly: '' })
    expect(inst.uly).toBe('')
    expect(inst.uly.length).toBe(0)
  })

  it('handles whitespace-only ccy (untrimmed edge case)', () => {
    const balance: OkxBalance = {
      ccy: '  ',
      cashBal: 0,
      availEq: 0,
      frozenBal: 0,
      eq: 0,
    }
    // Rust should trim, but if it doesn't, TS preserves whitespace.
    expect(balance.ccy.trim().length).toBe(0)
  })
})

// ===========================================================================
// Enum / union type consistency  (matching Rust enum → serde → TS perception)
// ===========================================================================
// The TS types use `string` for sid/ord/ty/state, so all values are accepted
// at runtime.  These tests document the expected OKX API constants.

describe('Enum / union serialization consistency', () => {
  describe('Order side', () => {
    const sides = ['buy', 'sell']

    it('recognises valid OKX order sides', () => {
      expect(sides).toContain('buy')
      expect(sides).toContain('sell')
    })

    it('accepts side from mockOkxOrder', () => {
      const order = mockOkxOrder()
      expect(sides).toContain(order.side)
    })

    it('rejects unknown sides (runtime only, TS string is loose)', () => {
      // TypeScript allows any string, but OKX only accepts buy/sell.
      const allSides = new Set(sides)
      expect(allSides.has('buy')).toBe(true)
      expect(allSides.has('sell')).toBe(true)
      expect(allSides.has('short')).toBe(false)
    })
  })

  describe('Order type', () => {
    const ordTypes = ['market', 'limit', 'post_only', 'fok', 'ioc']

    it('recognises valid OKX order types', () => {
      expect(ordTypes).toContain('market')
      expect(ordTypes).toContain('limit')
      expect(ordTypes).toContain('post_only')
      expect(ordTypes).toContain('fok')
      expect(ordTypes).toContain('ioc')
    })

    it('accepts ordType from mockOkxOrder', () => {
      const order = mockOkxOrder()
      expect(ordTypes).toContain(order.ordType)
    })
  })

  describe('Order state', () => {
    const states = [
      'live',
      'partially_filled',
      'filled',
      'cancelled',
      'failed',
    ]

    it('recognises valid OKX order states', () => {
      expect(states).toContain('live')
      expect(states).toContain('partially_filled')
      expect(states).toContain('filled')
      expect(states).toContain('cancelled')
      expect(states).toContain('failed')
    })

    it('accepts state from mockOkxOrder', () => {
      const order = mockOkxOrder()
      expect(states).toContain(order.state)
    })

    it('accepts all state values', () => {
      for (const state of states) {
        const order = mockOkxOrder({ state })
        expect(order.state).toBe(state)
      }
    })
  })

  describe('Position side', () => {
    it('recognises valid OKX position sides', () => {
      mockOkxPosition()
    })
  })

  describe('Instrument type', () => {
    const instTypes = ['SPOT', 'FUTURES', 'SWAP', 'OPTION', 'ANY']

    it('recognises valid OKX instrument types', () => {
      expect(instTypes).toContain('SPOT')
      expect(instTypes).toContain('FUTURES')
      expect(instTypes).toContain('SWAP')
      expect(instTypes).toContain('OPTION')
    })

    it('accepts instType from mockOkxInstrument', () => {
      const inst = mockOkxInstrument()
      expect(instTypes).toContain(inst.instType)
    })
  })
})

// ===========================================================================
// camelCase / snake_case mapping  (Rust serde rename_all vs TS field names)
// ===========================================================================
// Rust OKX structs use #[serde(rename_all = "camelCase")], so JSON from the
// backend uses camelCase (e.g. "availEq", "frozenBal"). TS interfaces follow
// the same wire contract, so no additional mapping is required.

describe('camelCase / snake_case field mapping', () => {
  it('maps Rust camelCase JSON to TS camelCase directly', () => {
    // Simulated Rust JSON output:
    const rustJson = JSON.stringify({
      ccy: 'BTC',
      eq: '1.5',        // Rust Decimal → String
      cashBal: '1.0',
      availEq: '1.5',
      frozenBal: '0',
    })

    const parsed = JSON.parse(rustJson)

    // Raw parse — fields keep the exact camelCase wire names.
    expect(parsed.avail_eq).toBeUndefined()
    expect(parsed.availEq).toBe('1.5')
    expect(parsed.frozenBal).toBe('0')

    // Explicit mapping step (what a real converter would do):
    const balance: OkxBalance = {
      ccy: parsed.ccy,
      cashBal: parseFloat(parsed.cashBal ?? '0'),
      availEq: parseFloat(parsed.availEq ?? '0'),
      frozenBal: parseFloat(parsed.frozenBal ?? '0'),
      eq: parseFloat(parsed.eq ?? '0'),
    }

    expect(balance.ccy).toBe('BTC')
    expect(balance.availEq).toBe(1.5)
    expect(balance.frozenBal).toBe(0)
    expect(balance.eq).toBe(1.5)
  })

  it('maps Rust camelCase JSON to TS camelCase directly', () => {
    const json = JSON.stringify({
      ccy: 'ETH',
      eq: 2.5,
      cashBal: 2.5,
      availEq: 2.5,
      frozenBal: 0,
    })
    const parsed: OkxBalance = JSON.parse(json)
    expect(parsed.ccy).toBe('ETH')
    expect(parsed.availEq).toBe(2.5)
  })
})

// ===========================================================================
// String → number coercion  (Rust Decimal → JSON string → TS number)
// ===========================================================================
// Rust serialises Decimal as JSON strings.  If the TS layer does NOT parse
// them (e.g. the field is typed `string` in WS types), they remain strings.

describe('String-to-number coercion (Rust Decimal → JSON string)', () => {
  it('parses Rust string-encoded numbers', () => {
    // OKX API returns numeric fields as strings; Rust exchange-okx crate
    // keeps them as strings for precision.  TS must parseFloat them.
    const raw = { eq: '1.23456789', availBal: '1000.50' }
    const eq = parseFloat(raw.eq)
    const availBal = parseFloat(raw.availBal)

    expect(eq).toBe(1.23456789)
    expect(availBal).toBe(1000.50)
  })

  it('parseFloat retains full precision for moderate values', () => {
    const val = parseFloat('1234567890.12345678')
    // IEEE 754 double can represent ~15-17 significant digits
    expect(val).toBe(1234567890.1234567) // last digit may round
    expect(val).toBeCloseTo(1234567890.12345678, 7)
  })

  it('handles parseFloat of negative string', () => {
    const upl = parseFloat('-1500.75')
    expect(upl).toBe(-1500.75)
  })

  it('parseFloat of empty string is NaN', () => {
    const result = parseFloat('')
    expect(result).toBeNaN()
  })
})

// ===========================================================================
// WebSocket string types  (WS types use `string` for numeric fields)
// ===========================================================================
// WsTicker, WsTrade, WsOrderBook, WsCandle use `string` for all numeric
// values — they are inherently precision-safe (no number coercion).

describe('WebSocket string types (precision-safe)', () => {
  it('WsTicker stores all numeric values as strings', () => {
    const ticker: WsTicker = {
      inst_id: 'BTC-USDT',
      last: '50000.123456789',
      last_sz: '0.001',
      ask_px: '50001.00',
      ask_sz: '1.5',
      bid_px: '49999.00',
      bid_sz: '2.0',
      open24h: '48000.00',
      high24h: '51000.00',
      low24h: '47500.00',
      vol24h: '12345.6789',
      ts: '1719000000123',
    }
    // All values are strings → no precision loss
    expect(typeof ticker.last).toBe('string')
    expect(typeof ticker.last_sz).toBe('string')
    expect(typeof ticker.ask_px).toBe('string')
    expect(Number(ticker.last)).toBe(50000.123456789)
    // String length is preserved:
    expect(ticker.last).toBe('50000.123456789')
  })

  it('WsTrade stores px and sz as strings', () => {
    const trade: WsTrade = {
      inst_id: 'BTC-USDT',
      px: '50000.5',
      sz: '0.01',
      side: 'buy',
      ts: '1719000000123',
    }
    expect(typeof trade.px).toBe('string')
    expect(typeof trade.sz).toBe('string')
    // Precision-preserving conversion:
    expect(Number(trade.px)).toBe(50000.5)
  })

  it('WsOrderBook stores bid/ask tuples as [string, string]', () => {
    const ob: WsOrderBook = {
      inst_id: 'BTC-USDT',
      asks: [['50001.0', '1.5'], ['50002.0', '2.0']],
      bids: [['49999.0', '0.5'], ['49998.0', '1.0']],
      ts: '1719000000123',
    }
    expect(ob.asks[0][0]).toBe('50001.0')
    expect(ob.asks[0][1]).toBe('1.5')
  })

  it('converts WS string price to number safely', () => {
    // Downstream consumers often convert WS string prices to number.
    // This test documents the precision boundaries of that conversion.
    const px = Number('50000.123456789')
    expect(px).toBe(50000.123456789)
    // Round-trip through string:
    expect(String(px)).toBe('50000.123456789')
  })

  it('handles large WS string values beyond safe integer', () => {
    // WS types take string, so they never lose precision from number coercion
    // until the consumer explicitly converts with Number().
    const largeVol = '9999999999999999' // > 2^53
    // IEEE 754 rounds 9999999999999999 → 10000000000000000:
    expect(Number(largeVol)).toBe(10000000000000000)
    expect(Number.isSafeInteger(Number(largeVol))).toBe(false)
    // But as a string, it's perfectly preserved:
    expect(largeVol).toBe('9999999999999999')
  })
})

// ===========================================================================
// ID / timestamp string types
// ===========================================================================
// Rust uses String / i64 for timestamps and order IDs.  TS keeps them as
// strings to avoid precision loss on large integers.

describe('ID and timestamp string handling', () => {
  it('preserves large order ID strings', () => {
    // OKX order IDs can be large (> 2^53), so they are typed as string.
    const largeOrdId = '12345678901234567890'
    const order = mockOkxOrder({ ordId: largeOrdId })
    expect(order.ordId).toBe(largeOrdId)
    // Would lose precision as number:
    expect(String(Number(largeOrdId))).not.toBe(largeOrdId)
  })

  it('preserves millisecond timestamp strings', () => {
    // OKX timestamps are millisecond epoch strings.
    const ts = '1719000000123'
    const candle = mockOkxCandle({ ts })
    expect(candle.ts).toBe(ts)
    // As a number it's safe (fits in 2^53), but as string it preserves
    // whatever format OKX returns.
    expect(Number(candle.ts)).toBe(1719000000123)
  })

  it('uTime on OkxOrder is a string', () => {
    const order = mockOkxOrder({ uTime: '1719000000999' })
    expect(typeof order.uTime).toBe('string')
    expect(order.uTime).toBe('1719000000999')
  })
})

// ===========================================================================
// Array and collection edge cases
// ===========================================================================

describe('Array and collection handling', () => {
  it('handles empty balance list', () => {
    const list = mockEmptyBalanceList()
    expect(list).toEqual([])
    expect(list.length).toBe(0)
  })

  it('handles empty position list', () => {
    const list = mockEmptyPositionList()
    expect(list).toEqual([])
    expect(list.length).toBe(0)
  })

  it('handles single-element balance list', () => {
    const list = mockOkxBalanceList(1)
    expect(list).toHaveLength(1)
    expect(list[0].ccy).toBe('BTC')
  })

  it('handles multiple candles from factory', () => {
    const candles = mockOkxCandleList(60)
    expect(candles).toHaveLength(60)
    for (const c of candles) {
      expect(typeof c.ts).toBe('string')
      expect(typeof c.o).toBe('number')
      expect(typeof c.vol).toBe('number')
    }
  })

  it('all candle numeric fields are finite numbers', () => {
    const candle = mockOkxCandle()
    expect(Number.isFinite(candle.o)).toBe(true)
    expect(Number.isFinite(candle.h)).toBe(true)
    expect(Number.isFinite(candle.l)).toBe(true)
    expect(Number.isFinite(candle.c)).toBe(true)
    expect(Number.isFinite(candle.vol)).toBe(true)
  })

  it('handles large instrument list from factory', () => {
    const instruments = mockOkxInstrumentList(15)
    expect(instruments).toHaveLength(15)
    expect(instruments[0].instId).toBe('BTC-USDT')
    expect(instruments[14].instId).toBe('FIL-USDT')
  })
})

// ===========================================================================
// Full round-trip:  object → JSON → parse → TS type
// ===========================================================================
// These tests simulate the actual serialisation path:
//
//   TS object → JSON.stringify → JSON.parse → typed object

describe('JSON round-trip serialisation', () => {
  it('round-trips OkxBalance through JSON', () => {
    const original = mockOkxBalance({ eq: 2.5 })
    const json = JSON.stringify(original)
    const restored: OkxBalance = JSON.parse(json)

    expect(restored.ccy).toBe(original.ccy)
    expect(restored.availEq).toBe(original.availEq)
    expect(restored.eq).toBe(original.eq)
  })

  it('round-trips OkxPosition through JSON', () => {
    const original = mockOkxPosition({ upl: 2500.75 })
    const json = JSON.stringify(original)
    const restored: OkxPosition = JSON.parse(json)

    expect(restored.instId).toBe(original.instId)
    expect(restored.upl).toBe(2500.75)
  })

  it('round-trips OkxOrder through JSON', () => {
    const original = mockOkxOrder({ state: 'live', accFillSz: 0 })
    const json = JSON.stringify(original)
    const restored: OkxOrder = JSON.parse(json)

    expect(restored.ordId).toBe(original.ordId)
    expect(restored.state).toBe('live')
    expect(restored.accFillSz).toBe(0)
  })

  it('JSON.stringify preserves null on number fields', () => {
    const order = {
      ...mockOkxOrder(),
      px: null as unknown as number,
    }
    const json = JSON.stringify(order)
    expect(json).toContain('"px":null')
    const restored = JSON.parse(json)
    expect(restored.px).toBeNull()
  })

  it('JSON.stringify omits undefined optional fields', () => {
    // Fields that are `undefined` are dropped during serialisation.
    const req: OkxPlaceOrderRequest = {
      instId: 'BTC-USDT',
      tdMode: 'cash',
      side: 'Buy',
      ordType: 'Market',
      sz: '0.1',
      // px is undefined
    }
    const json = JSON.stringify(req)
    expect(json).not.toContain('px')
  })

  it('round-trips NaN through JSON (hazard)', () => {
    // JSON.stringify converts NaN to null.
    const balance = mockOkxBalance({ eq: NaN })
    const json = JSON.stringify(balance)
    expect(json).toContain('"eq":null')
    const restored = JSON.parse(json)
    expect(restored.eq).toBeNull()
    expect(restored.eq).not.toBeNaN() // was NaN, now null
  })
})

// ===========================================================================
// TypeScript structural compatibility
// ===========================================================================
// These tests verify that factory output satisfies the interface constraints
// and that extra properties from Rust serialisation are handled.

describe('Structural type compatibility', () => {
  it('mockOkxBalance satisfies OkxBalance interface', () => {
    const balance: OkxBalance = mockOkxBalance()
    expect(balance).toHaveProperty('ccy')
    expect(balance).toHaveProperty('availEq')
    expect(balance).toHaveProperty('frozenBal')
    expect(balance).toHaveProperty('eq')
  })

  it('mockOkxPosition satisfies OkxPosition interface', () => {
    const pos: OkxPosition = mockOkxPosition()
    expect(pos).toHaveProperty('instId')
    expect(pos).toHaveProperty('pos')
    expect(pos).toHaveProperty('avgPx')
    expect(pos).toHaveProperty('upl')
  })

  it('mockOkxOrder satisfies OkxOrder interface', () => {
    const order: OkxOrder = mockOkxOrder()
    expect(order).toHaveProperty('ordId')
    expect(order).toHaveProperty('instId')
    expect(order).toHaveProperty('side')
    expect(order).toHaveProperty('ordType')
    expect(order).toHaveProperty('sz')
    expect(order).toHaveProperty('px')
    expect(order).toHaveProperty('state')
    expect(order).toHaveProperty('accFillSz')
    expect(order).toHaveProperty('avgPx')
    expect(order).toHaveProperty('uTime')
  })

  it('mockOkxCandle satisfies OkxCandle interface', () => {
    const candle: OkxCandle = mockOkxCandle()
    expect(candle).toHaveProperty('ts')
    expect(candle).toHaveProperty('o')
    expect(candle).toHaveProperty('h')
    expect(candle).toHaveProperty('l')
    expect(candle).toHaveProperty('c')
    expect(candle).toHaveProperty('vol')
  })

  it('mockOkxInstrument satisfies OkxInstrument interface', () => {
    const inst: OkxInstrument = mockOkxInstrument()
    expect(inst).toHaveProperty('instId')
    expect(inst).toHaveProperty('instType')
    expect(inst).toHaveProperty('uly')
    expect(inst).toHaveProperty('baseCcy')
    expect(inst).toHaveProperty('quoteCcy')
    expect(inst).toHaveProperty('ctVal')
    expect(inst).toHaveProperty('tickSz')
    expect(inst).toHaveProperty('lotSz')
    expect(inst).toHaveProperty('minSz')
  })

  it('tolerates extra fields from Rust (serde default behaviour)', () => {
    // Rust structs may have additional fields not yet in the TS interface.
    // JSON.parse should not throw on unknown fields.
    const raw = JSON.stringify({
      ccy: 'BTC',
      eq: 2.0,
      cashBal: 1.5,
      frozenBal: 0.5,
      // Extra fields Rust might send:
      notInTs: 'extra',
      anotherExtra: 42,
    })
    expect(() => JSON.parse(raw)).not.toThrow()
    const parsed: OkxBalance = JSON.parse(raw)
    expect(parsed.ccy).toBe('BTC')
    // Extra fields are accessible at runtime even though TS doesn't know:
    expect((parsed as unknown as Record<string, unknown>).notInTs).toBe('extra')
  })
})
