use std::fs::{File, OpenOptions, copy, create_dir_all, read_to_string};
use std::io::{Write, stdin, stdout};
use std::path::PathBuf;

use chrono::{Local, MappedLocalTime, NaiveDateTime, TimeZone, Utc};
use clap::{Parser, Subcommand};
use color_eyre::eyre::{Report, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json;

const TIME_FMT: &str = "%d/%m/%y %H:%M";

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add an entry to the diary
    AddEntry {
        /// Manually set a different timestamp than the current. Time of the entry must be in "DD/MM/YY HH:MM" format
        #[arg(short, long)]
        timestamp: Option<String>,
    },
    /// Modify an entry in the diary
    ChangeEntry {
        /// Time of the entry in format "DD/MM/YY HH:MM
        timestamp: String,
    },
    /// Remove an entry from the diary
    RemoveEntry {
        /// Time of the entry in format "DD/MM/YY HH:MM"
        timestamp: String,
    },
    /// Print the diary on the screen
    PrintDiary,
}

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct DiaryEntry {
    timestamp: i64,
    mood: u8,
    anxiety: u8,
    sadness: u8,
    anger: u8,
    tiredness: u8,
    restlessness: u8,
    note: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Diary {
    diary_entries: Vec<DiaryEntry>,
}

fn main() -> Result<()> {
    // Get arguments
    let cli = Cli::parse();

    // Set up color_eyre hooks for panics and errors
    color_eyre::install()?;

    // Define the directory path where files are stored
    let data_dir_path = ProjectDirs::from("", "", "study-diary")
        .ok_or(Report::msg(
            "No valid home directory path could be retrieved.",
        ))?
        .data_dir()
        .to_path_buf();

    // Define the path for the diary file
    let diary_path = {
        let mut diary_path = data_dir_path.clone();
        diary_path.push("diary.json");
        diary_path
    };

    // Check if directory already exists or if it path is something else
    if !data_dir_path.exists() {
        create_dir_all(&data_dir_path)?;
    } else if !data_dir_path.is_dir() {
        let name = data_dir_path.to_string_lossy();
        return Err(Report::msg(format!(
            "Objectc with name '{}' already exists.",
            name
        )));
    }

    // Define the file handle opening or creating it
    let mut diary = {
        if !diary_path.exists() {
            File::create_new(&diary_path)?;
        }

        let diary_contents = read_to_string(&diary_path)?;

        if diary_contents.trim().is_empty() {
            Diary {
                diary_entries: Vec::new(),
            }
        } else {
            serde_json::from_str(&diary_contents)?
        }
    };

    match cli.command {
        Commands::AddEntry {
            timestamp: timestring,
        } => {
            let timestamp = if let Some(timestring) = timestring {
                let naive_datetime = NaiveDateTime::parse_from_str(&timestring, TIME_FMT)?;
                let local_datetime = TimeZone::from_local_datetime(&Local, &naive_datetime);
                match local_datetime {
                    MappedLocalTime::Single(datetime) => datetime.timestamp(),
                    MappedLocalTime::Ambiguous(_, _) => {
                        let report = Report::msg(
                            "The local time is ambiguous because there is a fold in the local time.",
                        );
                        return Err(report);
                    }
                    MappedLocalTime::None => {
                        let report = Report::msg("The local time does not exist because there is a gap in the local time.
This error may also be returned if there was an error while resolving the local time,
caused by for example missing time zone data files, an error in an OS API, or overflow."
                    );
                        return Err(report);
                    }
                }
            } else {
                Utc::now().timestamp()
            };

            if diary
                .diary_entries
                .iter()
                .map(|entry| entry.timestamp)
                .any(|old_timestamp| old_timestamp == timestamp)
            {
                let report = Report::msg("An entry with this timestamp is already present.");
                return Err(report);
            }

            let new_entry = DiaryEntry::from_user_input(timestamp)?;
            diary.diary_entries.push(new_entry);
        }
        _ => (),
    }

    let mut new_diary_content = serde_json::to_vec(&diary)?;
    new_diary_content.push(b'\n');

    let mut backup_diary_path = diary_path.clone();
    backup_diary_path.push(PathBuf::from("~"));
    copy(&diary_path, &backup_diary_path)?;

    let mut diary_file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .open(&diary_path)?;

    diary_file.write_all(&new_diary_content)?;

    Ok(())
}

impl DiaryEntry {
    // Construct a new DiaryEntry asking the user
    fn from_user_input(timestamp: i64) -> Result<Self> {
        let mut input = String::new();
        let mut entry = DiaryEntry::default();

        entry.timestamp = timestamp;

        print!("Mood: ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
        entry.mood = input.trim().parse()?;
        input.clear();

        print!("Anxiety: ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
        entry.anxiety = input.trim().parse()?;
        input.clear();

        print!("Sadness: ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
        entry.sadness = input.trim().parse()?;
        input.clear();

        print!("Anger: ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
        entry.anger = input.trim().parse()?;
        input.clear();

        print!("Tiredness: ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
        entry.tiredness = input.trim().parse()?;
        input.clear();

        print!("Restlessness: ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
        entry.restlessness = input.trim().parse()?;
        input.clear();

        print!("Note: ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
        entry.note = if input.trim().is_empty() {
            None
        } else {
            Some(input)
        };

        Ok(entry)
    }
}
