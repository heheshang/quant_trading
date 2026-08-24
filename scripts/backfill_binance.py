#!/usr/bin/env python3
"""Backfill Binance historical 1h klines into the app's market_data table.

Source: production Binance public REST (no auth).
Target: Postgres `market_data` (partitioned by month), reached via the
quant-trading-postgres-1 docker container.

Usage:
  python3 scripts/backfill_binance.py --yesterday days=365 --timeframe 1h \
      --symbols top50            # top 50 USDT pairs by 24h quote volume
  python3 scripts/backfill_binance.py --symbols BTCUSDT
"""
import argparse
import datetime
import json
import subprocess
import time
import urllib.request

BASE = "https://api.binance.com/api/v3"
PG = ["docker", "exec", "-i", "quant-trading-postgres-1", "psql",
      "-U", "quant", "-d", "quant_trading", "-v", "ON_ERROR_STOP=1"]
STABLE_QUOTES = ("USDT",)
# Exclude leveraged / stable-coin / algorithmic-stable pairs from the universe.
BAD_SUBSTR = ("UPUSDT", "DOWNUSDT", "USDCUSDT", "FDUSDUSDT", "TUSDUSDT",
              "DAIUSDT", "USDPUSDT", "EURUSDT", "TRYUSDT", "BUSDUSDT", "EURIUSDT")


def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "quant/1.0"})
    with urllib.request.urlopen(req, timeout=40) as r:
        return json.loads(r.read())


def top_usdt_symbols(n=50):
    """Return the top `n` USDT-quoted symbols by 24h quote volume."""
    data = get(f"{BASE}/ticker/24hr")
    candidates = [
        s for s in data
        if s.get("symbol", "").endswith("USDT")
        and not any(b in s["symbol"] for b in BAD_SUBSTR)
        and float(s.get("quoteVolume", 0) or 0) > 0
    ]
    candidates.sort(key=lambda s: float(s["quoteVolume"]), reverse=True)
    return [s["symbol"] for s in candidates[:n]]


def to_domain(binance_symbol):
    q = "USDT"
    if binance_symbol.endswith(q):
        return f"{binance_symbol[:-len(q)]}-{q}"
    return binance_symbol


def fetch_klines(symbol, interval, start_ms, end_ms):
    """Paginate Binance klines from start_ms to end_ms (≤1000 per call)."""
    rows = []
    cur = start_ms
    while cur < end_ms:
        url = (f"{BASE}/klines?symbol={symbol}&interval={interval}"
               f"&startTime={cur}&endTime={end_ms}&limit=1000")
        batch = get(url)
        if not batch:
            break
        rows.extend(batch)
        if len(batch) < 1000:
            break
        nxt = batch[-1][0] + 1
        if nxt <= cur:
            break
        cur = nxt
        time.sleep(0.12)  # stay under rate limits
    return rows


def insert_records(domain, timeframe, rows):
    """Stream rows into market_data via psql \copy (CSV)."""
    if not rows:
        return 0
    csv_lines = []
    for r in rows:
        ts_ms = r[0]
        ts = datetime.datetime.fromtimestamp(ts_ms / 1000, tz=datetime.timezone.utc)
        ts_str = ts.strftime("%Y-%m-%d %H:%M:%S+00")
        csv_lines.append(
            f'{domain},{timeframe},{ts_str},{r[1]},{r[2]},{r[3]},{r[4]},{r[5]}'
        )
    payload = "\n".join(csv_lines) + "\n"
    cmd = PG + [
        "-c",
        "\\copy market_data (instrument_id,timeframe,timestamp,open,high,low,close,volume) "
        "FROM STDIN WITH CSV",
    ]
    proc = subprocess.run(cmd, input=payload.encode(), capture_output=True)
    if proc.returncode != 0:
        raise RuntimeError(f"copy failed for {domain}: {proc.stderr.decode()[-800:]}")
    return len(csv_lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--symbols", default="top50",
                    help="'top50' or comma-separated Binance symbols, e.g. BTCUSDT,ETHUSDT")
    ap.add_argument("--timeframe", default="1h")
    ap.add_argument("--days", type=int, default=365)
    ap.add_argument("--universe", type=int, default=50, help="top-N size when --symbols=top50")
    args = ap.parse_args()

    if args.symbols.lower() == "top50":
        symbols = top_usdt_symbols(args.universe)
        print(f"universe: {len(symbols)} symbols", flush=True)
    else:
        symbols = [s.strip().upper() for s in args.symbols.split(",") if s.strip()]

    end_ms = int(time.time() * 1000)
    start_ms = end_ms - args.days * 86400_000
    total = 0
    fetch_tf = args.timeframe.lower()   # Binance API interval (e.g. "1d")
    store_tf = args.timeframe.upper()   # market_data canonical (e.g. "1D")
    for sym in symbols:
        domain = to_domain(sym)
        try:
            rows = fetch_klines(sym, fetch_tf, start_ms, end_ms)
            n = insert_records(domain, store_tf, rows)
            total += n
            print(f"{sym} -> {domain}: {n} candles (total {total})", flush=True)
        except Exception as e:  # noqa: BLE001
            print(f"{sym} FAILED: {e}", flush=True)
    print(f"DONE. inserted {total} rows.", flush=True)


if __name__ == "__main__":
    main()
