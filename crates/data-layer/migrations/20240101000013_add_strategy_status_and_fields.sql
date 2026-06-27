-- 策略管理全链路优化：修复所有 schema 不一致
-- 迁移编号：0013

-- 1. 新增自增 id 列（原表只有 strategy_id 作为 PK）
ALTER TABLE strategies ADD COLUMN IF NOT EXISTS id SERIAL;

-- 2. 将 id 设为 PK（如果尚未设置）
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'strategies_pkey_id'
  ) THEN
    ALTER TABLE strategies ADD CONSTRAINT strategies_pkey_id PRIMARY KEY (id);
  END IF;
EXCEPTION
  WHEN duplicate_object THEN NULL;
END $$;

-- 3. 新增 status/description 列
ALTER TABLE strategies
  ADD COLUMN IF NOT EXISTS status VARCHAR(20) NOT NULL DEFAULT 'Draft',
  ADD COLUMN IF NOT EXISTS description TEXT;

-- 4. 修复 tags/symbols 类型：TEXT[] → JSONB（匹配 Rust serde_json::Value）
DO $$
BEGIN
  -- 删除 TEXT[] 类型的列（如果存在）
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'strategies' AND column_name = 'tags'
    AND udt_name = '_text'
  ) THEN
    ALTER TABLE strategies DROP COLUMN tags;
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'strategies' AND column_name = 'symbols'
    AND udt_name = '_text'
  ) THEN
    ALTER TABLE strategies DROP COLUMN symbols;
  END IF;
END $$;

-- 添加 JSONB 类型的列
ALTER TABLE strategies ADD COLUMN IF NOT EXISTS tags JSONB DEFAULT '[]'::jsonb;
ALTER TABLE strategies ADD COLUMN IF NOT EXISTS symbols JSONB DEFAULT '[]'::jsonb;

-- 5. 处理 user_id：设为可为空（代码中未使用）
ALTER TABLE strategies ALTER COLUMN user_id DROP NOT NULL;

-- 6. 索引
CREATE INDEX IF NOT EXISTS idx_strategies_status ON strategies(status);
CREATE INDEX IF NOT EXISTS idx_strategies_type_status ON strategies(strategy_type, status);

-- 7. 基于现有 enabled 列回填 status
UPDATE strategies SET status = 'Running' WHERE enabled = true AND (status = 'Draft' OR status IS NULL);
UPDATE strategies SET status = 'Archived' WHERE enabled = false AND (status = 'Draft' OR status IS NULL);
