extern crate flate2;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs;
use std::fs::DirBuilder;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::copy;
use std::time::Instant;

pub fn decompress_file(args: Vec<&str>) {
    if args.len() < 2 {
        println!("arg1 : {}  arg2: {}", args[0], args[1]);
        return;
    }

    let fname = std::path::Path::new(&args[1]);

    println!("File name is {:?}", fname);

    let file = fs::File::open(&fname).unwrap();

    println!("File is {:?}", file);

    let mut archive = zip::ZipArchive::new(&file).unwrap();

    println!("Archived File is {:?}", archive);

    for i in 0..archive.len() {
        println!("Index is {}", i);
        let mut file = archive.by_index(i).unwrap();
        println!("Indexed File is {:?}", file);

        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        println!("Outpath is {:?}", outpath);

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("The File with comment is {} and comment is {}", i, comment);
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
                    fs::create_dir_all(&p).unwrap();
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

pub fn compress_file(args: Vec<&str>) {
    if args.len() < 3 {
        eprintln!("Usage: `source` `targer`");
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

    println!("Target Len:{:?}", output.metadata().unwrap().len());

    println!("Elapsed Time:{:?}", start.elapsed());
}

pub fn create_file(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: create <file>");
        return;
    }

    match fs::write(args[1], "") {
        Ok(_) => println!("File created"),
        Err(e) => println!("Error: {}", e),
    }
}

pub fn write_file(args: Vec<&str>) {
    if args.len() < 3 {
        println!("Usage: write <file> <content>");
        return;
    }

    match fs::write(args[1], args[2]) {
        Ok(_) => println!("Written successfully"),
        Err(e) => println!("Error: {}", e),
    }
}

pub fn read_file(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: read <file>");
        return;
    }

    match fs::read_to_string(args[1]) {
        Ok(content) => println!("{}", content),
        Err(e) => println!("Error: {}", e),
    }
}

pub fn delete_file(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: delete <file>");
        return;
    }

    match fs::remove_file(args[1]) {
        Ok(_) => println!("Deleted"),
        Err(e) => println!("Error: {}", e),
    }
}

pub fn get_file_type(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: file_type <file>");
        return;
    }

    match fs::metadata(args[1]) {
        Ok(metadata) => {
            println!("{:?}", metadata.file_type());
        }
        Err(err) => println!("Error figuring out the file type : {}", err),
    };
}

pub fn create_directory(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: create_directory <folder path>");
        return;
    };
    match DirBuilder::new().recursive(true).create(args[1]) {
        Ok(path) => {
            println!("{:?}", path);
        }
        Err(err) => println!("Error creating folder  : {}", err),
    };
}

pub fn read_entries(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: create_directory <folder path>");
        return;
    };

    let entries = match fs::read_dir(args[1]) {
        Ok(dir) => dir,
        Err(e) => {
            println!("Error reading dir : {}", e);
            return;
        }
    };

    for entry in entries {
        match entry {
            Ok(file) => {
                println!("file is :{:?}", file.path());
            }
            Err(e) => {
                println!("Error reading file:{}", e);
            }
        }
    }
}
