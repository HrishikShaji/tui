use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::copy;
use std::time::Instant;

pub fn compress_file(args: Vec<&str>) {
    if args.len() < 3 {
        eprintln!("Usage: compress_file <source> <target>");
        return;
    }

    let mut input = BufReader::new(File::open(args[1]).unwrap());

    let output = File::create(args[2]).unwrap();

    let mut encoder = GzEncoder::new(output, Compression::default());

    let start = Instant::now();

    copy(&mut input, &mut encoder).unwrap();

    let output = encoder.finish().unwrap();

    println!(
        "Source len: {:?}",
        input.get_ref().metadata().unwrap().len()
    );

    println!("Target len: {:?}", output.metadata().unwrap().len());

    println!("Elapsed time: {:?}", start.elapsed());
}

pub fn decompress_file(args: Vec<&str>) {
    if args.len() < 2 {
        eprintln!("Usage: decompress_file <source>");
        return;
    }

    let fname = std::path::Path::new(&args[1]);

    let file = fs::File::open(fname).unwrap();

    let mut archive = zip::ZipArchive::new(&file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();

        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }

        if (*file.name()).ends_with("/") {
            println!("File {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );

            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }

            let mut outfile = fs::File::create(&outpath).unwrap();

            io::copy(&mut file, &mut outfile).unwrap();
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
}
