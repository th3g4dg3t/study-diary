use std::fs::{File, OpenOptions, copy, create_dir_all, read_to_string};
use std::io::{Write, stdin, stdout};

use chrono::{DateTime, Local, MappedLocalTime, NaiveDateTime, TimeZone, Utc};
use clap::{Parser, Subcommand};
use color_eyre::eyre::{Report, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json;

const TIME_FMT: &str = "%d/%m/%y %H:%M:%S";

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add an entry to the diary
    AddEntry {
        /// Manually set a different timestamp than the current. Time of the entry must be in "DD/MM/YY HH:MM:SS" format
        #[arg(short, long)]
        timestamp: Option<String>,
    },
    /// Modify an entry in the diary
    ChangeEntry {
        /// Time of the entry in format "DD/MM/YY HH:MM:SS"
        timestamp: String,
    },
    /// Remove an entry from the diary
    RemoveEntry {
        /// Time of the entry in format "DD/MM/YY HH:MM:SS"
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
            "Object with name '{}' already exists.",
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
        // Handle adding a new entry to the diary
        Commands::AddEntry {
            timestamp: timestring,
        } => {
            let timestamp = if let Some(timestring) = timestring {
                timestring_to_timestamp(&timestring)?
            } else {
                Utc::now().timestamp()
            };

            if let Some(old_entry) = diary
                .diary_entries
                .iter()
                .find(|old_diary_entry| old_diary_entry.timestamp == timestamp)
            {
                let report = Report::msg(format!(
                    "An entry with this timestamp is already present:\n{:#?}",
                    old_entry
                ));
                return Err(report);
            }

            let new_entry = DiaryEntry::from_user_input(timestamp)?;
            diary.diary_entries.push(new_entry);
        }
        // Handle changing an entry in the diary
        Commands::ChangeEntry {
            timestamp: timestring,
        } => {
            let timestamp = timestring_to_timestamp(&timestring)?;

            if let Some(old_entry) = diary
                .diary_entries
                .iter_mut()
                .find(|old_diary_entry| old_diary_entry.timestamp == timestamp)
            {
                println!("Old diary entry:\n{:#?}\nNew entry:\n", old_entry);

                *old_entry = DiaryEntry::from_user_input(timestamp)?;
            } else {
                let report = Report::msg("No entry with the given timestamp found.");
                return Err(report);
            }
        }
        // Handle removing an entry from the diary
        Commands::RemoveEntry {
            timestamp: timestring,
        } => {
            let timestamp = timestring_to_timestamp(&timestring)?;

            let entry_position = diary
                .diary_entries
                .iter()
                .position(|old_diary_entry| old_diary_entry.timestamp == timestamp);

            if let Some(old_entry_position) = entry_position {
                loop {
                    println!(
                        "Are you sure you want to delete this entry? [yes/no]\n{:#?}",
                        diary.diary_entries[old_entry_position]
                    );
                    let mut decision = String::new();
                    stdin().read_line(&mut decision)?;
                    match decision.trim() {
                        "yes" => {
                            diary.diary_entries.swap_remove(old_entry_position);
                            break;
                        }
                        "no" => {
                            // The diary has not been modified so don't bother updating the file
                            return Ok(());
                        }
                        _ => continue,
                    }
                }
            } else {
                let report = Report::msg("No entry with the given timestamp found.");
                return Err(report);
            }
        }
        // Print all the entries in a CSV format
        Commands::PrintDiary => {
            for entry in &diary.diary_entries {
                let DiaryEntry {
                    timestamp,
                    mood,
                    anxiety,
                    sadness,
                    anger,
                    tiredness,
                    restlessness,
                    note,
                } = entry;

                let note = if let Some(entry_note) = note {
                    entry_note.as_str()
                } else {
                    ""
                };

                let utc_datetime = DateTime::from_timestamp_secs(*timestamp);
                let local_datetime = if let Some(datetime) = utc_datetime {
                    let naive_datetime = datetime.naive_utc();
                    Local
                        .from_utc_datetime(&naive_datetime)
                        .format(TIME_FMT)
                        .to_string()
                } else {
                    String::from("out-of-range number of seconds")
                };

                println!(
                    "{},{},{},{},{},{},{},{}",
                    local_datetime, mood, anxiety, sadness, anger, tiredness, restlessness, note
                );
            }

            // The diary has not been modified so don't bother updating the file
            return Ok(());
        }
    }

    // Sort the diary chronologically
    diary.diary_entries.sort_by_key(|entry| entry.timestamp);

    // Serialize the new contents in the diary
    let mut new_diary_content = serde_json::to_vec(&diary)?;
    new_diary_content.push(b'\n');

    // Define the path to the backup copy of the diary e copy the current diary
    let mut backup_diary_path = diary_path.clone();
    backup_diary_path.set_extension("json~");
    copy(&diary_path, &backup_diary_path)?;

    // Open the diary and owerwrite its contents
    let mut diary_file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .open(&diary_path)?;
    diary_file.write_all(&new_diary_content)?;

    Ok(())
}

// Get the timestamp from a date and time string
fn timestring_to_timestamp(timestring: &str) -> Result<i64> {
    let naive_datetime = NaiveDateTime::parse_from_str(&timestring, TIME_FMT)?;
    let local_datetime = TimeZone::from_local_datetime(&Local, &naive_datetime);

    match local_datetime {
        MappedLocalTime::Single(datetime) => Ok(datetime.timestamp()),
        MappedLocalTime::Ambiguous(_, _) => {
            let report = Report::msg(
                "The local time is ambiguous because there is a fold in the local time.",
            );

            Err(report)
        }
        MappedLocalTime::None => {
            let report = Report::msg(
                "The local time does not exist because there is a gap in the local time.
This error may also be returned if there was an error while resolving the local time,
caused by for example missing time zone data files, an error in an OS API, or overflow.",
            );

            Err(report)
        }
    }
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
            Some(input.trim().to_string())
        };

        Ok(entry)
    }
}
