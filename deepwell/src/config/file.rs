/*
 * config/file.rs
 *
 * DEEPWELL - Wikijump API provider and database manager
 * Copyright (C) 2019-2023 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

use super::Config;
use anyhow::Result;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;
use tide::log::LevelFilter;
use time::Duration as TimeDuration;

/// Structure representing a configuration file.
///
/// This differs from the `Config` struct because
/// it contains sub-sections which are used within
/// the TOML file which are then flattened before
/// being used during execution.
///
/// This also lets us customize certain parts of
/// how serialization and deserialization occur.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigFile {
    logger: Logger,
    server: Server,
    database: Database,
    security: Security,
    locale: Locale,
    domain: Domain,
    job: Job,
    ftml: Ftml,
    special_pages: SpecialPages,
    user: User,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Logger {
    enable: bool,
    level: LevelFilter,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Server {
    address: SocketAddr,
    pid_file: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Database {
    run_migrations: bool,
    run_seeder: bool,
    seeder_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Security {
    authentication_fail_delay_ms: u64,
    session: Session,
    mfa: Mfa,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Session {
    token_prefix: String,
    token_length: usize,
    duration_session_minutes: u64,
    duration_login_minutes: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Mfa {
    recovery_code_count: usize,
    recovery_code_length: usize,
    time_step: u64,
    time_skew: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Job {
    delay_ms: u64,
    prune_session_secs: u64,
    prune_text_secs: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Locale {
    path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Domain {
    main: String,
    files: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct Ftml {
    render_timeout_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct SpecialPages {
    special_prefix: String,
    template: String,
    missing: String,
    private: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
struct User {
    default_name_changes: u8,
    maximum_name_changes: u8,
    refill_name_change_days: u64,
    minimum_name_bytes: usize,
}

impl ConfigFile {
    pub fn load(path: &Path) -> Result<(Self, String)> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config = toml::from_str(&contents)?;
        Ok((config, contents))
    }

    /// Deconstruct the `ConfigFile` and flatten it as a `Config` object.
    pub fn into_config(self, raw_toml: String) -> Config {
        macro_rules! time_duration {
            // Convert a stdlib duration into a 'time' crate duration
            ($method:ident, $value:expr $(,)?) => {{
                let std_duration = StdDuration::$method($value);
                let time_duration = TimeDuration::try_from(std_duration)
                    .expect("Unable to convert from standard to time::Duration");

                time_duration
            }};
        }

        let ConfigFile {
            logger:
                Logger {
                    enable: logger,
                    level: logger_level,
                },
            server:
                Server {
                    address,
                    mut pid_file,
                },
            database:
                Database {
                    run_migrations,
                    run_seeder,
                    seeder_path,
                },
            security:
                Security {
                    authentication_fail_delay_ms,
                    session:
                        Session {
                            token_prefix,
                            token_length,
                            duration_session_minutes,
                            duration_login_minutes,
                        },
                    mfa:
                        Mfa {
                            recovery_code_count,
                            recovery_code_length,
                            time_step,
                            time_skew,
                        },
                },
            domain:
                Domain {
                    main: main_domain,
                    files: files_domain,
                },
            job:
                Job {
                    delay_ms: job_delay_ms,
                    prune_session_secs,
                    prune_text_secs,
                },
            locale: Locale {
                path: localization_path,
            },
            ftml: Ftml { render_timeout_ms },
            special_pages:
                SpecialPages {
                    special_prefix: special_page_prefix,
                    template: special_page_template,
                    missing: special_page_missing,
                    private: special_page_private,
                },
            user:
                User {
                    default_name_changes,
                    maximum_name_changes,
                    refill_name_change_days,
                    minimum_name_bytes,
                },
        } = self;

        // Prefix domains with '.' so we can do easy subdomain checks
        // and concatenations.
        let (main_domain, main_domain_no_dot) = prefix_domain(main_domain);
        let (files_domain, files_domain_no_dot) = prefix_domain(files_domain);

        // Treats empty strings (which aren't valid paths anyways)
        // as null for the purpose of pid_file.
        if let Some(ref path) = pid_file {
            if path.as_os_str().is_empty() {
                pid_file = None;
            }
        }

        Config {
            raw_toml,
            logger,
            logger_level,
            address,
            pid_file,
            main_domain,
            main_domain_no_dot,
            files_domain,
            files_domain_no_dot,
            run_migrations,
            run_seeder,
            seeder_path,
            localization_path,
            authentication_fail_delay: StdDuration::from_millis(
                authentication_fail_delay_ms,
            ),
            session_token_prefix: token_prefix,
            session_token_length: token_length,
            normal_session_duration: time_duration!(
                from_secs,
                duration_session_minutes * 60,
            ),
            restricted_session_duration: time_duration!(
                from_secs,
                duration_login_minutes * 60,
            ),
            recovery_code_count,
            recovery_code_length,
            totp_time_step: time_step,
            totp_time_skew: time_skew,
            job_delay: StdDuration::from_millis(job_delay_ms),
            job_prune_session_period: StdDuration::from_secs(prune_session_secs),
            job_prune_text_period: StdDuration::from_secs(prune_text_secs),
            render_timeout: StdDuration::from_millis(render_timeout_ms),
            special_page_prefix,
            special_page_template,
            special_page_missing,
            special_page_private,
            default_name_changes: i16::from(default_name_changes),
            maximum_name_changes: i16::from(maximum_name_changes),
            refill_name_change: StdDuration::from_secs(
                refill_name_change_days * 24 * 60 * 60,
            ),
            minimum_name_bytes,
        }
    }
}

/// Takes a domain, and returns a value with and without a leading `.`
///
/// # Returns
/// Tuple with two items. First has a leading `.`, second does not.
fn prefix_domain(domain: String) -> (String, String) {
    if domain.starts_with('.') {
        let mut no_dot = domain.clone();
        no_dot.remove(0);

        (domain, no_dot)
    } else {
        let dot_domain = format!(".{domain}");

        (dot_domain, domain)
    }
}

#[test]
fn test_prefix_domain() {
    macro_rules! check {
        ($input:expr; $output_1:expr, $output_2:expr $(,)?) => {{
            let (actual_1, actual_2) = prefix_domain(str!($input));

            assert_eq!(actual_1, $output_1, "Actual dot domain doesn't match");
            assert_eq!(actual_2, $output_2, "Actual no-dot domain doesn't match");
        }};
    }

    check!("example.com"; ".example.com", "example.com");
    check!(".example.com"; ".example.com", "example.com");
}
