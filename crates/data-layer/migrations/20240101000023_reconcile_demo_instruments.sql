-- 对齐 demo 标的交易所：OKX → BINANCE。
--
-- 迁移 02（insert_demo_data）在历史提交中被改写为 Binance 版本，但已存在的
-- 开发库可能在改写前应用了旧（OKX）内容。此幂等迁移把残留的 OKX demo 标的
-- 对齐为 BINANCE；新库（直接应用 Binance 版迁移 02）执行此处为无操作。
UPDATE instruments
SET exchange = 'BINANCE'
WHERE symbol IN (
    'BTC-USDT',
    'ETH-USDT',
    'BNB-USDT',
    'SOL-USDT',
    'BTC-USDT-SWAP',
    'ETH-USDT-SWAP'
);
