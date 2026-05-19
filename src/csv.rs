use csv;
use std::error::Error;

pub fn read_csv_file() {
    if let Err(e) = read_from_file("./agreement.csv") {
        eprintln!("{}", e);
    }
}

fn read_from_file(path: &str) -> Result<(), Box<dyn Error>> {
    let mut reader = csv::Reader::from_path(path)?;

    for result in reader.records() {
        let record = result?;

        println!("{:?}", record);
    }

    Ok(())
}
