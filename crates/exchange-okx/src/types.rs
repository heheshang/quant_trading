use serde::{Deserialize, Serialize};

/// OKX 环境配置
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OkxEnvironment {
    Live, // 实盘
    Demo, // 模拟盘
}

impl OkxEnvironment {
    pub fn base_url(&self) -> &str {
        match self {
            OkxEnvironment::Live => "https://www.okx.com",
            OkxEnvironment::Demo => "https://www.okx.com", // 模拟盘使用相同地址，通过header区分
        }
    }

    pub fn ws_public_url(&self) -> &str {
        match self {
            OkxEnvironment::Live => "wss://ws.okx.com:8443/ws/v5/public",
            OkxEnvironment::Demo => "wss://wspap.okx.com:8443/ws/v5/public?brokerId=9999",
        }
    }

    pub fn ws_private_url(&self) -> &str {
        match self {
            OkxEnvironment::Live => "wss://ws.okx.com:8443/ws/v5/private",
            OkxEnvironment::Demo => "wss://wspap.okx.com:8443/ws/v5/private?brokerId=9999",
        }
    }
}

/// OKX 响应结构
#[derive(Debug, Deserialize, Serialize)]
pub struct OkxResponse<T> {
    pub code: String,
    pub msg: String,
    pub data: Option<Vec<T>>,
}

/// 账户余额
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxBalance {
    pub ccy: String,
    pub eq: String,         // 余额
    pub cash_bal: String,   // 现金余额
    pub avail_eq: String,   // 可用余额
    pub frozen_bal: String, // 冻结余额
}

/// 持仓信息
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxPosition {
    pub inst_id: String,
    pub pos: String,
    pub avail_pos: String,
    pub avg_px: String,
    pub upl: String, // 未实现盈亏
    pub upl_ratio: String,
}

/// 订单信息
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxOrder {
    pub ord_id: String,
    pub cl_ord_id: String,
    pub inst_id: String,
    pub side: String,
    pub ord_type: String,
    pub px: String,
    pub sz: String,
    pub state: String,
    pub avg_px: String,
    pub acc_fill_sz: String,
    pub u_time: String,
}

/// K线数据
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OkxCandle {
    #[serde(rename = "0")]
    pub ts: String, // 时间戳
    #[serde(rename = "1")]
    pub open: String,
    #[serde(rename = "2")]
    pub high: String,
    #[serde(rename = "3")]
    pub low: String,
    #[serde(rename = "4")]
    pub close: String,
    #[serde(rename = "5")]
    pub vol: String, // 成交量
    #[serde(rename = "6")]
    pub vol_ccy: String, // 成交额
}

/// 下单请求
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxPlaceOrderRequest {
    pub inst_id: String,
    pub td_mode: String,  // 交易模式：cash, cross, isolated
    pub side: String,     // buy, sell
    pub ord_type: String, // market, limit, post_only
    pub sz: String,       // 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub px: Option<String>, // 价格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<String>, // 自定义订单ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>, // 订单标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos_side: Option<String>, // 持仓方向
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ccy: Option<String>, // 保证金币种
    #[serde(skip_serializing_if = "Option::is_none")]
    pub px_usd: Option<String>, // USD价格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub px_vol: Option<String>, // 隐含波动率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>, // 是否只减仓
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tgt_ccy: Option<String>, // 市价单委托数量单位
}

/// WebSocket 订阅参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkxWsSubscription {
    pub channel: String,
    #[serde(rename = "instId")]
    pub inst_id: String,
}

/// WebSocket 消息
#[derive(Debug, Deserialize, Serialize)]
pub struct OkxWsMessage {
    pub event: Option<String>,
    pub arg: Option<OkxWsSubscription>,
    pub data: Option<serde_json::Value>,
    pub code: Option<String>,
    pub msg: Option<String>,
}
