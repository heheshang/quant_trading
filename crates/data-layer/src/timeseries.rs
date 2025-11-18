use chrono::{DateTime, Utc};
use influxdb::{Client, InfluxDbWriteable, ReadQuery};
use quant_common::config::InfluxDBConfig;
use quant_common::types::MarketData;
use quant_common::{Error, Result};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

/// 时序数据库客户端（InfluxDB）
pub struct TimeSeriesDB {
    client: Client,
}

#[derive(InfluxDbWriteable, Serialize, Deserialize)]
struct MarketDataPoint {
    time: DateTime<Utc>,
    #[influxdb(tag)]
    symbol: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    turnover: f64,
}

impl TimeSeriesDB {
    /// 创建新的时序数据库客户端
    pub fn new(config: &InfluxDBConfig) -> Result<Self> {
        let client = Client::new(&config.url, &config.database).with_token(&config.token);

        Ok(Self { client })
    }

    /// 写入市场数据
    pub async fn write_market_data(&self, data: &MarketData) -> Result<()> {
        let point = MarketDataPoint {
            time: data.timestamp,
            symbol: data.symbol.clone(),
            open: data.open.to_string().parse().unwrap_or(0.0),
            high: data.high.to_string().parse().unwrap_or(0.0),
            low: data.low.to_string().parse().unwrap_or(0.0),
            close: data.close.to_string().parse().unwrap_or(0.0),
            volume: data.volume.to_string().parse().unwrap_or(0.0),
            turnover: data.turnover.to_string().parse().unwrap_or(0.0),
        };

        self.client
            .query(point.into_query("market_data"))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// 批量写入市场数据
    pub async fn write_market_data_batch(&self, data: &[MarketData]) -> Result<()> {
        for market_data in data {
            self.write_market_data(market_data).await?;
        }
        Ok(())
    }

    /// 查询市场数据
    pub async fn query_market_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>> {
        let query = ReadQuery::new(format!(
            "SELECT * FROM market_data WHERE symbol = '{}' AND time >= '{}' AND time <= '{}'",
            symbol,
            start.to_rfc3339(),
            end.to_rfc3339()
        ));

        let result = self
            .client
            .query(query)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        println!("{:?}", result);
        // 解析结果并转换为 MarketData
        // 这里简化处理，实际需要根据 InfluxDB 返回格式解析
        let parsed_market_data = 
        self.parse_market_data_result(json!(result), symbol)?;
        Ok(parsed_market_data)
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool> {
        let query = ReadQuery::new("SHOW TABLES".to_string());
        let result = self.client.query(query).await;

        Ok(result.is_ok())
    }

    /// 解析 InfluxDB 查询结果并转换为 MarketData 向量
    fn parse_market_data_result(&self, query_result: JsonValue, symbol_param: &str) -> Result<Vec<MarketData>> {
        let mut market_data_vec = Vec::new();
        
        // 解析 JSON 结构
        if let Some(results) = query_result.get("results").and_then(|r| r.as_array()) {
            for result in results {
                if let Some(series) = result.get("series").and_then(|s| s.as_array()) {
                    for serie in series {
                        // 获取列信息
                        let columns = match serie.get("columns").and_then(|c| c.as_array()) {
                            Some(cols) => cols,
                            None => continue,
                        };
                        
                        // 创建列名到索引的映射
                        let mut column_indices = std::collections::HashMap::new();
                        for (i, column) in columns.iter().enumerate() {
                            if let Some(column_name) = column.as_str() {
                                column_indices.insert(column_name, i);
                            }
                        }
                        
                        // 获取数据值
                        if let Some(values) = serie.get("values").and_then(|v| v.as_array()) {
                            for value_row in values {
                                if let Some(row) = value_row.as_array() {
                                    let time = if let Some(&time_idx) = column_indices.get("time") {
                                        if let Some(time_val) = row.get(time_idx).and_then(|v| v.as_str()) {
                                            match DateTime::parse_from_rfc3339(time_val) {
                                                Ok(dt) => dt.with_timezone(&Utc),
                                                Err(_) => continue,
                                            }
                                        } else {
                                            continue;
                                        }
                                    } else {
                                        continue;
                                    };
                                    
                                    let open = if let Some(&open_idx) = column_indices.get("open") {
                                        if let Some(open_val) = row.get(open_idx).and_then(|v| v.as_f64()) {
                                            Decimal::from_f64_retain(open_val).unwrap_or(Decimal::ZERO)
                                        } else {
                                            Decimal::ZERO
                                        }
                                    } else {
                                        Decimal::ZERO
                                    };
                                    
                                    let high = if let Some(&high_idx) = column_indices.get("high") {
                                        if let Some(high_val) = row.get(high_idx).and_then(|v| v.as_f64()) {
                                            Decimal::from_f64_retain(high_val).unwrap_or(Decimal::ZERO)
                                        } else {
                                            Decimal::ZERO
                                        }
                                    } else {
                                        Decimal::ZERO
                                    };
                                    
                                    let low = if let Some(&low_idx) = column_indices.get("low") {
                                        if let Some(low_val) = row.get(low_idx).and_then(|v| v.as_f64()) {
                                            Decimal::from_f64_retain(low_val).unwrap_or(Decimal::ZERO)
                                        } else {
                                            Decimal::ZERO
                                        }
                                    } else {
                                        Decimal::ZERO
                                    };
                                    
                                    let close = if let Some(&close_idx) = column_indices.get("close") {
                                        if let Some(close_val) = row.get(close_idx).and_then(|v| v.as_f64()) {
                                            Decimal::from_f64_retain(close_val).unwrap_or(Decimal::ZERO)
                                        } else {
                                            Decimal::ZERO
                                        }
                                    } else {
                                        Decimal::ZERO
                                    };
                                    
                                    let volume = if let Some(&volume_idx) = column_indices.get("volume") {
                                        if let Some(volume_val) = row.get(volume_idx).and_then(|v| v.as_f64()) {
                                            Decimal::from_f64_retain(volume_val).unwrap_or(Decimal::ZERO)
                                        } else {
                                            Decimal::ZERO
                                        }
                                    } else {
                                        Decimal::ZERO
                                    };
                                    
                                    let turnover = if let Some(&turnover_idx) = column_indices.get("turnover") {
                                        if let Some(turnover_val) = row.get(turnover_idx).and_then(|v| v.as_f64()) {
                                            Decimal::from_f64_retain(turnover_val).unwrap_or(Decimal::ZERO)
                                        } else {
                                            Decimal::ZERO
                                        }
                                    } else {
                                        Decimal::ZERO
                                    };
                                    
                                    let symbol = if let Some(&symbol_idx) = column_indices.get("symbol") {
                                        if let Some(symbol_val) = row.get(symbol_idx).and_then(|v| v.as_str()) {
                                            symbol_val.to_string()
                                        } else {
                                            symbol_param.to_string()
                                        }
                                    } else {
                                        symbol_param.to_string()
                                    };
                                    
                                    let market_data = MarketData {
                                        symbol,
                                        timestamp: time,
                                        open,
                                        high,
                                        low,
                                        close,
                                        volume,
                                        turnover,
                                        open_interest: None,
                                        bid_prices: vec![],
                                        bid_volumes: vec![],
                                        ask_prices: vec![],
                                        ask_volumes: vec![],
                                    };
                                    
                                    market_data_vec.push(market_data);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(market_data_vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    #[tokio::test]
    async fn test_timeseries_db() {
        let ss = dotenv().ok();
        println!("{:?}", ss);
        let config = InfluxDBConfig {
            url: dotenv::var("INFLUXDB_URL").unwrap(),
            token: dotenv::var("INFLUXDB_TOKEN").unwrap(),
            database: dotenv::var("INFLUXDB_DATABASE").unwrap(),
        };
        println!("{:?}", config);

        // This test requires a running InfluxDB instance
        // Uncomment when InfluxDB is available
        let db = TimeSeriesDB::new(&config).unwrap();
        let health = db.health_check().await.unwrap();
        assert!(health);
    }
    #[tokio::test]
    async fn test_write_market_data() {
        let config = InfluxDBConfig {
            url: dotenv::var("INFLUXDB_URL").unwrap(),
            token: dotenv::var("INFLUXDB_TOKEN").unwrap(),
            database: dotenv::var("INFLUXDB_DATABASE").unwrap(),
        };
        let db = TimeSeriesDB::new(&config).unwrap();
        let market_data = MarketData {
            timestamp: Utc::now(),
            symbol: "TEST".to_string(),
            open: dec!(100.0),
            high: dec!(101.0),
            low: dec!(99.0),
            close: dec!(100.0),
            volume: dec!(1000.0),
            turnover: dec!(100000.0),
            ask_prices: vec![dec!(101.0), dec!(102.0)],
            ask_volumes: vec![dec!(10.0), dec!(20.0)],
            bid_prices: vec![dec!(99.0), dec!(98.0)],
            bid_volumes: vec![dec!(10.0), dec!(20.0)],
            open_interest: Some(Decimal::ZERO),
        };
        db.write_market_data(&market_data).await.unwrap();
    }
    #[tokio::test]
    async fn test_query_market_data() {
         let config = InfluxDBConfig {
            url: dotenv::var("INFLUXDB_URL").unwrap(),
            token: dotenv::var("INFLUXDB_TOKEN").unwrap(),
            database: dotenv::var("INFLUXDB_DATABASE").unwrap(),
        };
        let db = TimeSeriesDB::new(&config).unwrap();
        let market_data = db.query_market_data("TEST", Utc::now() - chrono::Duration::hours(2), Utc::now()).await.unwrap();
        println!("{:?}", market_data);
    }
}
