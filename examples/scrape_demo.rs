use select::document::Document;
use select::predicate::Name;

#[tokio::main]
async fn main() {
    let res = match reqwest::get("https://rust-lang.org/").await {
        Ok(body) => match body.text().await {
            Ok(text) => text,
            Err(_err) => return,
        },
        Err(_err) => return,
    };

    Document::from(res.as_str())
        .find(Name("a"))
        .filter_map(|n| n.attr("href"))
        .for_each(|x| println!("{}", x));
}
