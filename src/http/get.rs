pub async fn get(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Invalid Usage : get <url>");
        return;
    }

    match reqwest::get(args[1]).await {
        Ok(res) => match res.text().await {
            Ok(body) => println!("This is body {}", body),
            Err(e) => println!("Error parsing body:{}", e),
        },
        Err(e) => println!("Error fetching endpoint:{}", e),
    }
}
