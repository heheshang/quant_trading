<template>
  <div class="system-settings">
    <el-row :gutter="20" class="header">
      <el-col :span="24">
        <h2>系统设置</h2>
      </el-col>
    </el-row>

    <el-tabs v-model="activeTab" class="settings-tabs">
      <!-- 基本设置 -->
      <el-tab-pane label="基本设置" name="basic">
        <el-card class="settings-card">
          <template #header>
            <div class="card-header">
              <span>基本配置</span>
            </div>
          </template>
          
          <el-form ref="systemInfoFormRef" :model="systemInfo" :rules="systemInfoRules" label-width="150px">
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="系统名称" prop="name">
                  <el-input v-model="systemInfo.name" placeholder="输入系统名称" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="系统版本" prop="version">
                  <el-input v-model="systemInfo.version" placeholder="输入系统版本" readonly />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="语言" prop="language">
                  <el-select v-model="systemInfo.language" placeholder="选择语言" style="width: 100%">
                    <el-option label="中文" value="zh-CN" />
                    <el-option label="English" value="en-US" />
                  </el-select>
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="时区" prop="timezone">
                  <el-select v-model="systemInfo.timezone" placeholder="选择时区" style="width: 100%">
                    <el-option label="UTC" value="UTC" />
                    <el-option label="UTC+8 (北京)" value="UTC+8" />
                    <el-option label="UTC-5 (纽约)" value="UTC-5" />
                  </el-select>
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </el-card>
      </el-tab-pane>
      
      <!-- 数据库设置 -->
      <el-tab-pane label="数据库" name="database">
        <el-card class="settings-card">
          <template #header>
            <div class="card-header">
              <span>数据库配置</span>
            </div>
          </template>
          
          <el-form ref="databaseFormRef" :model="config.database" :rules="databaseRules" label-width="120px">
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="主机地址" prop="host">
                  <el-input v-model="config.database.host" placeholder="输入数据库主机地址" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="端口" prop="port">
                  <el-input-number 
                    v-model="config.database.port" 
                    :min="1" 
                    :max="65535" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="用户名" prop="username">
                  <el-input v-model="config.database.username" placeholder="输入数据库用户名" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="密码" prop="password">
                  <el-input 
                    v-model="config.database.password" 
                    type="password" 
                    placeholder="输入数据库密码" 
                    show-password
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="数据库名" prop="database">
                  <el-input v-model="config.database.database" placeholder="输入数据库名" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="最大连接数" prop="max_connections">
                  <el-input-number 
                    v-model="config.database.max_connections" 
                    :min="1" 
                    :max="1000" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </el-card>
      </el-tab-pane>
      
      <!-- 缓存设置 -->
      <el-tab-pane label="缓存" name="redis">
        <el-card class="settings-card">
          <template #header>
            <div class="card-header">
              <span>Redis配置</span>
            </div>
          </template>
          
          <el-form ref="redisFormRef" :model="config.redis" :rules="redisRules" label-width="120px">
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="主机地址" prop="host">
                  <el-input v-model="config.redis.host" placeholder="输入Redis主机地址" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="端口" prop="port">
                  <el-input-number 
                    v-model="config.redis.port" 
                    :min="1" 
                    :max="65535" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="密码" prop="password">
                  <el-input 
                    v-model="config.redis.password" 
                    type="password" 
                    placeholder="输入Redis密码（可选）" 
                    show-password
                  />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="数据库" prop="db">
                  <el-input-number 
                    v-model="config.redis.db" 
                    :min="0" 
                    :max="15" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="连接池大小" prop="pool_size">
                  <el-input-number 
                    v-model="config.redis.pool_size" 
                    :min="1" 
                    :max="100" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </el-card>
      </el-tab-pane>
      
      <!-- 时序数据库设置 -->
      <el-tab-pane label="时序数据库" name="influxdb">
        <el-card class="settings-card">
          <template #header>
            <div class="card-header">
              <span>InfluxDB配置</span>
            </div>
          </template>
          
          <el-form ref="influxdbFormRef" :model="config.influxdb" :rules="influxdbRules" label-width="120px">
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="URL" prop="url">
                  <el-input v-model="config.influxdb.url" placeholder="输入InfluxDB URL" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="Token" prop="token">
                  <el-input 
                    v-model="config.influxdb.token" 
                    type="password" 
                    placeholder="输入访问Token" 
                    show-password
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="组织" prop="org">
                  <el-input v-model="config.influxdb.org" placeholder="输入组织名称" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="Bucket" prop="bucket">
                  <el-input v-model="config.influxdb.bucket" placeholder="输入Bucket名称" />
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </el-card>
      </el-tab-pane>
      
      <!-- 交易设置 -->
      <el-tab-pane label="交易" name="trading">
        <el-card class="settings-card">
          <template #header>
            <div class="card-header">
              <span>交易配置</span>
            </div>
          </template>
          
          <el-form ref="tradingFormRef" :model="config.trading" :rules="tradingRules" label-width="150px">
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="模拟交易" prop="enable_paper_trading">
                  <el-switch v-model="config.trading.enable_paper_trading" />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="每秒最大订单数" prop="max_orders_per_second">
                  <el-input-number 
                    v-model="config.trading.max_orders_per_second" 
                    :min="1" 
                    :max="1000" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="默认手续费率" prop="default_commission_rate">
                  <el-input-number 
                    v-model="config.trading.default_commission_rate" 
                    :min="0" 
                    :max="0.1" 
                    :step="0.0001" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="默认滑点" prop="default_slippage">
                  <el-input-number 
                    v-model="config.trading.default_slippage" 
                    :min="0" 
                    :max="0.1" 
                    :step="0.0001" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="订单超时时间(秒)" prop="order_timeout_seconds">
                  <el-input-number 
                    v-model="config.trading.order_timeout_seconds" 
                    :min="1" 
                    :max="3600" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </el-card>
      </el-tab-pane>
      
      <!-- 风险设置 -->
      <el-tab-pane label="风险" name="risk">
        <el-card class="settings-card">
          <template #header>
            <div class="card-header">
              <span>风险配置</span>
            </div>
          </template>
          
          <el-form ref="riskFormRef" :model="config.risk" :rules="riskRules" label-width="150px">
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="最大持仓比例" prop="max_position_size">
                  <el-slider 
                    v-model="config.risk.max_position_size" 
                    :min="0" 
                    :max="1" 
                    :step="0.01" 
                    show-input
                    style="width: 100%"
                  />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="单日最大亏损比例" prop="max_daily_loss">
                  <el-slider 
                    v-model="config.risk.max_daily_loss" 
                    :min="0" 
                    :max="0.2" 
                    :step="0.001" 
                    show-input
                    style="width: 100%"
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="最大回撤限制" prop="max_drawdown">
                  <el-slider 
                    v-model="config.risk.max_drawdown" 
                    :min="0" 
                    :max="0.3" 
                    :step="0.01" 
                    show-input
                    style="width: 100%"
                  />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="VaR置信水平" prop="var_confidence_level">
                  <el-slider 
                    v-model="config.risk.var_confidence_level" 
                    :min="0.9" 
                    :max="0.999" 
                    :step="0.001" 
                    show-input
                    style="width: 100%"
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="启用事前检查" prop="enable_pre_trade_check">
                  <el-switch v-model="config.risk.enable_pre_trade_check" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="启用实时监控" prop="enable_real_time_monitor">
                  <el-switch v-model="config.risk.enable_real_time_monitor" />
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </el-card>
      </el-tab-pane>
      
      <!-- 监控设置 -->
      <el-tab-pane label="监控" name="monitoring">
        <el-card class="settings-card">
          <template #header>
            <div class="card-header">
              <span>监控配置</span>
            </div>
          </template>
          
          <el-form ref="monitoringFormRef" :model="config.monitoring" :rules="monitoringRules" label-width="150px">
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="启用Prometheus" prop="enable_prometheus">
                  <el-switch v-model="config.monitoring.enable_prometheus" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="Prometheus端口" prop="prometheus_port">
                  <el-input-number 
                    v-model="config.monitoring.prometheus_port" 
                    :min="1" 
                    :max="65535" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="日志级别" prop="log_level">
                  <el-select v-model="config.monitoring.log_level" placeholder="选择日志级别" style="width: 100%">
                    <el-option label="Debug" value="debug" />
                    <el-option label="Info" value="info" />
                    <el-option label="Warning" value="warning" />
                    <el-option label="Error" value="error" />
                  </el-select>
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="告警邮箱" prop="alert_email">
                  <el-input 
                    v-model="config.monitoring.alert_email" 
                    placeholder="输入告警邮箱地址（可选）" 
                  />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="告警Webhook" prop="alert_webhook">
                  <el-input 
                    v-model="config.monitoring.alert_webhook" 
                    placeholder="输入告警Webhook URL（可选）" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </el-card>
      </el-tab-pane>
      
      <!-- 安全设置 -->
      <el-tab-pane label="安全" name="security">
        <el-card class="settings-card">
          <template #header>
            <div class="card-header">
              <span>安全配置</span>
            </div>
          </template>
          
          <el-form ref="securityFormRef" :model="config.security" :rules="securityRules" label-width="150px">
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="启用加密" prop="enable_encryption">
                  <el-switch v-model="config.security.enable_encryption" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="启用双因素认证" prop="enable_2fa">
                  <el-switch v-model="config.security.enable_2fa" />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="JWT密钥" prop="jwt_secret">
                  <el-input 
                    v-model="config.security.jwt_secret" 
                    type="password" 
                    placeholder="输入JWT密钥" 
                    show-password
                  />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="Token过期时间(小时)" prop="token_expiry_hours">
                  <el-input-number 
                    v-model="config.security.token_expiry_hours" 
                    :min="1" 
                    :max="720" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="24">
                <el-form-item label="允许的IP地址" prop="allowed_ips">
                  <el-select
                    v-model="config.security.allowed_ips"
                    multiple
                    filterable
                    allow-create
                    default-first-option
                    placeholder="输入允许的IP地址"
                    style="width: 100%"
                  >
                  </el-select>
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </el-card>
      </el-tab-pane>
    </el-tabs>
    
    <!-- 操作按钮 -->
    <el-card class="action-card">
      <div class="action-buttons">
        <el-button type="primary" @click="saveConfig" :loading="saving">保存配置</el-button>
        <el-button @click="resetConfig">重置</el-button>
        <el-button @click="exportConfig">导出配置</el-button>
        <el-button @click="importConfig">导入配置</el-button>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage, type FormInstance, type FormRules } from 'element-plus';

