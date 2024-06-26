use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use rand::Rng;
use russh_keys::key::PublicKey;
use std::{collections::BTreeSet, fmt::Display, time::Duration};

use crate::utils;

use super::status::UserStatus;
use super::theme::UserTheme;
use super::timestamp_mode::TimestampMode;

#[derive(Clone, Debug)]
pub struct User {
    pub id: usize,
    pub username: String,
    pub status: UserStatus,
    pub joined_at: DateTime<Utc>,
    pub ssh_client: String,
    pub public_key: Option<PublicKey>,
    pub reply_to: Option<usize>,
    pub theme: UserTheme,
    pub quiet: bool,
    pub is_op: bool,
    pub is_muted: bool,
    pub timestamp_mode: TimestampMode,
    pub ignored: BTreeSet<usize>,
    pub focused: BTreeSet<usize>,
}

impl User {
    pub fn new(
        id: usize,
        username: String,
        ssh_client: String,
        key: Option<PublicKey>,
        is_op: bool,
    ) -> Self {
        Self {
            id,
            username,
            ssh_client,
            is_op,
            public_key: key,
            joined_at: Utc::now(),
            reply_to: None,
            quiet: false,
            is_muted: false,
            status: Default::default(),
            theme: Default::default(),
            timestamp_mode: Default::default(),
            ignored: BTreeSet::new(),
            focused: BTreeSet::new(),
        }
    }

    pub fn switch_quiet_mode(&mut self) {
        self.quiet = !self.quiet;
    }

    pub fn switch_mute_mode(&mut self) {
        self.is_muted = !self.is_muted;
    }

    pub fn set_timestamp_mode(&mut self, mode: TimestampMode) {
        self.timestamp_mode = mode;
    }

    pub fn go_away(&mut self, reason: String) {
        self.status = UserStatus::Away {
            reason,
            since: Utc::now(),
        };
    }

    pub fn return_active(&mut self) {
        self.status = UserStatus::Active;
    }

    pub fn set_new_name(&mut self, username: String) {
        self.username = username;
    }

    pub fn joined_duration(&self) -> Duration {
        let now = Utc::now();
        let secs = now.signed_duration_since(self.joined_at).num_seconds() as u64;
        Duration::from_secs(secs)
    }

    pub fn set_reply_to(&mut self, reply_to: usize) {
        self.reply_to = Some(reply_to);
    }

    pub fn gen_rand_name() -> String {
        let adjectives = [
            "Cool", "Mighty", "Brave", "Clever", "Happy", "Calm", "Eager", "Gentle", "Kind",
            "Jolly", "Swift", "Bold", "Fierce", "Wise", "Valiant", "Bright", "Noble", "Zany",
            "Epic",
        ];
        let nouns = [
            "Tiger", "Eagle", "Panda", "Shark", "Lion", "Wolf", "Dragon", "Phoenix", "Hawk",
            "Bear", "Falcon", "Panther", "Griffin", "Lynx", "Orca", "Cobra", "Jaguar", "Kraken",
            "Pegasus", "Stallion",
        ];

        let mut rng = rand::thread_rng();
        let adjective = adjectives.choose(&mut rng).unwrap();
        let noun = nouns.choose(&mut rng).unwrap();
        let number: u16 = rng.gen_range(1..=9999);

        format!("{}{}{}", adjective, noun, number)
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fingerprint = match &self.public_key {
            Some(key) => format!("SHA256: {}", key.fingerprint()),
            None => "(no public key)".to_string(),
        };

        write!(
            f,
            "name: {}{} > fingerprint: {}{} > client: {}{} > joined: {} ago",
            self.username,
            utils::NEWLINE,
            fingerprint,
            utils::NEWLINE,
            self.ssh_client,
            utils::NEWLINE,
            humantime::format_duration(self.joined_duration()),
        )?;

        match &self.status {
            UserStatus::Active => Ok(()),
            UserStatus::Away { reason, since } => {
                let now = Utc::now();
                let secs = now.signed_duration_since(since).num_seconds() as u64;
                write!(
                    f,
                    "{} > away ({} ago) {}",
                    utils::NEWLINE,
                    humantime::format_duration(Duration::from_secs(secs)),
                    reason
                )
            }
        }
    }
}
