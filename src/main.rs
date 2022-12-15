use std::{
    error, fmt, fs, io,
    path::{self, Path},
    process,
};

use clap::Parser;
use image::ImageFormat;
use uncased::UncasedStr;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Image(BadImage),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("no usable extension: {0}")]
    BadExtension(String),
}

impl Error {
    fn bad_extension(path: impl Into<String>) -> Self {
        Error::BadExtension(path.into())
    }

    fn bad_image(path: impl Into<String>, error: image::ImageError) -> Self {
        Error::Image(BadImage {
            path: path.into(),
            error,
        })
    }
}

#[derive(Debug)]
struct BadImage {
    path: String,
    error: image::ImageError,
}

impl fmt::Display for BadImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.error, self.path)
    }
}

impl error::Error for BadImage {}

#[derive(Clone, Debug, Parser)]
struct Args {
    /// images to be corrected
    #[arg(required = true)]
    images: Vec<String>,

    /// correct image names
    #[arg(short, long)]
    force: bool,
}

impl Args {
    fn paths(&self) -> impl Iterator<Item = &str> {
        self.images.iter().map(AsRef::as_ref)
    }
}

fn main() {
    if let Err(e) = run(&Args::parse()) {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn run(args: &Args) -> Result<()> {
    for path in args.paths() {
        let extension = read_extension(path)?;

        // Not being able to figure out one file type isn't the end of the world.
        let format = match guess_format(path) {
            Ok(format) => format,
            Err(e) => {
                eprintln!("warning: {e}");
                continue;
            }
        };

        if !is_allowed_extension(extension, format) {
            let from = Path::new(path);

            if args.force {
                let to = from.with_extension(preferred_extension(format));
                fs::rename(from, &to)?;
                println!("{}", display_filename(&to));
            } else {
                let preferred_extension = preferred_extension(format);
                println!("{} -> {preferred_extension}", display_filename(from));
            }
        }
    }

    Ok(())
}

fn display_filename(path: &Path) -> path::Display {
    Path::new(path.file_name().unwrap_or(path.as_os_str())).display()
}

fn preferred_extension(format: ImageFormat) -> &'static str {
    format.extensions_str().first().unwrap()
}

fn is_allowed_extension(extension: &str, format: ImageFormat) -> bool {
    let extension: &UncasedStr = extension.into();
    format.extensions_str().iter().any(|&ext| ext == extension)
}

fn guess_format(path: &str) -> Result<ImageFormat> {
    let buffer = fs::read(path)?;
    let format = image::guess_format(&buffer).map_err(|e| Error::bad_image(path, e))?;
    Ok(format)
}

fn read_extension(path: &str) -> Result<&str> {
    let (_stem, extension) = path
        .rsplit_once('.')
        .ok_or_else(|| Error::bad_extension(path))?;
    Ok(extension)
}
