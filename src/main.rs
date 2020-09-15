mod listing;
mod frontmatter;

use listing::Listing;
use frontmatter::{Frontmatter, EntryStatus};
use structopt::StructOpt;
use std::path::PathBuf;
use anyhow::Error;

#[derive(StructOpt)]
struct Args {
    // The directory containing all entries
    entries: PathBuf,

    // The output directory.
    #[structopt(default_value=".")]
    output: PathBuf,
}

#[paw::main]
fn main(args: Args) -> Result<(), Error> {
    let entry_paths = std::fs::read_dir(&args.entries)?;

    let mut entries = Vec::new();

    for entry in entry_paths {
        let entry = entry?;

        println!("Working on file {}", entry.path().display());

        let content = std::fs::read_to_string(entry.path())?;
        let split = content.split("---");
        let second = split.skip(1).next();

        let fm: Frontmatter = if let Some(inner) = second {
            serde_yaml::from_str(inner)?
        } else {
            // Skip because there is no front-matter
            continue
        };

        if fm.status == EntryStatus::Unspecified {
            // Skip because the status is unspecified
            continue;
        }

        entries.push(fm.into_post(entry.path().file_stem().unwrap().to_str().unwrap().to_owned()));
    }

    let listing = Listing { entries };

    let listing_path = args.output.join("listing.json");

    std::fs::create_dir_all(&args.output)?;
    println!("Dumping listing to {}", listing_path.display());

    let listing_file = std::fs::File::create(listing_path)?;
    serde_json::to_writer(listing_file, &listing)?;

    Ok(())
}
