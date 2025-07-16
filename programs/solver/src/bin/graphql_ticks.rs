use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct Tick {
    tickIdx: String,
    liquidityGross: String,
    liquidityNet: String,
}

#[derive(Debug, Deserialize)]
struct Data {
    ticks: Vec<Tick>,
}

#[derive(Debug, Deserialize)]
struct GraphResponse {
    data: Data,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example variables
    let pool_address = "0x477e1a178f308fb8c2967d3e56e157c4b8b6f5df";
    let skip = 0;

    // Construct the GraphQL query string
    let query = format!(
        r#"{{
            ticks(
                where: {{poolAddress: "{}", liquidityNet_not: "0"}}
                first: 1000,
                skip: {},
            ) {{
                id
                liquidityGross
                liquidityNet
            }}
        }}"#,
        pool_address.to_lowercase(),
        skip
    );

    // Prepare the request body
    // JSON body
    let body = json!({
        "query": query,
        "operationName": "Subgraphs",
        "variables": {}
    });

    let api_key = "";

    // Send the POST request
    let client = Client::new();
    let resp = client
        .post("https://gateway.thegraph.com/api/subgraphs/id/5zvR82QoaXYFyDEKLZ9t6v9adgnptxYpKpSbxtgVENFV")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await?;

    // Print raw JSON response
    let text = resp.text().await?;
    println!("Response:\n{}", text);

    Ok(())
}
