use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

pub struct WebPrices {
    pub snt_rate: Option<f64>,    // Currency value per SNT (e.g. 0.20)
    pub btc_rate: Option<f64>,    // Currency value per BTC

    pub currency_apiname:    String,   // For API query (e.g. "USD")
    pub currency_symbol:    String,    // For UI (e.g. "$")

    pub last_update_time:   Option<DateTime<Utc>>,
}

impl WebPrices {
    pub fn new() -> WebPrices {
        WebPrices {
            snt_rate: None,
            btc_rate: None,

            currency_apiname:    String::from(""),
            currency_symbol:    String::from(""),

            last_update_time: None,
        }
    }
}

const DEFAULT_COINGECKO_POLL_INTERVAL: i64 = 30;        // Minutes (based on free account)
const DEFAULT_COINMARKETCAP_POLL_INTERVAL: i64 = 30;    // Minutes (based on free account)
const DEFAULT_SWITCH_API_POLL_INTERVAL: i64 = 5;        // Minutes to wait after switching API

pub struct WebPriceAPIs {
    currency_apiname:    String,    // For API query (e.g. "USD")

    current_api_key:    Option<String>,
    switching_api_interval: Duration,

    // CoinGecko
    coingecko_api_key: Option<String>,
    coingecko_next_poll: Option<DateTime<Utc>>,
    coingecko_min_poll_interval: Duration,

    // CoinMarketCap Configuration
    coinmarketcap_api_key: Option<String>,
    coinmarketcap_next_poll: Option<DateTime<Utc>>,
    coinmarketcap_min_poll_interval: Duration,
}

pub const CMC_API_SAFE_TOKEN_NAME: &str = "EMAID";          // Coinmarketcap API

// For vdash UI:
pub const SAFE_TOKEN_TICKER: &str = "SNT";
pub const BTC_TICKER: &str = "BTC";

impl WebPriceAPIs {
    pub fn new(coingecko_api_key: Option<String>, coinmarketcap_api_key: Option<String>, currency_apiname: &String) -> WebPriceAPIs {
        WebPriceAPIs {
            currency_apiname: currency_apiname.clone(),

            current_api_key: None,
            switching_api_interval: Duration::seconds(DEFAULT_SWITCH_API_POLL_INTERVAL),

            coingecko_api_key: coingecko_api_key,
            coingecko_next_poll: None,
            coingecko_min_poll_interval: Duration::minutes(DEFAULT_COINGECKO_POLL_INTERVAL),

            coinmarketcap_api_key: coinmarketcap_api_key,
            coinmarketcap_next_poll: None,
            coinmarketcap_min_poll_interval: Duration::minutes(DEFAULT_COINMARKETCAP_POLL_INTERVAL),
        }
    }

    /// Call one of up to two web apis to get prices. Uses a minimum poll interval to
    /// avoid excessive use of the metered APIs and avoid slowing down other threads.
    ///
    /// If the default API fails to return a value, switches to using the alternate API
    /// for the next cycle (setting a shorter interval for the retry).
    ///
    /// /// Returns the currency_per_token rate if successful
    pub async fn handle_web_requests(&mut self) -> Result<Option<f64>, Box<dyn std::error::Error>> {
        let now = Utc::now();

        let mut currency_token_rate = None;
        if self.coingecko_api_key.is_some() {

            if self.current_api_key.is_none() || self.current_api_key.as_ref().unwrap() == self.coingecko_api_key.as_ref().unwrap() {
                if self.coingecko_next_poll.is_none() || self.coingecko_next_poll.unwrap() < now {
                    self.coingecko_next_poll = Some(now + self.coingecko_min_poll_interval);
                    currency_token_rate = self.get_coingecko_prices().await?;

                    if currency_token_rate.is_some() {
                        self.current_api_key = Some(self.coingecko_api_key.as_ref().unwrap().clone());
                    } else if self.coinmarketcap_api_key.is_some() {
                        self.coinmarketcap_next_poll = Some(now + self.switching_api_interval);
                        self.current_api_key = Some(self.coinmarketcap_api_key.as_ref().unwrap().clone());
                    }
                }
            }
        }

        if self.coinmarketcap_api_key.is_some() {

            if self.current_api_key.is_none() || self.current_api_key.as_ref().unwrap() == self.coinmarketcap_api_key.as_ref().unwrap() {
                if self.coinmarketcap_next_poll.is_none() || self.coinmarketcap_next_poll.unwrap() < now {
                    self.coinmarketcap_next_poll = Some(now + self.coinmarketcap_min_poll_interval);
                    currency_token_rate = self.get_coinmarketcap_prices().await?;

                    if currency_token_rate.is_some() {
                        self.current_api_key = Some(self.coinmarketcap_api_key.as_ref().unwrap().clone());
                    } else if self.coingecko_api_key.is_some() {
                        self.coingecko_next_poll = Some(now + self.switching_api_interval);
                        self.current_api_key = Some(self.coingecko_api_key.as_ref().unwrap().clone());
                    }
                }
            }
        }

        Ok(currency_token_rate)
    }

