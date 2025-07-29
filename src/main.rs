use anyhow::Result;
use bartib::view::status::StatusReport;
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime};
use clap::Parser;

use bartib::data::getter::ActivityFilter;
use bartib::data::processor;

#[cfg(windows)]
use nu_ansi_term::enable_ansi_support;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "A simple timetracker",
    long_about = "To get help for a specific subcommand, run `bartib [SUBCOMMAND] --help`.
To get started, view the `start` help with `bartib start --help`"
)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// the file in which bartib tracks all the activities
    #[arg(short, long, value_name = "FILE", env = "BARTIB_FILE")]
    file: String,
}

#[derive(Parser)]
enum Commands {
    /// starts a new activity
    Start {
        /// the project to which the new activity belongs
        #[arg(short, long)]
        project: String,
        /// the description of the new activity
        #[arg(short, long)]
        description: String,
        /// the time for changing the activity status (HH:MM)
        #[arg(short, long, value_name = "TIME", value_parser = parse_time)]
        time: Option<NaiveTime>,
    },
    /// continues a previous activity
    Continue {
        /// the description of the new activity
        #[arg(short, long)]
        description: Option<String>,
        /// the project to which the new activity belongs
        #[arg(short, long)]
        project: Option<String>,
        /// the number of the activity to continue (see subcommand `last`)
        #[arg(value_name = "NUMBER", default_value = "0")]
        number: usize,
        /// the time for changing the activity status (HH:MM)
        #[arg(short, long, value_name = "TIME", value_parser = parse_time)]
        time: Option<NaiveTime>,
    },
    /// changes the current activity
    Change {
        /// the description of the new activity
        #[arg(short, long)]
        description: Option<String>,
        /// the project to which the new activity belongs
        #[arg(short, long)]
        project: Option<String>,
        /// the time for changing the activity status (HH:MM)
        #[arg(short, long, value_name = "TIME", value_parser = parse_time)]
        time: Option<NaiveTime>,
    },
    /// stops all currently running activities
    Stop {
        /// the time for changing the activity status (HH:MM)
        #[arg(short, long, value_name = "TIME", value_parser = parse_time)]
        time: Option<NaiveTime>,
    },
    /// cancels all currently running activities
    Cancel,
    /// lists all currently running activities
    Current,
    /// list recent activities
    List {
        /// begin of date range (inclusive)
        #[arg(long, value_name = "FROM_DATE", value_parser = parse_date)]
        from: Option<NaiveDate>,
        /// end of date range (inclusive)
        #[arg(long, value_name = "TO_DATE", value_parser = parse_date)]
        to: Option<NaiveDate>,
        /// show activities of a certain date only
        #[arg(short, long, value_name = "DATE", conflicts_with_all = &["from", "to", "today", "yesterday", "current_week", "last_week"], value_parser = parse_date)]
        date: Option<NaiveDate>,
        /// show activities of the current day
        #[arg(long, conflicts_with_all = &["from", "to", "date", "yesterday", "current_week", "last_week"])]
        today: bool,
        /// show yesterdays' activities
        #[arg(long, conflicts_with_all = &["from", "to", "date", "today", "current_week", "last_week"])]
        yesterday: bool,
        /// show activities of the current week
        #[arg(long, conflicts_with_all = &["from", "to", "date", "today", "yesterday", "last_week"])]
        current_week: bool,
        /// show activities of the last week
        #[arg(long, conflicts_with_all = &["from", "to", "date", "today", "yesterday", "current_week"])]
        last_week: bool,
        /// rounds the start and end time to the nearest duration. Durations can be in minutes or hours. E.g. 15m or 4h
        #[arg(long, value_parser = parse_duration)]
        round: Option<Duration>,
        /// do list activities for this project only
        #[arg(short, long)]
        project: Option<String>,
        /// do not group activities by date in list
        #[arg(long)]
        no_grouping: bool,
        /// maximum number of activities to display
        #[arg(short, long, value_name = "NUMBER")]
        number: Option<usize>,
    },
    /// reports duration of tracked activities
    Report {
        /// begin of date range (inclusive)
        #[arg(long, value_name = "FROM_DATE", value_parser = parse_date)]
        from: Option<NaiveDate>,
        /// end of date range (inclusive)
        #[arg(long, value_name = "TO_DATE", value_parser = parse_date)]
        to: Option<NaiveDate>,
        /// show activities of a certain date only
        #[arg(short, long, value_name = "DATE", conflicts_with_all = &["from", "to", "today", "yesterday", "current_week", "last_week"], value_parser = parse_date)]
        date: Option<NaiveDate>,
        /// show activities of the current day
        #[arg(long, conflicts_with_all = &["from", "to", "date", "yesterday", "current_week", "last_week"])]
        today: bool,
        /// show yesterdays' activities
        #[arg(long, conflicts_with_all = &["from", "to", "date", "today", "current_week", "last_week"])]
        yesterday: bool,
        /// show activities of the current week
        #[arg(long, conflicts_with_all = &["from", "to", "date", "today", "yesterday", "last_week"])]
        current_week: bool,
        /// show activities of the last week
        #[arg(long, conflicts_with_all = &["from", "to", "date", "today", "yesterday", "current_week"])]
        last_week: bool,
        /// rounds the start and end time to the nearest duration. Durations can be in minutes or hours. E.g. 15m or 4h
        #[arg(long, value_parser = parse_duration)]
        round: Option<Duration>,
        /// do report activities for this project only
        #[arg(short, long)]
        project: Option<String>,
    },
    /// displays the descriptions and projects of recent activities
    Last {
        /// maximum number of lines to display
        #[arg(short, long, value_name = "NUMBER", default_value = "10")]
        number: usize,
    },
    /// list all projects
    Projects {
        /// prints currently running projects only
        #[arg(short, long)]
        current: bool,
        /// prints projects without quotes
        #[arg(short, long)]
        no_quotes: bool,
    },
    /// opens the activity log in an editor
    Edit {
        /// the command to start your preferred text editor
        #[arg(short, long, value_name = "EDITOR", env = "EDITOR")]
        editor: Option<String>,
    },
    /// checks file and reports parsing errors
    Check,
    /// checks sanity of bartib log
    Sanity,
    /// search for existing descriptions and projects
    Search {
        /// the search term
        #[arg(default_value = "")]
        search_term: String,
    },
    /// shows current status and time reports for today, current week, and current month
    Status {
        /// show status for this project only
        #[arg(short, long)]
        project: Option<String>,
    },
}

