// Test implementation

use reqwest;
// use serde_json::Value;
// use futures::Future;
// use reqwest::Response;
pub async fn handle_web_requests() -> Result<(), Box<dyn std::error::Error>> {
	return Ok(());
	let client = reqwest::Client::new();
	// let web_future = client.get("https://markhughes.com/getmyip.php").send().fuse();

	// client has timeout and connect_timeout set to 5
	// return client.get("https://markhughes.com/getmyip.php").send();

    let api_key = "your_api_key";
    let url = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest";
    let res = client.get("https://markhughes.com/getmyip.php")
        .header("X-CMC_PRO_API_KEY", api_key)
        .query(&[("symbol", "EMAID"), ("convert", "USD")])
        .send()
        .await?;

    let body = res.text().await?;
	// panic!("body: {}", body);
	// let parsed: Value = serde_json::from_str(&body)?;

   Ok(())
}

