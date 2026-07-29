// cathedral-agent/src/rfq/pricing_engine.rs
use std::collections::HashMap;
use tokio::sync::RwLock;
use chrono::{Utc, DateTime};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Estratégia de precificação para RFQ.
pub trait PricingStrategy: Send + Sync {
    fn calculate_price(
        &self,
        asset_ref: &str,
        amount: u64,
        side: OrderSide,
        context: &PricingContext,
    ) -> f64;
}

/// Contexto de precificação
pub struct PricingContext {
    pub market_price: f64,
    pub liquidity: u64,
    pub volatility: f64,
    pub timestamp: DateTime<Utc>,
    pub peer_reputation: f64,  // 0.0 - 1.0
}

/// Estratégia baseada em mercado com spread dinâmico
pub struct MarketSpreadPricing {
    spread_base: f64,          // spread base (ex: 0.01 = 1%)
    spread_volatility_factor: f64,  // fator de volatilidade
    min_spread: f64,
    max_spread: f64,
}

impl MarketSpreadPricing {
    pub fn new(spread_base: f64, volatility_factor: f64, min_spread: f64, max_spread: f64) -> Self {
        Self {
            spread_base,
            spread_volatility_factor: volatility_factor,
            min_spread,
            max_spread,
        }
    }

    fn calculate_spread(&self, volatility: f64) -> f64 {
        let spread = self.spread_base + (volatility * self.spread_volatility_factor);
        spread.clamp(self.min_spread, self.max_spread)
    }
}

impl PricingStrategy for MarketSpreadPricing {
    fn calculate_price(
        &self,
        _asset_ref: &str,
        amount: u64,
        side: OrderSide,
        context: &PricingContext,
    ) -> f64 {
        let spread = self.calculate_spread(context.volatility);
        let base_price = context.market_price;

        // Ajuste por quantidade (ordens maiores pagam mais spread)
        let liquidity_ratio = amount as f64 / (context.liquidity as f64 + 1.0);
        let size_adj = (liquidity_ratio * 0.5).min(0.05);

        let total_spread = spread + size_adj;

        match side {
            OrderSide::Buy => base_price * (1.0 + total_spread),
            OrderSide::Sell => base_price * (1.0 - total_spread),
        }
    }
}

/// Estratégia baseada em AMM (Automated Market Maker)
pub struct AmmPricing {
    reserve_asset: RwLock<HashMap<String, u64>>,  // reserva em asset
    reserve_btc: RwLock<HashMap<String, u64>>,    // reserva em BTC (sats)
    fee: f64,  // taxa AMM (0.003 = 0.3%)
}

impl AmmPricing {
    pub fn new(fee: f64) -> Self {
        Self {
            reserve_asset: RwLock::new(HashMap::new()),
            reserve_btc: RwLock::new(HashMap::new()),
            fee,
        }
    }

    pub async fn update_reserves(
        &self,
        asset_ref: &str,
        asset_amount: u64,
        btc_amount: u64,
    ) {
        let mut reserves = self.reserve_asset.write().await;
        reserves.insert(asset_ref.to_string(), asset_amount);
        let mut btc_reserves = self.reserve_btc.write().await;
        btc_reserves.insert(asset_ref.to_string(), btc_amount);
    }
}

impl PricingStrategy for AmmPricing {
    fn calculate_price(
        &self,
        asset_ref: &str,
        _amount: u64,
        side: OrderSide,
        _context: &PricingContext,
    ) -> f64 {
        let asset_reserve = self.reserve_asset.blocking_read().get(asset_ref).cloned().unwrap_or(1_000_000);
        let btc_reserve = self.reserve_btc.blocking_read().get(asset_ref).cloned().unwrap_or(100_000_000);

        // Fórmula AMM: price = (reserve_btc / reserve_asset) * (1 + fee)
        let price = btc_reserve as f64 / asset_reserve as f64;

        match side {
            OrderSide::Buy => price * (1.0 + self.fee),   // comprador paga fee
            OrderSide::Sell => price * (1.0 - self.fee), // vendedor recebe menos
        }
    }
}
