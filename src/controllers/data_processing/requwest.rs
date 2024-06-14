use reqwest::{
header::{self, HeaderMap, HeaderValue},
Client,
};
use serde_json::json;

// ... (other code)

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
dotenv().ok();
let arguments = Args::parse();
let url = std::env::var("OPENAI_API_BASE").expect("url not set");
let open_ai_api_key = env::var("OPENAI_API_KEY").expect("token not set");
let query = arguments.query.to_owned();
let client = Client::new();

let headers: HeaderMap<HeaderValue> = header::HeaderMap::from_iter(vec![
(header::CONTENT_TYPE, "application/json".parse().unwrap()),
(
header::AUTHORIZATION,
format!("Bearer {}", open_ai_api_key).parse().unwrap(),
),
]);

let body = json!(
{
"model":"GigaChat",
"messages":[{
"role":"user",
"content": query,
}]
}
);

let response: ApiResponse = client
.post(url)
.headers(headers)
.json(&body)
.send()
.await?
.json()
.await?;

println!("{}", &response.choices[0].message.content);

Ok(())
}
