use chrono::{Duration, NaiveDateTime, NaiveDate};
use ical::parser::ical::component::IcalEvent;
use ical::parser::ical::IcalParser;
use thiserror::Error;

#[derive(Debug)]
pub struct Calendar {
	pub name: String,
	pub events: Vec<Event>,
}

#[derive(Debug)]
pub struct Event {
	pub uid: String,
	pub summary: String,
	pub description: String,
	/// X-ALT-DESC;FMTTYPE=text/html
	pub description_html: Option<String>,
	pub start: NaiveDateTime,
	pub end: NaiveDateTime,
	pub duration: Option<Duration>,
	pub location: String,
}

pub fn parse_calendar(data: &str) -> Result<Calendar, Error> {
	let cal = IcalParser::new(data.as_bytes()).next().ok_or(Error::Nothing)??;
	let mut name = None;
	let mut events = Vec::new();
	for prop in cal.properties {
		match prop.name.as_ref() {
			"NAME" => name = Some(prop.value.unwrap_or_default()),
			_ => {}
		}
	}
	for event in cal.events {
		events.push(process_event(event)?);
	}
	let name = name.unwrap_or_default();
	Ok(Calendar {
		name, events
	})
}

fn process_event(event: IcalEvent) -> Result<Event, Error> {
	let mut uid = None;
	let mut summary = None;
	let mut description = None;
	let mut description_html = None;
	let mut start = None;
	let mut end = None;
	let mut duration = None;
	let mut location = None;
	for prop in event.properties {
		let value = prop.value.unwrap_or_default();
		match prop.name.as_ref() {
			"UID" => uid = Some(value),
			"SUMMARY" => summary = Some(value),
			"LOCATION" => location = Some(value),
			"DESCRIPTION" => description = Some(value),
			"STATUS" => { /* TODO: status */ },
			"DTSTART" => start = Some(process_dt(&value)?),
			"DTEND" => end = Some(process_dt(&value)?),
			"DURATION" => duration = Some(process_duration(&value)?),
			"RRULE" => { /* TODO: periodic */ },
			"X-ALT-DESC" => {
				if prop.params.as_ref()
					.map(|x| x.iter()
						.any(|(key, values)| key == "FMTTYPE" && values.first().map(|x| &**x) == Some("text/html"))
					).unwrap_or(false) {
					description_html = Some(value);
				}
			}
			_ => (),
		};
	}
	// TODO: don't put defaults here
	let start = start.ok_or(Error::Data("no dtstart"))?;
	let end = end.ok_or(Error::Data("no dtend"))?;
	Ok(Event {
		uid: uid.unwrap_or_default(),
		summary: summary.unwrap_or_default(),
		description: description.unwrap_or_default(),
		description_html,
	    start: start,
		end: end,
		duration,
	    location: location.unwrap_or_default(), 
	})
}

fn process_dt(value: &str) -> Result<NaiveDateTime, Error> {
	// 20200626T140000
	if value.len() != 15 {
		return Err(Error::Data("invalid dt length"));
	}
	// TODO: error handling
	let year = value[0..4].parse()?;
	let month = value[4..6].parse()?;
	let day = value[6..8].parse()?;
	let hour = value[9..11].parse()?;
	let minute = value[11..13].parse()?;
	let second = value[13..15].parse()?;

	Ok(NaiveDate::from_ymd(year, month, day).and_hms(hour, minute, second))
}

fn process_duration(_value: &str) -> Result<Duration, Error> {
	// TODO
	return Err(Error::Data("duration parsing not implemented"));
}

#[derive(Error, Debug)]
pub enum Error {
	#[error("parsing error: {0}")]
	Ical(ical::parser::ParserError),
	#[error("data error: {0}")]
	Data(&'static str),
	#[error("parse error: {0}")]
	IntegerParsing(#[from] std::num::ParseIntError),
	#[error("no calendar found")]
	Nothing
}

impl From<ical::parser::ParserError> for Error {
	fn from(x: ical::parser::ParserError) -> Self {
		Error::Ical(x)
	}
}
