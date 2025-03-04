use rquest::{redirect::Policy, Impersonate};

#[tokio::main]
async fn main() -> Result<(), rquest::Error> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("trace"));

    // Build a client to impersonate Safari18
    let mut client = rquest::Client::builder()
        .impersonate(Impersonate::Safari18)
        .build()?;

    // Set the redirect policy
    client.as_mut().redirect(Policy::default());

    // Use the API you're already familiar with
    let text = client
        .get("http://google.com/")
        .send()
        .await?
        .text()
        .await?;

    println!("{}", text);

    Ok(())
}
