use select::document::Document;
use select::predicate::Name;

pub async fn scrape_site() {
    // let res =  reqwest::get("https://rust-lang.org/").await?.text().await?;
    let res = match reqwest::get("https://rust-lang.org/").await {
        Ok(body) => match body.text().await {
            Ok(text) => text,
            Err(err) => return,
        },
        Err(err) => return,
    };

    Document::from(res.as_str())
        .find(Name("a"))
        .filter_map(|n| n.attr("href"))
        .for_each(|x| println!("{}", x));
}
