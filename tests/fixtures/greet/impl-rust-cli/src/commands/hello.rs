use super::Output;
use super::args::HelloArgs;
use anyhow::Result;

pub fn run(args: &HelloArgs) -> Result<Output> {
    let mut greeting = format!("Hello, {}!", args.who);
    if args.shout {
        greeting = greeting.to_uppercase();
    }
    Ok(Output::new(greeting.clone(), serde_json::json!({ "greeting": greeting })))
}
