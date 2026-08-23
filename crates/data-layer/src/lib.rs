pub mod binance_source;
pub mod data_quality;
pub mod market_data;
pub mod market_data_repo;
pub mod postgres;

pub use binance_source::BinanceDataSource;
pub use market_data_repo::{
    AccountSnapshotRecord, FundingRateRecord, MarkPriceRecord, MarketDataRecord,
    MarketDataRepository, NewAccountSnapshot, NewFundingRate, NewMarkPrice, NewMarketDataRecord,
    NewPositionSnapshot, NewTickerSnapshot, PositionSnapshotRecord, TickerSnapshotRecord,
};
pub use postgres::PostgresClient;
