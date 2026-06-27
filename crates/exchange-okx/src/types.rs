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

/// K线数据 (OKX API 原始格式, 使用数字索引)
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

/// K线数据 (前端展示格式)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CandleView {
    pub ts: String,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub vol: f64,
}

impl From<OkxCandle> for CandleView {
    fn from(c: OkxCandle) -> Self {
        CandleView {
            ts: c.ts,
            o: c.open.parse().unwrap_or(0.0),
            h: c.high.parse().unwrap_or(0.0),
            l: c.low.parse().unwrap_or(0.0),
            c: c.close.parse().unwrap_or(0.0),
            vol: c.vol.parse().unwrap_or(0.0),
        }
    }
}

/// 交易对信息 (OKX API 原始格式)
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxInstrument {
    pub inst_id: String,
    pub inst_type: String,
    pub uly: String,
    pub base_ccy: String,
    pub quote_ccy: String,
    pub ct_val: String,
    pub tick_sz: String,
    pub lot_sz: String,
    pub min_sz: String,
}

/// 交易对信息 (前端展示格式)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentView {
    pub inst_id: String,
    pub inst_type: String,
    pub uly: String,
    pub base_ccy: String,
    pub quote_ccy: String,
    pub ct_val: f64,
    pub tick_sz: String,
    pub lot_sz: f64,
    pub min_sz: f64,
}

impl From<OkxInstrument> for InstrumentView {
    fn from(i: OkxInstrument) -> Self {
        InstrumentView {
            inst_id: i.inst_id,
            inst_type: i.inst_type,
            uly: i.uly,
            base_ccy: i.base_ccy,
            quote_ccy: i.quote_ccy,
            ct_val: i.ct_val.parse().unwrap_or(0.0),
            tick_sz: i.tick_sz,
            lot_sz: i.lot_sz.parse().unwrap_or(0.0),
            min_sz: i.min_sz.parse().unwrap_or(0.0),
        }
    }
}

/// 余额信息 (前端展示格式)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BalanceView {
    pub ccy: String,
    pub eq: f64,
    pub cash_bal: f64,
    pub avail_eq: f64,
    pub frozen_bal: f64,
}

impl From<OkxBalance> for BalanceView {
    fn from(b: OkxBalance) -> Self {
        BalanceView {
            ccy: b.ccy,
            eq: b.eq.parse().unwrap_or(0.0),
            cash_bal: b.cash_bal.parse().unwrap_or(0.0),
            avail_eq: b.avail_eq.parse().unwrap_or(0.0),
            frozen_bal: b.frozen_bal.parse().unwrap_or(0.0),
        }
    }
}

/// 持仓信息 (前端展示格式)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PositionView {
    pub inst_id: String,
    pub pos: f64,
    pub avail_pos: f64,
    pub avg_px: f64,
    pub upl: f64,
    pub upl_ratio: f64,
}

impl From<OkxPosition> for PositionView {
    fn from(p: OkxPosition) -> Self {
        PositionView {
            inst_id: p.inst_id,
            pos: p.pos.parse().unwrap_or(0.0),
            avail_pos: p.avail_pos.parse().unwrap_or(0.0),
            avg_px: p.avg_px.parse().unwrap_or(0.0),
            upl: p.upl.parse().unwrap_or(0.0),
            upl_ratio: p.upl_ratio.parse().unwrap_or(0.0),
        }
    }
}

/// 订单信息 (前端展示格式)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderView {
    pub ord_id: String,
    pub cl_ord_id: String,
    pub inst_id: String,
    pub side: String,
    pub ord_type: String,
    pub px: f64,
    pub sz: f64,
    pub state: String,
    pub avg_px: f64,
    pub acc_fill_sz: f64,
    pub u_time: String,
}

impl From<OkxOrder> for OrderView {
    fn from(o: OkxOrder) -> Self {
        OrderView {
            ord_id: o.ord_id,
            cl_ord_id: o.cl_ord_id,
            inst_id: o.inst_id,
            side: o.side,
            ord_type: o.ord_type,
            px: o.px.parse().unwrap_or(0.0),
            sz: o.sz.parse().unwrap_or(0.0),
            state: o.state,
            avg_px: o.avg_px.parse().unwrap_or(0.0),
            acc_fill_sz: o.acc_fill_sz.parse().unwrap_or(0.0),
            u_time: o.u_time,
        }
    }
}

/// 下单请求
#[derive(Debug, Serialize, Deserialize, Clone)]
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

// ── Market Data Types ──

/// Ticker 24h 统计
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxTicker {
    pub inst_id: String,
    pub last: String,
    pub last_sz: String,
    pub ask_px: String,
    pub bid_px: String,
    pub open_24h: String,
    pub high_24h: String,
    pub low_24h: String,
    pub vol_ccy_24h: String,
    pub vol_24h: String,
    pub sod_utc0: String,
    pub sod_utc8: String,
    pub ts: String,
}

/// 资金费率
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxFundingRate {
    pub inst_id: String,
    pub funding_rate: String,
    pub next_funding_rate: String,
    pub funding_time: String,
    pub inst_type: String,
}

/// 标记价格
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxMarkPrice {
    pub inst_id: String,
    pub mark_px: String,
    pub ts: String,
}

/// 指数价格
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxIndexPrice {
    pub inst_id: String,
    pub idx_px: String,
    pub ts: String,
}

/// 持仓量
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxOpenInterest {
    pub inst_id: String,
    pub oi: String,
    pub oi_ccy: String,
    pub ts: String,
}

/// 成交明细
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OkxTrade {
    pub inst_id: String,
    pub trade_id: String,
    pub px: String,
    pub sz: String,
    pub side: String,
    pub ts: String,
}

/// 订单薄
#[derive(Debug, Serialize, Clone)]
pub struct OkxOrderBook {
    pub asks: Vec<Vec<String>>,
    pub bids: Vec<Vec<String>>,
    pub ts: String,
}

impl<'de> Deserialize<'de> for OkxOrderBook {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct DepthHelper {
            asks: Vec<Vec<String>>,
            bids: Vec<Vec<String>>,
            ts: String,
        }

        let helper = DepthHelper::deserialize(deserializer)?;
        Ok(OkxOrderBook {
            asks: helper.asks,
            bids: helper.bids,
            ts: helper.ts,
        })
    }
}
