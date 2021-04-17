use cucumber_rust::{async_trait, given, then, when, World, WorldInit};
use reqwest::{Client, RequestBuilder, Response};
use serde_json;
use std::convert::Infallible;

mod answer_data;
const API_DOMAIN: &str = "https://api.kraken.com";

pub trait Validatable {
    fn check_valid(&self);
}

#[derive(WorldInit)]
pub struct MyWorld {
    request_builder: Option<RequestBuilder>,
    response: Option<Response>,
    api_public_key: String,
    api_private_key: String,
    two_factor_pwd: String,
}

#[async_trait(?Send)]
impl World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        use std::env;
        Ok(Self {
            request_builder: None,
            response: None,
            api_public_key: env::var("API_Public_Key")
                .expect("to have the environment variable API_Public_Key"),
            api_private_key: env::var("API_Private_Key")
                .expect("to have the environment variable API_Private_Key"),
            two_factor_pwd: env::var("OTP").expect("to have the environment variable OTP"),
        })
    }
}

#[given(regex = "A request to public url (.*)")]
fn public_api(world: &mut MyWorld, url: String) {
    let request_url = format!("{}{}", API_DOMAIN, url);
    let req_builder = Client::new()
        .get(request_url)
        .header("User-Agent", "Kraken REST API");
    world.request_builder = Some(req_builder);
}

#[given(regex = "An authenticated request to private url (.*)")]
fn private_api(world: &mut MyWorld, url: String) {
    let nonce: u64 = chrono::offset::Utc::now().timestamp_millis() as u64;
    //let nonce: u64 = 1618690640656;
    let request_url = format!("{}{}", API_DOMAIN, url);
    let post_data = format!("&nonce={}&otp={}", nonce, world.two_factor_pwd);
    let to_hash = format!("{}{}", nonce, post_data);

    use sha2::{Digest, Sha256, Sha512};
    let sha256_digest = Sha256::digest(to_hash.as_bytes());

    use hmac::{Hmac, Mac, NewMac};
    type HmacSha512 = Hmac<Sha512>;
    let api_secret = base64::decode(world.api_private_key.as_str()).expect("to decode private key");
    let mut mac = HmacSha512::new_varkey(&api_secret).expect("to be able to create hmac");
    mac.update(&url.as_bytes());
    mac.update(&sha256_digest);
    let hmac_sha512 = mac.finalize();

    let req_builder = Client::new()
        .post(request_url)
        .body(post_data)
        .header("API-Key", world.api_public_key.clone())
        .header("API-Sign", base64::encode(hmac_sha512.into_bytes()))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", "Kraken REST API");
    world.request_builder.replace(req_builder);
}

#[when("I send it")]
async fn i_request(world: &mut MyWorld) {
    let req = world
        .request_builder
        .take()
        .expect("to have a request already built");
    let res = req.send().await;
    if !res.is_ok() {
        println!("{:?}", res);
        panic!("Server responded with error")
    }
    world.response = res.ok();
}

#[then(regex = "The server responds with status (.*)")]
fn server_responds(world: &mut MyWorld, status: String) {
    match status.to_lowercase().as_str() {
        "ok" => {
            let status = world.response.as_ref().map(|r| r.status().is_success());
            if status != Some(true) {
                println!("{:?}", world.response);
            }
            assert_eq!(status, Some(true));
        }
        _ => panic!("not implemented"),
    }
}

#[then(regex = "The response has the correct (time|ticker|orders) format")]
async fn response_time_format(world: &mut MyWorld, check_type: String) {
    let response = world.response.take().expect("to have a response");
    let resp_bytes = response
        .bytes()
        .await
        .expect("to have been able to read response");
    match check_type.to_lowercase().as_str() {
        "time" => {
            //json response validation
            let resp_json: answer_data::Answer<answer_data::TimeResult> =
                serde_json::from_slice(&resp_bytes).expect("to be able to parse response");
            println!("Server responded with time: {}", resp_json.result.rfc1123);
            resp_json.result.check_valid();
        }
        "ticker" => {
            //json response validation
            let resp_json: answer_data::Answer<answer_data::TickerResult> =
                serde_json::from_slice(&resp_bytes).expect("to be able to parse response");
            resp_json.result.check_valid();
            resp_json.result.print_price();
        }
        "orders" => {
            let resp_json: answer_data::Answer<answer_data::OrdersResult> =
                serde_json::from_slice(&resp_bytes).expect("to be able to parse response");
            resp_json.result.check_valid();
            let order_names: Vec<&String> =
                resp_json.result.open.as_object().unwrap().keys().collect();
            println!("Got {} open orders: {:?}", order_names.len(), order_names);
        }
        _ => panic!("unrecognized check type"),
    }
}

#[tokio::main]
async fn main() {
    let runner = MyWorld::init(&["./features"]);
    runner.debug(true).run_and_exit().await;
}
