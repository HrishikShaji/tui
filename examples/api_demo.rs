use std::io::Read;

fn main() -> anyhow::Result<()> {
    let mut res = reqwest::blocking::get("https://httpbin.org/get")?;

    let mut body = String::new();
    res.read_to_string(&mut body)?;

    println!("Status :{}", res.status());
    println!("Headers :\n {:#?}", res.headers());
    println!("Body :\n {}", body);

    Ok(())
}