// ========================
// TypeScript Interfaces
// ========================

interface SystemInfo {
  name: string;
  version: string;
  language: string;
  timezone: string;
}

interface DatabaseConfig {
  host: string;
  port: number;
  username: string;
  password: string | null;
  database: string;
  max_connections: number;
}

interface RedisConfig {
  host: string;
  port: number;
  password: string | null;
  db: number;
  pool_size: number;
}

interface InfluxDBConfig {
  url: string;
  token: string;
  org: string;
  bucket: string;
}

interface TradingConfig {
  enable_paper_trading: boolean;
  max_orders_per_second: number;
  default_commission_rate: number;
  default_slippage: number;
  order_timeout_seconds: number;
}

interface RiskConfig {
  max_position_size: number;
  max_daily_loss: number;
  max_drawdown: number;
  enable_pre_trade_check: boolean;
  enable_real_time_monitor: boolean;
  var_confidence_level: number;
}

interface MonitoringConfig {
  enable_prometheus: boolean;
  prometheus_port: number;
  log_level: string;
  alert_email: string | null;
  alert_webhook: string | null;
}

interface SecurityConfig {
  enable_encryption: boolean;
  jwt_secret: string;
  token_expiry_hours: number;
  enable_2fa: boolean;
  allowed_ips: string[];
}

