use chrono::offset::Utc;
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
    otp_setup_key: String,
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
            otp_setup_key: env::var("OTP_Setup_Key")
                .expect("to have the environment variable OTP_Setup_Key"),
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

fn otp_token(otp_setup_key: &str) -> String {
    let start_code =
        base32::decode(base32::Alphabet::RFC4648 { padding: false }, otp_setup_key).unwrap();
    let otp_code = oath::totp_raw_now(&start_code, 6, 0, 30, &oath::HashType::SHA1);
    otp_code.to_string()
}

#[given(regex = "An authenticated request to private url (.*)")]
fn private_api(world: &mut MyWorld, url: String) {
    let nonce: u64 = Utc::now().timestamp_millis() as u64;
    let request_url = format!("{}{}", API_DOMAIN, url);
    let otp_code = otp_token(&world.otp_setup_key);
    let post_data = [("nonce", &nonce.to_string()), ("otp", &otp_code)];
    let to_hash = format!(
        "{}{}",
        nonce,
        serde_urlencoded::to_string(post_data).expect("to encode post_data")
    );

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
        .form(&post_data)
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
            let response_data: answer_data::Answer<answer_data::TimeResult> =
                serde_json::from_slice(&resp_bytes).expect("to be able to parse response");
            response_data.check_valid();
            println!(
                "Server responded with time: {}",
                response_data.result.unwrap().rfc1123
            );
        }
        "ticker" => {
            //json response validation
            let response_data: answer_data::Answer<answer_data::TickerResult> =
                serde_json::from_slice(&resp_bytes).expect("to be able to parse response");
            response_data.check_valid();
            response_data.result.unwrap().print_price();
        }
        "orders" => {
            let response_data: answer_data::Answer<answer_data::OrdersResult> =
                serde_json::from_slice(&resp_bytes).expect("to be able to parse response");
            response_data.check_valid();
            let result = response_data.result.unwrap(); //can't fail since check_valid would return failure
            let order_names: Vec<&String> = result.open.as_object().unwrap().keys().collect();
            println!("Got {} open orders: {:?}", order_names.len(), order_names);
            println!("Orders_json {}", result.open.to_string());
        }
        _ => panic!("unrecognized check type"),
    }
}

#[tokio::main]
async fn main() {
    let runner = MyWorld::init(&["./features"]);
    // debug needed to output information
    let results = runner.debug(true).cli().run().await;
    // Print results of the test run
    if results.failed() {
        println!("Test failed!");
        std::process::exit(1);
    } else {
        println!("Ran {} scenarios successfully!", results.scenarios.passed);
        std::process::exit(0);
    }
}
