# Study Diary

A command-line interface (CLI) application written in Rust to manage and track your study sessions and diary entries.

## Features

- Keep track of your study activities locally.
- Manage customizable active fields for your diary entries.
- Add, modify, remove, and print your study logs entirely from the terminal.
- Data is securely saved locally in a `diary.json` file within your operating system's standard user data directory.

## Requirements

To build and run this project, you need to have [Rust and Cargo](https://www.rust-lang.org/tools/install) installed on your system.

## Installation

Clone the repository and build the project using Cargo:

```
bash
git clone <your-repository-url>
cd study-diary
cargo build --release
```

You can find the compiled executable in the `target/release/` directory.

## Usage

The application provides several commands to manageyour study diary:
- Manage active fields:  
Add, remove, or list the fields you want to track.

```
bash
study-diary active-fields
```

- Add a new entry:  
Record a new study session. You can optionally use the `--timestamp` flag to provide a custom time.

```
bash
study-diary add-entry
```

- Modify an entry:  
Change an existing entry by providing its timestamp.

```
bash
study-diary change-entry <TIMESTAMP>
```

- Remove an entry:  
Delete a specific entry from your diary using its timestamp.

```
bash
study-diary remove-entry <TIMESTAMP>
```

- View the diary:  
Print all the recorded entries in chronological order.

```
bash
study-diary print-diary
```

## License

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

Copyright (C) 2026 Andrea Cingolani
