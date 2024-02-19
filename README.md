# Arc Ingester

Read the JSON export from the [Arc](https://www.bigpaua.com/arcapp) app and ingest it into a postgresql database. This
is designed to be run as a cron job or launchctl jjob to keep the database up to date with the latest data from the app.

## Requirements

- The Arc app installed on your iPhone (or possibly other iOS devices?)
- The app set to export data
- A postgresql database to ingest the data into

## Installation

Currently, the ingester is not available as a binary, so you will need to build it from source. You will need to have
[Rust](https://www.rust-lang.org) installed to do this.

```shell
cargo install --git https://github.com/joshuacoles/arc-ingester
```

## Running

The ingester is a command line tool that takes the following arguments,

```shell
arc-ingester --root "~/Library/Mobile Documents/iCloud~com~bigpaua~LearnerCoacher/Documents" --db "postgresql://localhost/arc"
```

see `arc-ingester --help` for more information.

## TODO

- Logging
- Investigate using modified date as a replacement for hashing to determine if a file has changed
    - At the very least, use the modified date to determine if a file has changed and then hash check the ones that have
      the same modified date.
- Multithreading
- Error handling
- What happens if some places or activities are deleted from the app? I think atm they will be left in the database.
