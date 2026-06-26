-- Create risk_config table for risk management settings
CREATE TABLE IF NOT EXISTS risk_config (
    id SERIAL PRIMARY KEY,
    var_confidence_level DOUBLE PRECISION NOT NULL DEFAULT 0.95,
    max_position_size DOUBLE PRECISION NOT NULL DEFAULT 1000000,
    max_daily_loss DOUBLE PRECISION NOT NULL DEFAULT 100000,
    max_drawdown DOUBLE PRECISION NOT NULL DEFAULT 0.2,
    max_concentration DOUBLE PRECISION NOT NULL DEFAULT 0.3,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO risk_config (id, var_confidence_level, max_position_size, max_daily_loss, max_drawdown, max_concentration)
VALUES (1, 0.95, 1000000, 100000, 0.2, 0.3)
ON CONFLICT (id) DO NOTHING;