fn main() -> Result<()> {
    #[cfg(windows)]
    if let Err(e) = enable_ansi_support() {
        println!("Could not enable ansi support! Errorcode: {}", e);
    }

    let cli = Cli::parse();

    run_subcommand(cli)
}

fn run_subcommand(cli: Cli) -> Result<()> {
    let file_name = &cli.file;
    match cli.command {
        Commands::Start {
            project,
            description,
            time,
        } => {
            let time = time.map(|t| Local::now().date_naive().and_time(t));

            bartib::controller::manipulation::start(file_name, &project, &description, time)
        }
        Commands::Change {
            project,
            description,
            time,
        } => {
            let time = time.map(|t| Local::now().date_naive().and_time(t));

            bartib::controller::manipulation::change(
                file_name,
                project.as_deref(),
                description.as_deref(),
                time,
            )
        }
        Commands::Continue {
            project,
            description,
            time,
            number,
        } => {
            let time = time.map(|t| Local::now().date_naive().and_time(t));

            bartib::controller::manipulation::continue_last_activity(
                file_name,
                project.as_deref(),
                description.as_deref(),
                time,
                number,
            )
        }
        Commands::Stop { time } => {
            let time = time.map(|t| Local::now().date_naive().and_time(t));

            bartib::controller::manipulation::stop(file_name, time)
        }
        Commands::Cancel => bartib::controller::manipulation::cancel(file_name),
        Commands::Current => bartib::controller::list::list_running(file_name),
        Commands::List {
            from,
            to,
            date,
            today,
            yesterday,
            current_week,
            last_week,
            round,
            project,
            no_grouping,
            number,
        } => {
            let filter = ActivityFilter::new(
                number,
                from,
                to,
                date,
                project.as_deref(),
                today,
                yesterday,
                current_week,
                last_week,
            );
            let processors = create_processors(round);
            let do_group_activities = !no_grouping && filter.date.is_none();
            bartib::controller::list::list(file_name, filter, do_group_activities, processors)
        }
        Commands::Report {
            from,
            to,
            date,
            today,
            yesterday,
            current_week,
            last_week,
            round,
            project,
        } => {
                        let filter = ActivityFilter::new(
                None,
                from,
                to,
                date,
                project.as_deref(),
                today,
                yesterday,
                current_week,
                last_week,
            );
            let processors = create_processors(round);
            bartib::controller::report::show_report(file_name, filter, processors)
        }
        Commands::Projects { current, no_quotes } => {
            bartib::controller::list::list_projects(file_name, current, no_quotes)
        }
        Commands::Last { number } => {
            bartib::controller::list::list_last_activities(file_name, number)
        }
        Commands::Edit { editor } => {
            bartib::controller::manipulation::start_editor(file_name, editor.as_deref())
        }
        Commands::Check => bartib::controller::list::check(file_name),
        Commands::Sanity => bartib::controller::list::sanity_check(file_name),
        Commands::Search { search_term } => {
            bartib::controller::list::search(file_name, Some(&search_term))
        }
        Commands::Status { project } => {
            let filter = ActivityFilter {
                number_of_activities: None,
                from_date: None,
                to_date: None,
                date: None,
                project: project.as_deref(),
            };
            let processors = create_processors(None);
            let writer = create_status_writer();
            bartib::controller::status::show_status(file_name, filter, processors, writer.as_ref())
        }
    }
}

