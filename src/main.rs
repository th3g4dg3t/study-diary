/*
    Copyright (C) 2026  Andrea Cingolani

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions, copy, create_dir_all, read_to_string};
use std::io::{Write, stdin, stdout};

use chrono::{Local, MappedLocalTime, NaiveDateTime, TimeZone};
use clap::{Parser, Subcommand};
use color_eyre::eyre::{Report, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json;

const TIME_FMT: &str = "%d/%m/%y %H:%M:%S";

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Manage and display active fields
    ActiveFields {
        /// Add some active fields (comma-separated, eg. --add "field1,...")
        #[arg(short, long, value_delimiter = ',')]
        add: Option<Vec<String>>,
        /// Remove some active fields (comma-separated, eg. --remove "field1,...")
        #[arg(short, long, value_delimiter = ',')]
        remove: Option<Vec<String>>,
    },
    /// Add an entry to the diary
    AddEntry {
        /// Manually set a different timestamp than the current. Time of the entry must be in "DD/MM/YY HH:MM:SS" format
        #[arg(short, long)]
        timestamp: Option<String>,
    },
    /// Modify an entry in the diary
    ChangeEntry {
        /// Time of the entry in format "DD/MM/YY HH:MM:SS"
        #[arg(short, long)]
        timestamp: String,
    },
    /// Remove an entry from the diary
    RemoveEntry {
        /// Time of the entry in format "DD/MM/YY HH:MM:SS"
        #[arg(short, long)]
        timestamp: String,
    },
    /// Print the diary on the screen
    PrintDiary,
}

#[derive(Debug, Deserialize, Serialize)]
struct Diary {
    active_fields: HashSet<String>,
    diary_entries: Vec<DiaryEntry>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct DiaryEntry {
    timestamp: i64,
    date: String,
    fields: HashMap<String, i8>,
    note: Option<String>,
}

impl DiaryEntry {
    // Construct a new DiaryEntry asking the user
    fn from_user_input(
        timestamp: i64,
        date: String,
        active_fields: &HashSet<String>,
    ) -> Result<Self> {
        let mut entry = DiaryEntry::default();

        entry.timestamp = timestamp;
        entry.date = date;

        for field in active_fields {
            let mut user_input = String::new();
            print!("{}: ", field);
            stdout().flush()?;
            stdin().read_line(&mut user_input)?;
            entry
                .fields
                .insert(field.clone(), user_input.trim().parse()?);
        }

        let mut user_input = String::new();
        print!("Note: ");
        stdout().flush()?;
        stdin().read_line(&mut user_input)?;
        let user_input = user_input.trim().to_string();
        if !user_input.is_empty() {
            entry.note = Some(user_input)
        }

        Ok(entry)
    }
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

    // Create directory if doesn't exist already
    create_dir_all(&data_dir_path)?;

    // Define the file handle opening or creating the default
    let mut diary_isnt_new = true;
    let mut diary = {
        if !diary_path.exists() {
            File::create_new(&diary_path)?;
        }

        let diary_contents = read_to_string(&diary_path)?;

        if diary_contents.trim().is_empty() {
            diary_isnt_new = false;
            let mut active_fields: HashSet<String>;
            println!("Your diary seems empty. Let's start by adding some active fields.");
            loop {
                println!("Write a string of values (comma-separated): ");
                let mut user_input = String::new();
                stdin().read_line(&mut user_input)?;
                active_fields = user_input
                    .trim()
                    .split(',')
                    .filter_map(|field| {
                        let field = field.trim();
                        if field.is_empty() {
                            None
                        } else {
                            Some(field.to_string())
                        }
                    })
                    .collect();
                if active_fields.is_empty() {
                    continue;
                } else {
                    break;
                }
            }
            Diary {
                active_fields,
                diary_entries: Vec::new(),
            }
        } else {
            serde_json::from_str(&diary_contents)?
        }
    };

    match cli.command {
        // Manage the active fields
        Commands::ActiveFields { add, remove } => {
            let mut display_fields = true;

            if let Some(add) = add {
                display_fields = false;
                let add: Vec<_> = add
                    .into_iter()
                    .map(|field| field.trim().to_string())
                    .collect();
                diary.active_fields.extend(add);
            }

            if let Some(remove) = remove {
                display_fields = false;
                let remove: Vec<_> = remove
                    .into_iter()
                    .map(|field| field.trim().to_string())
                    .collect();
                remove.iter().for_each(|field| {
                    diary.active_fields.remove(field);
                });
            }

            if display_fields {
                let active_fields_str = diary
                    .active_fields
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("{active_fields_str}");

                // The diary has not been modified so don't bother updating the file, unless is new
                if diary_isnt_new {
                    return Ok(());
                }
            }
        }
        // Handle adding a new entry to the diary
        Commands::AddEntry {
            timestamp: timestring,
        } => {
            let (timestamp, date) = if let Some(timestring) = timestring {
                (timestring_to_timestamp(&timestring)?, timestring)
            } else {
                let now = Local::now();
                (now.timestamp(), now.format(TIME_FMT).to_string())
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

            let new_entry = DiaryEntry::from_user_input(timestamp, date, &diary.active_fields)?;
            diary.diary_entries.push(new_entry);
        }
        // Handle changing an entry in the diary
        Commands::ChangeEntry {
            timestamp: timestring,
        } => {
            let (timestamp, date) = (timestring_to_timestamp(&timestring)?, timestring);

            if let Some(old_entry) = diary
                .diary_entries
                .iter_mut()
                .find(|old_diary_entry| old_diary_entry.timestamp == timestamp)
            {
                println!("Old diary entry:\n{:#?}\nNew entry:\n", old_entry);
                let old_entry_fields: HashSet<_> = old_entry.fields.keys().cloned().collect();
                *old_entry = DiaryEntry::from_user_input(timestamp, date, &old_entry_fields)?;
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
        // Print all the entries
        Commands::PrintDiary => {
            for entry in &diary.diary_entries {
                println!("===== {} =====", entry.date);
                for (field, val) in &entry.fields {
                    println!("{field}: {val}");
                }
                if let Some(note) = &entry.note {
                    println!("{note}");
                }
                println!();
            }
            // The diary has not been modified so don't bother updating the file
            return Ok(());
        }
    }

    // Sort the diary chronologically
    diary.diary_entries.sort_by_key(|entry| entry.timestamp);

    // Serialize the new contents in the diary
    let mut new_diary_content = serde_json::to_vec_pretty(&diary)?;
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