interface SystemConfig {
  database: DatabaseConfig;
  redis: RedisConfig;
  influxdb: InfluxDBConfig;
  trading: TradingConfig;
  risk: RiskConfig;
  monitoring: MonitoringConfig;
  security: SecurityConfig;
}

// ========================
// Form Refs
// ========================

const systemInfoFormRef = ref<FormInstance>();
const databaseFormRef = ref<FormInstance>();
const redisFormRef = ref<FormInstance>();
const influxdbFormRef = ref<FormInstance>();
const tradingFormRef = ref<FormInstance>();
const riskFormRef = ref<FormInstance>();
const monitoringFormRef = ref<FormInstance>();
const securityFormRef = ref<FormInstance>();

// ========================
// Validation Rules
// ========================

const systemInfoRules: FormRules = {
  name: [
    { required: true, message: '请输入系统名称', trigger: 'blur' },
  ],
  version: [
    { required: true, message: '系统版本不能为空', trigger: 'blur' },
  ],
  language: [
    { required: true, message: '请选择语言', trigger: 'change' },
  ],
  timezone: [
    { required: true, message: '请选择时区', trigger: 'change' },
  ],
};

const databaseRules: FormRules = {
  host: [
    { required: true, message: '请输入数据库主机地址', trigger: 'blur' },
  ],
  port: [
    { required: true, message: '请输入数据库端口', trigger: 'blur' },
    { type: 'number', min: 1, max: 65535, message: '端口范围 1-65535', trigger: 'blur' },
  ],
  username: [
    { required: true, message: '请输入数据库用户名', trigger: 'blur' },
  ],
  password: [],
  database: [
    { required: true, message: '请输入数据库名', trigger: 'blur' },
  ],
  max_connections: [
    { required: true, message: '请输入最大连接数', trigger: 'blur' },
    { type: 'number', min: 1, max: 1000, message: '连接数范围 1-1000', trigger: 'blur' },
  ],
};