fn create_processors(round: Option<Duration>) -> processor::ProcessorList {
    let mut processors: Vec<Box<dyn processor::ActivityProcessor>> = Vec::new();

    if let Some(round) = round {
        processors.push(Box::new(processor::RoundProcessor { round }));
    }

    processors
}

fn create_status_writer() -> Box<dyn processor::StatusReportWriter> {
    let result = StatusReport {};
    Box::new(result)
}

fn apply_date_presets(
    filter: &mut ActivityFilter,
    today: bool,
    yesterday: bool,
    current_week: bool,
    last_week: bool,
) {
    let now = Local::now().naive_local().date();
    if today {
        filter.date = Some(now);
    }

    if yesterday {
        filter.date = Some(now - Duration::days(1));
    }

    if current_week {
        filter.from_date =
            Some(now - Duration::days(i64::from(now.weekday().num_days_from_monday())));
        filter.to_date = Some(
            now - Duration::days(i64::from(now.weekday().num_days_from_monday()))
                + Duration::days(6),
        );
    }

    if last_week {
        filter.from_date = Some(
            now - Duration::days(i64::from(now.weekday().num_days_from_monday()))
                - Duration::weeks(1),
        );
        filter.to_date = Some(
            now - Duration::days(i64::from(now.weekday().num_days_from_monday()))
                - Duration::weeks(1)
                + Duration::days(6),
        )
    }
}

fn parse_date(date_string: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(date_string, bartib::conf::FORMAT_DATE).map_err(|e| e.to_string())
}

fn parse_time(time_string: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(time_string, bartib::conf::FORMAT_TIME).map_err(|e| e.to_string())
}

fn parse_duration(duration_string: &str) -> Result<Duration, String> {
    let (number_string, duration_unit) = duration_string.split_at(duration_string.len() - 1);
    let number: i64 = number_string
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    match duration_unit {
        "m" => Ok(Duration::minutes(number)),
        "h" => Ok(Duration::hours(number)),
        _ => Err(format!(
            "invalid duration unit '{duration_unit}', expected 'm' or 'h'"
        )),
    }
}
