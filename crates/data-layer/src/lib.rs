pub mod binance_source;
pub mod data_quality;
pub mod live_trades_repo;
pub mod market_data;
pub mod market_data_repo;
pub mod postgres;
pub mod repository;

pub use binance_source::BinanceDataSource;
pub use live_trades_repo::{LiveTrade, LiveTradesRepository};
pub use market_data_repo::{
    AccountSnapshotRecord, BalanceRecord, FundingRateRecord, LastPriceRecord, MarkPriceRecord,
    MarketDataRecord, MarketDataRepository, NewAccountSnapshot, NewBalance, NewLastPrice,
    NewMarketDataRecord, NewOrderbookSnapshot, NewPositionSnapshot, NewStreamTrade,
    NewTickerSnapshot, OrderbookSnapshotRecord, PositionSnapshotRecord, StreamTradeRecord,
    TickerSnapshotRecord,
};
pub use postgres::PostgresClient;
pub use repository::{
    AlertRepository, ApiKeyRecord, ApiKeyRepository, AuditFilter, AuditLogRecord,
    AuditRepository, BacktestRepository, BacktestResultSummaryRow, BacktestResultsPage,
    NewApiKey, NewAuditLog, PgAlertRepository, PgApiKeyRepository, PgAuditRepository,
    PgBacktestRepository, PgStrategyRepository, RepoError, StrategyRepository, StrategyStats,
    StrategySummaryRow,
};
