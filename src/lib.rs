use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde_json::json;
use telegram_bot::*;
use telegram_bot::types::SendMessage;
use thiserror::Error;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use chrono::{DateTime, Datelike, Timelike, Local};

pub mod ical_parsing;

pub static TELEGRAM_BOT_TOKEN: Lazy<String> = Lazy::new(|| {
	env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set")
});

pub static API: Lazy<Arc<Api>> = Lazy::new(|| {
	println!("Initializing Telegram API..");
	Arc::new(Api::new(&*TELEGRAM_BOT_TOKEN))
});

pub static TRILIUM_HOST: Lazy<String> = Lazy::new(|| {
	env::var("TRILIUM_HOST").expect("TRILIUM_HOST not set")
});

pub fn trilium_url(path: &str) -> String {
	format!("http://{}{}", *TRILIUM_HOST, path)
}

pub static TRILIUM_TOKEN: Lazy<String> = Lazy::new(|| {
	println!("Initializing Trilium API..");
	let trilium_user = env::var("TRILIUM_USER").expect("TRILIUM_USER not set");
	let trilium_password = env::var("TRILIUM_PASSWORD").expect("TRILIUM_PASSWORD not set");
	let client = reqwest::blocking::Client::new();
	// curl /api/login/token -H 'User-Agent: Mozilla/5.0 ..' -H 'Accept: application/json' -H 'Accept-Language: en' --compressed -H 'Content-Type: application/json' -H 'Origin: moz-extension://13bc3fd7-5cb0-4d48-b368-76e389fd7c5f' -H 'DNT: 1' -H 'Connection: keep-alive' --data '{"username":"username","password":"insert_password_here"}'
	// -> {"token":"icB3xohFDpkVt7YFpbTflUYC8pucmryVGpb1DFpd6ns="}
	let resp: HashMap<String, String> = client.post(&trilium_url("/api/login/token"))
		.json(&json!({ "username": &trilium_user, "password": &trilium_password }))
		.send().unwrap().json().unwrap();
	resp["token"].clone()
});

pub static OWNER: Lazy<UserId> = Lazy::new(|| {
	println!("Loading configuration..");
	UserId::new(env::var("TELEGRAM_USER_ID").expect("TELEGRAM_USER_ID not set").parse().expect("TELEGRAM_USER_ID not numeric"))
});

pub static CLIENT: Lazy<Client> = Lazy::new(|| {
	Client::builder().http1_title_case_headers().build().unwrap()
});


#[derive(Error, Debug)]
pub enum Error {
	#[error("network error: {0}")]
	Network(#[from] reqwest::Error),
	#[error("telegram error: {0}")]
	Telegram(#[from] telegram_bot::Error),
	#[error("json parsing error: {0}")]
	Json(#[from] serde_json::Error),
	#[error("mime parsing error: {0}")]
	Mime(#[from] mime::FromStrError),
	#[error("chrono parsing error: {0}")]
	Chrono(#[from] chrono::format::ParseError),
	#[error("integer parsing error: {0}")]
	Integer(#[from] std::num::ParseIntError),
	#[error("ical parsing error: {0}")]
	Ical(#[from] ical_parsing::Error),
	#[error("internal error: {0}")]
	CustomMessage(String),
}

pub fn error<S: Into<String>>(msg: S) -> Error {
	Error::CustomMessage(msg.into())
}

pub async fn send_message<S: Into<String>>(msg: S) -> Result<(), Error> {
	API.send(SendMessage::new(*OWNER, msg.into()).parse_mode(ParseMode::MarkdownV2)).await?;
	Ok(())
}

static DATE_TIME_REGEX: Lazy<Regex> = Lazy::new(|| {
	Regex::new(r"(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})(?:[\sT](?P<hour>\d{2}).(?P<minute>\d{2}))?").unwrap()
});

pub fn parse_time<S: AsRef<str>>(s: S) -> Result<DateTime<Local>, Error> {
	let s = s.as_ref();
	// YYYY-MM-DD format
	let data = DATE_TIME_REGEX.captures(s).ok_or_else(|| error("regex failed"))?;
	let year = data.name("year").unwrap().as_str().parse().unwrap();
	let month = data.name("month").unwrap().as_str().parse().unwrap();
	let day = data.name("day").unwrap().as_str().parse().unwrap();
	let hour = data.name("hour").map(|x| x.as_str().parse().unwrap()).unwrap_or(0);
	let minute = data.name("minute").map(|x| x.as_str().parse().unwrap()).unwrap_or(0);
	// TODO: construct this datetime in a more elegant way
	let dt = Local::now()
		.with_year(year).unwrap()
		.with_month(month).unwrap()
		.with_day(day).unwrap()
		.with_hour(hour).unwrap()
		.with_minute(minute).unwrap()
		.with_second(0).unwrap();
	Ok(dt)
}
