use std::fs::File;
use std::io::copy;
use tempfile::Builder;

pub async fn download_image() {
    let temp_dir = match Builder::new().prefix("example").tempdir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to create temp dir: {}", e);
            return;
        }
    };

    let target = "https://www.rust-lang.org/logos/rust-logo-512x512.png";

    let response = match reqwest::get(target).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Request failed: {}", e);
            return;
        }
    };

    let mut dest = {
        let fname = response
            .url()
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .unwrap_or("tmp.bin");

        println!("File name to download: {}", fname);

        let fname = temp_dir.path().join(fname);

        println!("Will be located under: {:?}", fname);

        match File::create(fname) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to create file: {}", e);
                return;
            }
        }
    };

    let content = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Failed to read response body: {}", e);
            return;
        }
    };

    if let Err(e) = copy(&mut content.as_ref(), &mut dest) {
        eprintln!("Failed to write file: {}", e);
    }
}
