-- 策略管理全链路优化：修复所有 schema 不一致
-- 迁移编号：0013

-- 1. 新增自增 id 列（业务查询使用 strategy_id，id 作为稳定行标识）
ALTER TABLE strategies ADD COLUMN IF NOT EXISTS id SERIAL;

-- 2. 为 id 建立唯一约束；保留 strategy_id 主键，避免全新库重复添加主键。
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
      FROM pg_constraint c
      JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)
     WHERE c.conrelid = 'strategies'::regclass
       AND c.contype IN ('p', 'u')
       AND a.attname = 'id'
  ) THEN
    ALTER TABLE strategies ADD CONSTRAINT strategies_id_key UNIQUE (id);
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
