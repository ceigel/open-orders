use std::{cell::RefCell, convert::Infallible};

use cucumber_rust::{async_trait, given, then, when, World, WorldInit};

#[derive(WorldInit)]
pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String,
    some_value: RefCell<u8>,
}

impl MyWorld {
    async fn test_async_fn(&mut self) {
        *self.some_value.borrow_mut() = 123u8;
    }
}

#[async_trait(?Send)]
impl World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            foo: "wat".into(),
            some_value: RefCell::new(0),
        })
    }
}

#[given("I connect to server")]
fn i_am_trying_out(world: &mut MyWorld) {
    world.foo = "Some string".to_string();
}

#[when("I query time")]
fn i_consider(world: &mut MyWorld) {
    let new_string = format!("{}.", &world.foo);
    world.foo = new_string;
}

#[then("The server responds with time")]
fn i_am_interested(world: &mut MyWorld) {
    assert_eq!(world.foo, "Some string.");
}

#[then("The response has the correct format")]
fn we_can_regex(world: &mut MyWorld) {
    assert_eq!(world.foo, "Some string.");
}

#[tokio::main]
async fn main() {
    let runner = MyWorld::init(&["./features/public_api.feature"]);
    runner.run_and_exit().await;
}