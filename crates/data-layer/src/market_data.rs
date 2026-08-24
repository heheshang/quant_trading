use chrono::{DateTime, Utc};
use quant_common::types::MarketData;
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use tracing::instrument;

/// 数据源接口
#[async_trait::async_trait]
pub trait DataSource: Send + Sync {
    /// 获取实时行情
    async fn get_realtime_data(&self, symbol: &str) -> Result<MarketData>;

    /// 获取历史数据
    async fn get_historical_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>>;

    /// 拉取某标的/周期最近 `limit` 根 K 线（用于 DB 冷启动 REST 回填）。
    ///
    /// 区别于 [`DataSource::get_historical_data`]（按时间窗 + 固定周期），
    /// 该方法按调用方指定的周期返回最新 `limit` 根，供 `get_klines` 在库内
    /// 无数据时回填 `market_data`。
    async fn get_klines_history(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: i64,
    ) -> Result<Vec<MarketData>>;

    /// 订阅实时行情
    async fn subscribe(&self, symbols: Vec<String>) -> Result<()>;

    /// 取消订阅
    async fn unsubscribe(&self, symbols: Vec<String>) -> Result<()>;
}

/// 市场数据管理器
pub struct MarketDataManager {
    sources: Vec<Box<dyn DataSource>>,
}

impl MarketDataManager {
    /// 创建新的市场数据管理器
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// 添加数据源
    pub fn add_source(&mut self, source: Box<dyn DataSource>) {
        self.sources.push(source);
    }

    /// 获取实时行情（从第一个可用的数据源）
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_realtime_data(&self, symbol: &str) -> Result<MarketData> {
        for source in &self.sources {
            match source.get_realtime_data(symbol).await {
                Ok(data) => return Ok(data),
                Err(_) => continue,
            }
        }
        Err(Error::NotFound(format!(
            "No data found for symbol: {}",
            symbol
        )))
    }

    /// 获取历史数据
    #[instrument(skip(self), fields(symbol = %symbol, %start, %end))]
    pub async fn get_historical_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>> {
        for source in &self.sources {
            match source.get_historical_data(symbol, start, end).await {
                Ok(data) => return Ok(data),
                Err(_) => continue,
            }
        }
        Err(Error::NotFound(format!(
            "No historical data found for symbol: {}",
            symbol
        )))
    }
}

impl Default for MarketDataManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 示例数据源（模拟数据）
pub struct MockDataSource;

#[async_trait::async_trait]
impl DataSource for MockDataSource {
    async fn get_realtime_data(&self, symbol: &str) -> Result<MarketData> {
        Ok(MarketData {
            symbol: symbol.to_string(),
            timestamp: Utc::now(),
            open: Decimal::new(10000, 2),
            high: Decimal::new(10100, 2),
            low: Decimal::new(9900, 2),
            close: Decimal::new(10050, 2),
            volume: Decimal::new(1000000, 0),
            turnover: Decimal::new(100500000, 2),
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        })
    }

    async fn get_historical_data(
        &self,
        _symbol: &str,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>> {
        Ok(Vec::new())
    }

    async fn get_klines_history(
        &self,
        _symbol: &str,
        _timeframe: &str,
        _limit: i64,
    ) -> Result<Vec<MarketData>> {
        Ok(Vec::new())
    }

    async fn subscribe(&self, _symbols: Vec<String>) -> Result<()> {
        Ok(())
    }

    async fn unsubscribe(&self, _symbols: Vec<String>) -> Result<()> {
        Ok(())
    }
}
