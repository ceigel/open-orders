use chrono::DateTime;
use cucumber_rust::{async_trait, given, then, when, World, WorldInit};
use reqwest::{Client, Response};
use std::convert::Infallible;

const API_DOMAIN: &str = "https://api.kraken.com";

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

mod time_answer {
    use serde::Deserialize;
    #[derive(Deserialize, Debug)]
    pub struct TimeResult {
        pub unixtime: i64,
        pub rfc1123: String,
    }

    #[derive(Deserialize, Debug)]
    pub struct Answer {
        pub error: Vec<String>,
        pub result: TimeResult,
    }
}
#[then("The response has the correct time format")]
async fn response_time_format(world: &mut MyWorld) {
    let response = world.response.take().expect("to have a response");
    let resp_json = response
        .json::<time_answer::Answer>()
        .await
        .expect("to have response with correct format");
    assert_eq!(resp_json.error.len(), 0);
    // rfc2822 is a newer format of rfc1233, thus they should be compatible
    let time_rfc2822 = DateTime::parse_from_rfc2822(&resp_json.result.rfc1123)
        .expect("to be able to parse rfc1233 time");
    // Expect that unixtime is the same time as the rfc1233 field
    assert_eq!(time_rfc2822.timestamp(), resp_json.result.unixtime);
    println!("Server responded with time: {}", resp_json.result.rfc1123);
}

#[tokio::main]
async fn main() {
    let runner = MyWorld::init(&["./features/public_api"]);
    runner.debug(true).run_and_exit().await;
}
