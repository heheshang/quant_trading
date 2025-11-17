-- 插入示例用户 (密码: admin123)
INSERT INTO users (username, password_hash, email, roles)
VALUES (
    'admin',
    '$argon2id$v=19$m=19456,t=2,p=1$VE5FVEVTVFNBTFQ$8jqS3nB5V5xqY9Z3k2nF0A', -- 需要使用实际的 argon2 哈希
    'admin@example.com',
    ARRAY['admin', 'trader']
) ON CONFLICT (username) DO NOTHING;

-- 插入常用交易标的
INSERT INTO instruments (symbol, exchange, instrument_type, tick_size) VALUES
    ('BTC-USDT', 'OKX', 'Spot', 0.01),
    ('ETH-USDT', 'OKX', 'Spot', 0.01),
    ('BNB-USDT', 'OKX', 'Spot', 0.001),
    ('SOL-USDT', 'OKX', 'Spot', 0.001),
    ('BTC-USDT-SWAP', 'OKX', 'Future', 0.1),
    ('ETH-USDT-SWAP', 'OKX', 'Future', 0.01)
ON CONFLICT (symbol) DO NOTHING;

-- 为 admin 用户创建模拟账户
DO $$
DECLARE
    admin_user_id UUID;
BEGIN
    SELECT id INTO admin_user_id FROM users WHERE username = 'admin';
    
    IF admin_user_id IS NOT NULL THEN
        INSERT INTO accounts (user_id, account_type, total_assets, available_cash)
        VALUES (admin_user_id, 'demo', 100000, 100000)
        ON CONFLICT DO NOTHING;
    END IF;
END $$;