const redisRules: FormRules = {
  host: [
    { required: true, message: '请输入Redis主机地址', trigger: 'blur' },
  ],
  port: [
    { required: true, message: '请输入Redis端口', trigger: 'blur' },
    { type: 'number', min: 1, max: 65535, message: '端口范围 1-65535', trigger: 'blur' },
  ],
  password: [],
  db: [],
  pool_size: [
    { required: true, message: '请输入连接池大小', trigger: 'blur' },
    { type: 'number', min: 1, max: 100, message: '连接池大小范围 1-100', trigger: 'blur' },
  ],
};

const influxdbRules: FormRules = {
  url: [
    { required: true, message: '请输入InfluxDB URL', trigger: 'blur' },
  ],
  token: [],
  org: [],
  bucket: [],
};

const tradingRules: FormRules = {
  enable_paper_trading: [],
  max_orders_per_second: [
    { required: true, message: '请输入每秒最大订单数', trigger: 'blur' },
    { type: 'number', min: 1, max: 1000, message: '订单数范围 1-1000', trigger: 'blur' },
  ],
  default_commission_rate: [
    { required: true, message: '请输入默认手续费率', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.1, message: '手续费率范围 0-0.1', trigger: 'blur' },
  ],
  default_slippage: [
    { required: true, message: '请输入默认滑点', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.1, message: '滑点范围 0-0.1', trigger: 'blur' },
  ],
  order_timeout_seconds: [
    { required: true, message: '请输入订单超时时间', trigger: 'blur' },
    { type: 'number', min: 1, max: 3600, message: '超时范围 1-3600秒', trigger: 'blur' },
  ],
};

const riskRules: FormRules = {
  max_position_size: [
    { required: true, message: '请输入最大持仓比例', trigger: 'blur' },
    { type: 'number', min: 0, max: 1, message: '持仓比例范围 0-1', trigger: 'blur' },
  ],
  max_daily_loss: [
    { required: true, message: '请输入单日最大亏损比例', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.2, message: '亏损比例范围 0-0.2', trigger: 'blur' },
  ],
  max_drawdown: [
    { required: true, message: '请输入最大回撤限制', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.3, message: '回撤范围 0-0.3', trigger: 'blur' },
  ],
  var_confidence_level: [
    { required: true, message: '请输入VaR置信水平', trigger: 'blur' },
    { type: 'number', min: 0.9, max: 0.999, message: '置信水平范围 0.9-0.999', trigger: 'blur' },
  ],
  enable_pre_trade_check: [],
  enable_real_time_monitor: [],
};

const monitoringRules: FormRules = {
  enable_prometheus: [],
  prometheus_port: [
    { required: true, message: '请输入Prometheus端口', trigger: 'blur' },
    { type: 'number', min: 1, max: 65535, message: '端口范围 1-65535', trigger: 'blur' },
  ],
  log_level: [
    { required: true, message: '请选择日志级别', trigger: 'change' },
  ],
  alert_email: [],
  alert_webhook: [],
};

