mod frontmatter;
mod listing;

use anyhow::Error;
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use frontmatter::{EntryStatus, Frontmatter};
use git2::{Repository, Sort};
use listing::Listing;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

fn path_to_slug<P: AsRef<Path>>(p: P) -> String {
    p.as_ref().file_stem().unwrap().to_str().unwrap().to_owned()
}

fn emit_markdown<P1, P2>(output: P1, from: P2, slug: &str) -> Result<(), Error>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let mut base = output.as_ref().join("entries");
    std::fs::create_dir_all(&base)?;

    base.push(slug);
    base.set_extension("md");
    println!("Copy from {}", from.as_ref().display());
    std::fs::copy(from, &base)?;

    Ok(())
}

#[derive(StructOpt)]
struct Args {
    // The directory containing all entries
    entries: PathBuf,

    // The output directory.
    #[structopt(default_value = "./gust_generated")]
    output: PathBuf,

    // The git ref for resolving timestamp
    #[structopt(long, short, default_value = "HEAD")]
    gitref: String,
}

#[paw::main]
fn main(args: Args) -> Result<(), Error> {
    let entry_paths = std::fs::read_dir(&args.entries)?;

    let mut entries = Vec::new();

    let repo = Repository::discover(&args.entries)?;
    let base = repo
        .workdir()
        .ok_or(anyhow::anyhow!("Cannot read workdir of git repo!"))?;
    let current_commit = repo
        .resolve_reference_from_short_name(&args.gitref)?
        .peel_to_commit()?;
    let ref_time = current_commit.time();
    let ref_dt: DateTime<Utc> = DateTime::<FixedOffset>::from_utc(
        NaiveDateTime::from_timestamp(ref_time.seconds(), 0),
        FixedOffset::east(ref_time.offset_minutes() * 60),
    )
    .into();
    let current = current_commit.tree()?;

    let mut pending = Vec::new();

    for entry in entry_paths {
        let entry = entry?;

        let entry_path = entry.path().canonicalize()?;

        println!("Staged file {}", entry_path.display());

        let content = std::fs::read_to_string(&entry_path)?;
        let split = content.split("---");
        let second = split.skip(1).next();

        let fm: Frontmatter = if let Some(inner) = second {
            serde_yaml::from_str(inner)?
        } else {
            // Skip because there is no front-matter
            continue;
        };

        if fm.status == EntryStatus::Unspecified {
            // Skip because the status is unspecified
            continue;
        }

        let rel_path = pathdiff::diff_paths(&entry_path, base).unwrap();
        if let Ok(ent) = current.get_path(&rel_path) {
            let oid = ent.id();
            pending.push((rel_path, fm, oid, ref_dt));
        } else {
            let slug = path_to_slug(rel_path);
            println!("Found {} out of tree", slug);

            emit_markdown(&args.output, &entry_path, &slug)?;
            entries.push(fm.into_post(slug, Utc::now()));
        }
    }

    // Try to fetch last_modified time tag from Git
    let mut walker = repo.revwalk()?;
    walker.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    walker.push_ref(&args.gitref)?;

    let mut shuffle = Vec::new();

    for item in walker {
        if pending.len() == 0 {
            break;
        }

        let oid = item?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let time = commit.time();
        let dt: DateTime<Utc> = DateTime::<FixedOffset>::from_utc(
            NaiveDateTime::from_timestamp(time.seconds(), 0),
            FixedOffset::east(time.offset_minutes() * 60),
        )
        .into();

        for (path, fm, oid, last_mod) in pending.drain(..) {
            let slug = path_to_slug(&path);

            let in_tree = tree.get_path(&path);
            let changed = if let Ok(ent) = in_tree {
                ent.id() != oid
            } else {
                true
            };

            if changed {
                println!("Found {} changed from {}", slug, commit.id());

                emit_markdown(&args.output, base.join(&path), &slug)?;
                entries.push(fm.into_post(slug, last_mod));
            } else {
                shuffle.push((path, fm, oid, dt));
            }
        }

        std::mem::swap(&mut shuffle, &mut pending);
    }

    // Those files stay unchanged since the beginning
    for (path, fm, _oid, dt) in pending {
        let slug = path_to_slug(&path);
        println!("{} was there ever since the beginning", slug);

        emit_markdown(&args.output, base.join(&path), &slug)?;
        entries.push(fm.into_post(slug, dt));
    }

    let listing = Listing { entries };

    let listing_path = args.output.join("listing.json");

    std::fs::create_dir_all(&args.output)?;
    println!("Dumping listing to {}", listing_path.display());

    let listing_file = std::fs::File::create(listing_path)?;
    serde_json::to_writer(listing_file, &listing)?;

    Ok(())
}
