use error_chain::error_chain;
use reqwest::Client;
use reqwest::header::USER_AGENT;
use serde::Deserialize;

error_chain! {
    foreign_links{
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

pub async fn fetch() -> Result<()> {
    let res = reqwest::get("https://httpbin.org/get").await?;
    println!("Async Status :{}", res.status());
    println!("Async Headers :\n {:#?}", res.headers());
    let body = res.text().await?;
    println!("Async Body :\n {}", body);

    Ok(())
}

#[derive(Deserialize, Debug)]
struct User {
    login: String,
    id: u32,
}

pub async fn fetch_github_stargazers() -> Result<()> {
    let request_url = format!(
        "https://api.github.com/repos/{owner}/{repo}/stargazers",
        owner = "rust-lang-nursery",
        repo = "rust-cookbook"
    );

    println!("{}", request_url);

    let client = reqwest::Client::new();
    let response = client
        .get(&request_url)
        .header(USER_AGENT, "rust-web-api-client-demo")
        .send()
        .await?;

    let users: Vec<User> = response.json().await?;
    println!("Users : {:?}", users);
    Ok(())
}

pub async fn auth_check() -> Result<()> {
    let client = Client::new();

    let user = "testuser".to_string();
    let passwd: Option<String> = None;

    let response = client
        .get("http://httpbin.org/get")
        .basic_auth(user, passwd)
        .send()
        .await?;

    println!("{:?}", response);

    Ok(())
}
