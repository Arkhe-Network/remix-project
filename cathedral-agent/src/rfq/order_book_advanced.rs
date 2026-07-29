// cathedral-agent/src/rfq/order_book_advanced.rs
use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;
use chrono::{Utc, DateTime, Duration};
use crate::rfq::pricing_engine::OrderSide;

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExecutionPolicy {
    FOK, // Fill or Kill
    IOC, // Immediate or Cancel
    GTC, // Good Til Cancelled
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub asset_ref: String,
    pub side: OrderSide,
    pub amount: u64,
    pub filled: u64,
    pub price: f64,
    pub policy: ExecutionPolicy,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub peer_did: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Expired,
    Settled,
}

/// Order Book com matching engine Price-Time priority.
pub struct OrderBookAdvanced {
    /// Ordens de compra (preço descendente)
    pub buy_orders: Mutex<Vec<Order>>,
    /// Ordens de venda (preço ascendente)
    pub sell_orders: Mutex<Vec<Order>>,
    /// Histórico de execuções
    history: Mutex<Vec<Execution>>,
}

impl OrderBookAdvanced {
    pub fn new() -> Self {
        Self {
            buy_orders: Mutex::new(Vec::new()),
            sell_orders: Mutex::new(Vec::new()),
            history: Mutex::new(Vec::new()),
        }
    }

    /// Adiciona uma ordem
    pub async fn add_order(&self, order: Order) {
        let mut orders = match order.side {
            OrderSide::Buy => self.buy_orders.lock().await,
            OrderSide::Sell => self.sell_orders.lock().await,
        };
        orders.push(order.clone());
        self.sort_orders(&mut orders, &order.side);
    }

    /// Ordena ordens (Price-Time priority)
    fn sort_orders(&self, orders: &mut Vec<Order>, side: &OrderSide) {
        match side {
            OrderSide::Buy => {
                // Maior preço primeiro, depois mais antigo
                orders.sort_by(|a, b| {
                    b.price.partial_cmp(&a.price)
                        .unwrap()
                        .then(a.created_at.cmp(&b.created_at))
                });
            }
            OrderSide::Sell => {
                // Menor preço primeiro, depois mais antigo
                orders.sort_by(|a, b| {
                    a.price.partial_cmp(&b.price)
                        .unwrap()
                        .then(a.created_at.cmp(&b.created_at))
                });
            }
        }
    }

    /// Executa matching entre ordens de compra e venda
    pub async fn match_orders(&self, asset_ref: &str) -> Vec<Execution> {
        let mut buy_orders = self.buy_orders.lock().await;
        let mut sell_orders = self.sell_orders.lock().await;
        let mut executions = Vec::new();

        // Filtra por ativo e status pending
        let mut buys: Vec<Order> = buy_orders
            .iter()
            .filter(|o| o.asset_ref == asset_ref && o.status == OrderStatus::Pending)
            .cloned()
            .collect();

        let mut sells: Vec<Order> = sell_orders
            .iter()
            .filter(|o| o.asset_ref == asset_ref && o.status == OrderStatus::Pending)
            .cloned()
            .collect();

        // Ordena
        buys.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        sells.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

        let mut buy_idx = 0;
        let mut sell_idx = 0;

        while buy_idx < buys.len() && sell_idx < sells.len() {
            let buy = &mut buys[buy_idx];
            let sell = &mut sells[sell_idx];

            // Verifica se o preço de compra é >= preço de venda
            if buy.price < sell.price {
                break;  // não há match possível
            }

            // Quantidade a ser executada
            let buy_remaining = buy.amount - buy.filled;
            let sell_remaining = sell.amount - sell.filled;
            let exec_amount = buy_remaining.min(sell_remaining);

            // Preço de execução (média ponderada ou preço do sell)
            let exec_price = sell.price;

            // Cria execução
            let execution = Execution {
                id: format!("exec_{}", uuid::Uuid::new_v4()),
                asset_ref: asset_ref.to_string(),
                buy_order_id: buy.id.clone(),
                sell_order_id: sell.id.clone(),
                amount: exec_amount,
                price: exec_price,
                timestamp: Utc::now(),
                status: ExecutionStatus::Pending,
            };

            // Atualiza ordens
            buy.filled += exec_amount;
            sell.filled += exec_amount;

            // Atualiza status
            if buy.filled == buy.amount {
                buy.status = OrderStatus::Filled;
            } else {
                buy.status = OrderStatus::PartiallyFilled;
            }

            if sell.filled == sell.amount {
                sell.status = OrderStatus::Filled;
            } else {
                sell.status = OrderStatus::PartiallyFilled;
            }

            executions.push(execution);

            // Avança índices
            if buy.filled == buy.amount {
                buy_idx += 1;
            }
            if sell.filled == sell.amount {
                sell_idx += 1;
            }
        }

        // Atualiza as listas principais
        *buy_orders = buys;
        *sell_orders = sells;

        // Registra histórico
        let mut history = self.history.lock().await;
        history.extend(executions.clone());

        executions
    }

    /// Obtém o melhor preço de compra
    pub async fn best_bid(&self, asset_ref: &str) -> Option<f64> {
        let orders = self.buy_orders.lock().await;
        orders
            .iter()
            .filter(|o| o.asset_ref == asset_ref && o.status == OrderStatus::Pending)
            .map(|o| o.price)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Obtém o melhor preço de venda
    pub async fn best_ask(&self, asset_ref: &str) -> Option<f64> {
        let orders = self.sell_orders.lock().await;
        orders
            .iter()
            .filter(|o| o.asset_ref == asset_ref && o.status == OrderStatus::Pending)
            .map(|o| o.price)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Obtém o spread atual
    pub async fn spread(&self, asset_ref: &str) -> Option<f64> {
        let bid = self.best_bid(asset_ref).await?;
        let ask = self.best_ask(asset_ref).await?;
        Some(ask - bid)
    }
}

/// Execução de ordem
#[derive(Clone, Debug)]
pub struct Execution {
    pub id: String,
    pub asset_ref: String,
    pub buy_order_id: String,
    pub sell_order_id: String,
    pub amount: u64,
    pub price: f64,
    pub timestamp: DateTime<Utc>,
    pub status: ExecutionStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Confirmed,
    Failed,
}
