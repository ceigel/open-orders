use cucumber_rust::{async_trait, given, then, when, World, WorldInit};
use reqwest::{Client, Response};
use serde_json;
use std::convert::Infallible;

const API_DOMAIN: &str = "https://api.kraken.com";

pub trait Validatable {
    fn check_valid(&self);
}

#[derive(WorldInit)]
pub struct MyWorld {
    request_url: String,
    response: Option<Response>,
}

#[async_trait(?Send)]
impl World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            request_url: "".into(),
            response: None,
        })
    }
}

#[given(regex = "The api url (.*)")]
fn the_api(world: &mut MyWorld, url: String) {
    world.request_url = format!("{}{}", API_DOMAIN, url);
}

#[when("I do a GET request to it")]
async fn i_request(world: &mut MyWorld) {
    let res = Client::new()
        .get(world.request_url.as_str())
        .header("User-Agent", "Kraken REST API")
        .send()
        .await;
    assert!(res.is_ok());
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

mod api_answer {
    use crate::Validatable;
    use chrono::DateTime;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize, Debug)]
    pub struct TimeResult {
        pub unixtime: i64,
        pub rfc1123: String,
    }

    impl Validatable for TimeResult {
        fn check_valid(&self) {
            // rfc2822 is a newer format of rfc1233, thus they should be compatible
            let time_rfc2822 = DateTime::parse_from_rfc2822(&self.rfc1123)
                .expect("to be able to parse rfc1233 time");
            // Expect that unixtime is the same time as the rfc1233 field
            assert_eq!(time_rfc2822.timestamp(), self.unixtime);
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct TickerResultData {
        #[serde(rename(deserialize = "a"))]
        ask: [String; 3],

        #[serde(rename(deserialize = "b"))]
        bid: [String; 3],

        #[serde(rename(deserialize = "c"))]
        closed: [String; 2],

        #[serde(rename(deserialize = "v"))]
        volume: [String; 2],

        #[serde(rename(deserialize = "p"))]
        weighted_average_volume: [String; 2],

        #[serde(rename(deserialize = "t"))]
        number_of_trades: [u64; 2],

        #[serde(rename(deserialize = "l"))]
        low: [String; 2],
        #[serde(rename(deserialize = "h"))]
        high: [String; 2],
        #[serde(rename(deserialize = "o"))]
        day_opening_price: String,
    }

    // Check if the array is parsable as float (decimal would be better, buf float is also ok here)
    fn as_float_array(arr: &[String]) -> Vec<f64> {
        use std::str::FromStr;
        let vals: Result<Vec<f64>, std::num::ParseFloatError> =
            arr.iter().map(|val| f64::from_str(val)).collect();
        vals.expect("to be able to parse all values")
    }

    impl Validatable for TickerResultData {
        fn check_valid(&self) {
            assert_ne!(self.number_of_trades[0], 0);
            assert_ne!(self.number_of_trades[1], 0);
            assert!(self.number_of_trades[0] < self.number_of_trades[1]);
            let asks = as_float_array(self.ask.as_ref());
            assert!(asks.iter().all(|&v| v > 0.0));

            let bids = as_float_array(self.bid.as_ref());
            assert!(bids.iter().all(|&v| v > 0.0));

            let closed = as_float_array(self.closed.as_ref());
            //maybe this fails at beginning of the day
            assert!(closed.iter().all(|&v| v > 0.0));

            let volume = as_float_array(self.volume.as_ref());
            // since we only test with XBT, the volume for last 24 hours can't be null
            // at beginning of the day this can be null
            assert!(volume[1..].iter().all(|&v| v > 0.0));

            let wav = as_float_array(self.weighted_average_volume.as_ref());
            // since we only test with XBT, the volume for last 24 hours can't be null
            // at beginning of the day this can be null
            assert!(wav[1..].iter().all(|&v| v > 0.0));

            let low = as_float_array(self.low.as_ref());
            assert!(low.iter().all(|&v| v > 0.0));

            let high = as_float_array(self.high.as_ref());
            assert!(high.iter().all(|&v| v > 0.0));

            let open = as_float_array(&[self.day_opening_price.clone()][..]);
            assert!(open.iter().all(|&v| v > 0.0));
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct TickerResult(HashMap<String, TickerResultData>);
    impl TickerResult {
        pub fn print_price(&self) {
            println!("XBT/USD last price: {}", self.0["XXBTZUSD"].closed[0]);
        }
    }

    impl Validatable for TickerResult {
        fn check_valid(&self) {
            let ticker_names: Vec<&str> = self.0.keys().map(|s| s.as_str()).collect();
            assert_eq!(ticker_names, vec!["XXBTZUSD"]);
            self.0[ticker_names[0]].check_valid();
        }
    }
    #[derive(Deserialize, Debug)]
    pub struct Answer<T> {
        pub error: Vec<String>,
        pub result: T,
    }
    impl<T: Validatable> Validatable for Answer<T> {
        fn check_valid(&self) {
            assert_eq!(self.error.len(), 0);
            self.result.check_valid();
        }
    }
}
#[then(regex = "The response has the correct (time|ticker) format")]
async fn response_time_format(world: &mut MyWorld, check_type: String) {
    let response = world.response.take().expect("to have a response");
    let resp_bytes = response
        .bytes()
        .await
        .expect("to have been able to read response");
    match check_type.to_lowercase().as_str() {
        "time" => {
            //json response validation
            let resp_json: api_answer::Answer<api_answer::TimeResult> =
                serde_json::from_slice(&resp_bytes).expect("to be able to parse response");
            println!("Server responded with time: {}", resp_json.result.rfc1123);
            resp_json.result.check_valid();
        }
        "ticker" => {
            //json response validation
            let resp_json: api_answer::Answer<api_answer::TickerResult> =
                serde_json::from_slice(&resp_bytes).expect("to be able to parse response");
            resp_json.result.check_valid();
            resp_json.result.print_price();
        }
        _ => panic!("unrecognized check type"),
    }
}

#[tokio::main]
async fn main() {
    let runner = MyWorld::init(&["./features/public_api"]);
    runner.debug(true).run_and_exit().await;
}
