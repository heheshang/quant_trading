-- 插入示例用户 (密码: admin123)
INSERT INTO users (username, password_hash, email, role, full_name)
VALUES (
    'admin',
    '$argon2id$v=19$m=19456,t=2,p=1$zWsHWLEtMXkoMyCRejA+lw$SlK2FE7IwQ9trOmv/NBeQPIKQWgzs359Y/9q7atw73s',
    'admin@example.com',
    'admin',
    'Administrator'
) ON CONFLICT (username) DO NOTHING;

-- 插入常用交易标的
INSERT INTO instruments (symbol, exchange, instrument_type, tick_size) VALUES
    ('BTC-USDT', 'BINANCE', 'Spot', 0.01),
    ('ETH-USDT', 'BINANCE', 'Spot', 0.01),
    ('BNB-USDT', 'BINANCE', 'Spot', 0.001),
    ('SOL-USDT', 'BINANCE', 'Spot', 0.001),
    ('BTC-USDT-SWAP', 'BINANCE', 'Future', 0.1),
    ('ETH-USDT-SWAP', 'BINANCE', 'Future', 0.01)
ON CONFLICT (symbol) DO NOTHING;

-- 为 admin 用户创建模拟账户
DO $$
DECLARE
    admin_user_id BIGINT;
BEGIN
    SELECT user_id INTO admin_user_id FROM users WHERE username = 'admin';
    
    IF admin_user_id IS NOT NULL THEN
        INSERT INTO accounts (user_id, account_type, total_assets, available_cash)
        VALUES (admin_user_id, 'demo', 100000, 100000)
        ON CONFLICT DO NOTHING;
        
        INSERT INTO strategies (strategy_id, user_id, strategy_name, strategy_type, params, enabled, max_position, max_daily_loss)
        VALUES (
            'mean_reversion_001',
            admin_user_id,
            '均值回归策略',
            'MeanReversion',
            '{"period": 20, "std_dev": 2.0, "entry_threshold": 2.0}'::jsonb,
            true,
            100000,
            5000
        ) ON CONFLICT (strategy_id) DO NOTHING;
    END IF;
END $$;