    // Access price via API, lock the WebPrices object and store the new values
    // Returns the currency_per_token rate if successful
    pub async fn get_coingecko_prices(&mut self) -> Result<Option<f64>, Box<dyn std::error::Error>> {
        if let Some(api_key) = &self.coingecko_api_key {
            let client = reqwest::Client::new();
            let url = "https://api.coingecko.com/api/v3/simple/price";
            let response = client.get(url)
                .header("x-cg-demo-api-key", api_key)
                .query(&[("ids", "maidsafecoin,bitcoin"), ("vs_currencies", &format!("{}", self.currency_apiname).to_lowercase())])
                .send()
                .await?;

            let body = response.text().await?;
            let json = serde_json::from_str::<Value>(&body)?;
            let mut prices = super::app::WEB_PRICES.lock()?;
            let time_now = Some(Utc::now());
            if let Some(btcprices) = json["bitcoin"].as_object() {
                let currency_key = &self.currency_apiname.as_str().to_lowercase();
                if !btcprices.contains_key(currency_key) {
                    let message = format!("unrecognised API value for --currency-apiname option: {}", &self.currency_apiname.as_str());
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, message.as_str())));
                }

                prices.btc_rate = btcprices[self.currency_apiname.to_lowercase().as_str()].as_f64();
            }
            if let Some(token_prices) = json["maidsafecoin"].as_object() {
                prices.snt_rate = token_prices[self.currency_apiname.to_lowercase().as_str()].as_f64();
                prices.last_update_time = time_now;
                return Ok(prices.snt_rate);
            }
        }


        Ok(None)
    }

    // Access price via API, lock the WebPrices object and store the new values
    // Returns the currency_per_token rate if successful
    pub async fn get_coinmarketcap_prices(&mut self) -> Result<Option<f64>, Box<dyn std::error::Error>> {
        let mut currency_per_token = None;
        let mut error = None;

        if let Some(api_key) = &self.coinmarketcap_api_key {
            let response: reqwest::Response = reqwest::Client::builder()
            // .pool_idle_timeout(None)
                .build()?
                .get("https://pro-api.coinmarketcap.com/v2/cryptocurrency/quotes/latest")
                .header("X-CMC_PRO_API_KEY", api_key)
                .header("Accept", "application/json")
                .query(&[("symbol", CMC_API_SAFE_TOKEN_NAME), ("convert", self.currency_apiname.as_str())])
                .send()
                .await?;

            let body = response.text().await?;
            let json = serde_json::from_str::<Value>(&body)?;

            let _ = json["data"].as_object().is_some_and(|data| {
                data["EMAID"].as_array().is_some_and(|emaid| {
                    emaid[0].as_object().is_some_and(|emaid_0| {
                        emaid_0["quote"].as_object().is_some_and(|quote| {
                            let currency_key = &self.currency_apiname.as_str().to_uppercase();
                            if !quote.contains_key(currency_key) {
                                let message = format!("unrecognised API value for --currency-apiname option: {}", &self.currency_apiname.as_str());
                                error = Some(std::io::Error::new(std::io::ErrorKind::Other, message.as_str()));
                                return false;
                            }
                            quote[currency_key].as_object().is_some_and(|usd| {
                                usd["price"].as_f64().is_some_and(|token_price|{
                                    let mut prices = super::app::WEB_PRICES.lock().unwrap();
                                    prices.snt_rate = Some(token_price);
                                    prices.last_update_time = Some(Utc::now());
                                    currency_per_token = Some(token_price);
                                    true
                                })
                            })
                        })
                    })
                })
            });
        }

        if error.is_some() {
            return Err(Box::new(error.unwrap()));
        }

        Ok(currency_per_token)
    }

}