const securityRules: FormRules = {
  enable_encryption: [],
  enable_2fa: [],
  jwt_secret: [],
  token_expiry_hours: [
    { required: true, message: '请输入Token过期时间', trigger: 'blur' },
    { type: 'number', min: 1, max: 720, message: '过期时间范围 1-720小时', trigger: 'blur' },
  ],
  allowed_ips: [],
};

// ========================
// Reactive Data
// ========================

const activeTab = ref('basic');
const saving = ref(false);

const systemInfo = ref<SystemInfo>({
  name: '量化交易系统',
  version: '1.0.0',
  language: 'zh-CN',
  timezone: 'UTC+8',
});

const config = ref<SystemConfig>({
  database: {
    host: 'localhost',
    port: 5432,
    username: 'quant',
    password: 'quant_password',
    database: 'quant_trading',
    max_connections: 50,
  },
  redis: {
    host: 'localhost',
    port: 6379,
    password: null,
    db: 0,
    pool_size: 20,
  },
  influxdb: {
    url: 'http://localhost:8086',
    token: '',
    org: 'quant-trading',
    bucket: 'market-data',
  },
  trading: {
    enable_paper_trading: true,
    max_orders_per_second: 100,
    default_commission_rate: 0.0003,
    default_slippage: 0.0001,
    order_timeout_seconds: 30,
  },
  risk: {
    max_position_size: 0.2,
    max_daily_loss: 0.05,
    max_drawdown: 0.15,
    enable_pre_trade_check: true,
    enable_real_time_monitor: true,
    var_confidence_level: 0.95,
  },
  monitoring: {
    enable_prometheus: true,
    prometheus_port: 9090,
    log_level: 'info',
    alert_email: null,
    alert_webhook: null,
  },
  security: {
    enable_encryption: true,
    jwt_secret: 'change_this_secret_in_production',
    token_expiry_hours: 24,
    enable_2fa: false,
    allowed_ips: ['127.0.0.1'],
  },
});

// ========================
// Methods
// ========================

// Fetch current configuration
async function fetchConfig() {
  try {
    const data = await invoke<SystemConfig>('get_config');
    config.value = data;
  } catch (error) {
    console.error('Failed to fetch config:', error);
    ElMessage.error('获取配置失败');
  }
}

// Save configuration
async function saveConfig() {
  if (!systemInfoFormRef.value) return;

  const formRefs = [
    systemInfoFormRef,
    databaseFormRef,
    redisFormRef,
    influxdbFormRef,
    tradingFormRef,
    riskFormRef,
    monitoringFormRef,
    securityFormRef,
  ];

  // Validate all forms
  const validationResults = await Promise.all(
    formRefs.map((ref) => {
      if (!ref.value) return Promise.resolve(true);
      return ref.value.validate().then(() => true).catch(() => false);
    }),
  );

  const allValid = validationResults.every((valid) => valid);
  if (!allValid) {
    ElMessage.warning('请检查表单中的必填项');
    return;
  }

  saving.value = true;
  try {
    await invoke<boolean>('update_config', { config: config.value });
    ElMessage.success('配置保存成功');
  } catch (error) {
    console.error('Failed to save config:', error);
    ElMessage.error('保存配置失败: ' + (error as Error).message);
  } finally {
    saving.value = false;
  }
}

// Reset configuration
function resetConfig() {
  ElMessage.info('重置功能开发中...');
}

// Export configuration
function exportConfig() {
  ElMessage.info('导出功能开发中...');
}

// Import configuration
function importConfig() {
  ElMessage.info('导入功能开发中...');
}

// Initialize on mount
onMounted(() => {
  fetchConfig();
});
</script>

<style scoped>
.system-settings {
  padding: 20px;
}

.header {
  margin-bottom: 20px;
}

.settings-tabs {
  margin-bottom: 20px;
}

.settings-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.action-card {
  margin-top: 20px;
}

.action-buttons {
  display: flex;
  gap: 10px;
  justify-content: center;
}
</style>
