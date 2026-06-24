# Harness 引导初始化 + Dry Run

> 在拿真实需求使用 Harness 之前，应当用一个虚拟需求完整走一遍全流程。
> 不要期望第一版 Harness 就是完美的，用低成本的方式快速验证、快速修复。

## 引导初始化

### 第 1 步：环境检查

```bash
# 检查必要工具
which git 2>/dev/null || echo "WARN: git missing"

# 自动检测构建工具
source .harness/scripts/detect-build.sh

# 检查项目是否可编译
${BUILD_CHECK_CMD} 2>/dev/null && echo "✅ ${BUILD_TOOL} compile OK" || echo "❌ ${BUILD_TOOL} compile failed"

# 检查项目是否有测试
${TEST_CMD} -q 2>/dev/null && echo "✅ ${BUILD_TOOL} tests OK" || echo "ℹ️  No tests yet"

# 检查 git 仓库
git status --short 2>/dev/null && echo "✅ Git OK" || echo "⚠️  Not a git repo"
```

### 第 2 步：验证 Harness 文件完整性

```bash
# 检查关键文件是否存在
required_files=(
  ".harness/README.md"
  ".harness/platform.md"
  ".harness/agents/initializer-agent.md"
  ".harness/agents/owner-agent.md"
  ".harness/agents/planner-agent.md"
  ".harness/agents/generator-agent.md"
  ".harness/agents/evaluator-agent.md"
  ".harness/rules/arch-rules-rust.md"
  ".harness/rules/coding-rules-rust.md"
  ".harness/rules/quality-gates.md"
  ".harness/rules/workflow-rules.md"
  ".harness/rules/entropy-gc.md"
  ".harness/skills/request-analysis/skill.md"
  ".harness/skills/coding-skill/skill.md"
  ".harness/skills/expert-reviewer/skill.md"
  ".harness/skills/unit-test-write/skill.md"
  ".harness/skills/unit-test-ci/skill.md"
  ".harness/skills/deploy-verify/skill.md"
  ".harness/changes/template/spec.md"
  ".harness/changes/template/tasks.md"
  ".harness/changes/template/summary.md"
  ".harness/changes/template/feature_list.json"
  ".harness/changes/template/contract.md"
  ".harness/changes/template/design.md"     # 新增：技术设计模板
  ".harness/changes/template/review.md"     # 新增：评审报告模板
  ".harness/changes/template/test-plan.md"  # 新增：测试计划模板
  ".harness/changes/template/deploy-log.md" # 新增：部署验证模板
  ".harness/scripts/init.sh"                # 新增：会话启动脚本
  ".harness/scripts/verify-qg.sh"
)

all_ok=true
for f in "${required_files[@]}"; do
  if [ -f "$f" ]; then
    echo "✅ $f"
  else
    echo "❌ MISSING: $f"
    all_ok=false
  fi
done

if $all_ok; then
  echo "✅ All required files present"
else
  echo "❌ Missing files detected. Fix before proceeding."
fi
```

### 第 3 步：创建 Dry Run 变更目录

```bash
# 创建一个虚拟需求的变更目录
DRY_RUN_DIR=".harness/changes/dry-run-initial-setup-$(date +%Y%m%d)"
cp -r .harness/changes/template "$DRY_RUN_DIR"

# 填写虚拟 spec.md
cat > "$DRY_RUN_DIR/spec.md" << 'EOF'
# 需求规格说明书（Dry Run）

## 背景
这是一个虚拟需求，用于验证 Harness 体系的完整性。

## 需求描述
在 PriceService 中新增一个 echo 方法，返回传入的参数。

## 变更范围
- 涉及模块：crates/services
- 涉及代码层：Service
- 不涉及模块：app, handler, repository, client

## 影响分析
无实际影响（这是一个虚拟需求）

## 验收标准
1. Given 调用 echo 方法，When 传入"hello"，Then 返回"hello"
2. Given 调用 echo 方法，When 传入 null，Then 返回 null
EOF
```

## Dry Run 流程

按 owner-agent.md 中的 6 阶段流程逐步推进：

```yaml
阶段 1: 需求分析
  → 读取 spec.md（已创建）
  → 检查完整性（QG-1）
  → 确认

阶段 2: 需求评审
  → Evaluator Agent 评审 spec.md
  → 生成 review-v1.md
  → 如果发现问题，修复后重评

阶段 3: 编码实现
  → Generator Agent 按 spec 实现
  → 编译检查

阶段 4: 编码评审
  → Evaluator Agent 评审代码
  → 如果发现问题，回退到阶段 3

阶段 5: 单元测试编写
  → Generator Agent 编写测试
  → 本地运行通过

阶段 6: CI 验证
  → 触发 CI pipeline（非本地运行测试——本地测试在阶段 5 已完成，CI 使用 detect-build.sh 自动适配）
  → 验证 CI 结果：status == SUCCESS && total_tests > 0 && passed == total
  → 记录结果到 summary.md
```

## Dry Run 检查清单

完成 Dry Run 后，检查以下要点：

- [ ] 所有 6 个阶段都能正常推进
- [ ] 质量门禁正确拦截了不符合条件的产出
- [ ] 评审报告正确生成
- [ ] summary.md 正确记录了每个阶段的状态
- [ ] CI 门禁正确检查了测试覆盖（而非仅检查状态码）
- [ ] 评审报告在简单需求下正确生成文件
- [ ] summary.md 没有因"追加"倾向出现重复行
- [ ] 部署参数没有被 Agent 错误推测
- [ ] 编译环境与 CI 环境一致

## 已知的常见 Dry Run 问题

| 问题 | 表现 | 修复方式 |
|------|------|----------|
| CI 门禁只检查状态码 | status=SUCCESS 但测试数为 0 | 在 QG-5 中增加 `total_tests > 0` 条件 |
| 评审报告不生成 | 简单需求下 Agent 跳过生成 | 在 QG-2 中增加"文件必须存在"检查 |
| summary.md 重复行 | Agent 追加时产生重复 | 在 owner-agent.md 中明确"覆盖而非追加" |
| 部署参数推测错误 | Agent 自行填写了错误的参数 | 阶段 8 增加人工确认点 |

## 初始化完成

Dry Run 通过后，Harness 体系即可投入真实需求使用。
以后每发现一个新错误，就更新 Rules/Skills 来防止它再次发生。
