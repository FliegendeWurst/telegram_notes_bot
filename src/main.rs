use chrono::prelude::*;
use futures_util::stream::StreamExt;
use log::debug;
use mime::Mime;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_derive::Deserialize;
use serde_json::json;
use telegram_bot::{MessageId, types::{EditMessageText, InlineKeyboardButton, InlineKeyboardMarkup, SendMessage}, Update, UpdateKind, MessageKind, CanReplySendMessage, GetFile};
use telegram_bot::types::refs::ToMessageId;
use tokio::task;

use std::time::Duration;

use telegram_notes_bot::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
	task::spawn_blocking(|| {
		env_logger::init();
		Lazy::force(&OWNER);
		Lazy::force(&API);
		Lazy::force(&TRILIUM_TOKEN);
	}).await.unwrap();
	println!("Init done!");

	task::spawn(task_alerts());
	task::spawn(event_alerts());

	let mut context = Context::default();

	let mut stream = API.stream();
	while let Some(update) = stream.next().await {
		if update.is_err() {
			println!("Telegram error: {:?}", update.err().unwrap());
			continue;
		}

		if let Err(e) = process_one(update.unwrap(), &mut context).await {
			println!("Error: {}", e);
		}
	}
	Ok(())
}

struct Context {
	reminder_msg: MessageId,
	reminder_text: String,
	reminder_start: DateTime<Local>,
	reminder_time: chrono::Duration,
}

impl Default for Context {
	fn default() -> Self {
		Context {
			reminder_msg: MessageId::new(1),
			reminder_text: String::new(),
			reminder_start: Local::now(),
			reminder_time: chrono::Duration::minutes(0),
		}
	}
}

async fn process_one(update: Update, context: &mut Context) -> Result<(), Error> {
	let reminder_msg = &mut context.reminder_msg;
	let reminder_text = &mut context.reminder_text;
	let reminder_start = &mut context.reminder_start;
	let reminder_time = &mut context.reminder_time;

	if let UpdateKind::Message(message) = update.kind {
		let now = Local::now();
		println!("[{}-{:02}-{:02} {:02}:{:02}] Receiving msg {:?}", now.year(), now.month(), now.day(), now.hour(), now.minute(), message);
		if message.from.id != *OWNER {
			println!("ignoring, not sent by authorized user");
			return Ok(());
		}

		if let MessageKind::Text { ref data, .. } = message.kind {
			if data == "/next" {
				command_next().await?;
				return Ok(());
			} else if data == "/remindme" {
				let mut msg = SendMessage::new(*OWNER, "in 0m: new reminder");
				msg.reply_markup(get_keyboard());
				*reminder_msg = API.send(msg).await?.to_message_id();
				*reminder_text = "new reminder".to_owned();
				*reminder_time = chrono::Duration::minutes(0);
				*reminder_start = Local::now();
				return Ok(());
			} else if !reminder_text.is_empty() {
				if data.starts_with("time ") && data.len() > 5 {
					let time = parse_time(&data[5..]);
					match time {
						Ok(time) => {
							*reminder_start = time;
							send_message(format!("got time {}", reminder_start.format("%Y-%m-%d %H:%M"))).await
						},
						Err(e) => send_message(format!("{:?}", e)).await,
					}?;
					return Ok(());
				} else {
					*reminder_text = data.to_owned();
					let mut edit = EditMessageText::new(*OWNER, *reminder_msg, format!("in {}: {}", format_time(*reminder_time), reminder_text));
					edit.reply_markup(get_keyboard());
					API.send(edit).await?;
					return Ok(());
				}
			}
			let is_url = false; //Url::parse(&data).is_ok(); // TODO: read this data from the Telegram json data (utf16 idxes..)
			let formatted_text = if is_url {
				format!("<ul><li><a href=\"{}\">{}</a></li></ul>", data, data)
			} else {
				format!("<ul><li>{}</li></ul>", data)
			};
			let title = format!("{} found at {:02}:{:02}", if is_url { "URL" } else { "content" }, now.hour(), now.minute());
			create_text_note(&CLIENT, &*TRILIUM_TOKEN,
				&title,
				&formatted_text
			).await?;

			// answer message
			if is_url {
				API.send(message.text_reply("URL saved :-)")).await?;
			} else {
				API.send(message.text_reply("Text saved :-)")).await?;
			}
		} else if let MessageKind::Document { ref data, ref caption, .. } = message.kind {
			let document = data;
			let get_file = GetFile::new(&document);
			let file = API.send(get_file).await?;
			let url = file.get_url(&TELEGRAM_BOT_TOKEN).ok_or_else(|| error("url is none"))?;
			let data = CLIENT.get(&url).send().await?.bytes().await?;
			let mime: Mime = document.mime_type.as_ref().ok_or_else(|| error("no mime type"))?.parse()?;
			match (mime.type_(), mime.subtype()) {
				(mime::TEXT, x) if x == "calendar" => {
					let text = String::from_utf8_lossy(&data);
					let text = text.replace("\n<", "<"); // newlines in HTML values
					//send_message(&text).await?;
					let calendar = ical_parsing::parse_calendar(&text)?;
					//send_message(format!("{:?}", calendar)).await?;
					if calendar.events.len() != 1 {
						return Ok(());
					}
					if CLIENT.get(&trilium_url("/custom/new_event")).form(&json!({
						"uid": calendar.events[0].uid,
						"name": calendar.events[0].summary,
						"summary": calendar.events[0].description.replace("\\n", "\n"),
						"summaryHtml": calendar.events[0].description_html.as_deref().map(|x| x.replace("\\n", "\n")).unwrap_or_default(),
						"fileName": document.file_name,
						"fileData": text,
						"location": calendar.events[0].location,
						"startTime": calendar.events[0].start.format("%Y-%m-%dT%H:%M:%S").to_string(),
						"endTime": calendar.events[0].end.format("%Y-%m-%dT%H:%M:%S").to_string(),
					})).send().await?.status().is_success() {
						send_message("Event saved :-)").await?;
					} else {
						send_message("error saving event").await?;
					}
				},
				_ => {
					send_message(format!("Document {:?} {:?} {:?} {:?}", caption, document.file_id, document.file_name, document.mime_type)).await?;
				}
			}
		}
	} else if let UpdateKind::CallbackQuery(cb) = update.kind {
		match &*cb.data.unwrap_or_default() {
			"10m_cb" => {
				*reminder_time = reminder_time.checked_add(&chrono::Duration::minutes(10)).unwrap();
				let mut edit = EditMessageText::new(*OWNER, *reminder_msg, format!("in {}: {}", format_time(*reminder_time), reminder_text));
				edit.reply_markup(get_keyboard());
				API.send(edit).await?;
			},
			"1h_cb" => {
				*reminder_time = reminder_time.checked_add(&chrono::Duration::hours(1)).unwrap();
				let mut edit = EditMessageText::new(*OWNER, *reminder_msg, format!("in {}: {}", format_time(*reminder_time), reminder_text));
				edit.reply_markup(get_keyboard());
				API.send(edit).await?;
			},
			"1d_cb" => {
				*reminder_time = reminder_time.checked_add(&chrono::Duration::days(1)).unwrap();
				let mut edit = EditMessageText::new(*OWNER, *reminder_msg, format!("in {}: {}", format_time(*reminder_time), reminder_text));
				edit.reply_markup(get_keyboard());
				API.send(edit).await?;
			},
			"1w_cb" => {
				*reminder_time = reminder_time.checked_add(&chrono::Duration::days(7)).unwrap();
				let mut edit = EditMessageText::new(*OWNER, *reminder_msg, format!("in {}: {}", format_time(*reminder_time), reminder_text));
				edit.reply_markup(get_keyboard());
				API.send(edit).await?;
			},
			"save_cb" => {
				let remind_time = *reminder_start + *reminder_time;
				CLIENT.get(&trilium_url("/custom/new_reminder")).form(&json!({
					"time": remind_time.to_rfc3339(),
					"task": *reminder_text
				})).send().await?;
				API.send(SendMessage::new(*OWNER, format!("Reminder scheduled for {} :-)", remind_time.format("%Y-%m-%d %H:%M")))).await?;
				*reminder_text = String::new();
			},
			_ => {}
		}
	} else {
		println!("{:?}", update.kind);
	}
	Ok(())	
}

fn get_keyboard() -> InlineKeyboardMarkup {
	let mut keyboard = InlineKeyboardMarkup::new();
	let key = InlineKeyboardButton::callback("10m", "10m_cb");
	let key2 = InlineKeyboardButton::callback("1h", "1h_cb");
	let key3 = InlineKeyboardButton::callback("1d", "1d_cb");
	let key4 = InlineKeyboardButton::callback("1w", "1w_cb");
	keyboard.add_row(vec![key, key2, key3, key4]);
	let key = InlineKeyboardButton::callback("save", "save_cb");
	keyboard.add_row(vec![key]);
	keyboard
}

async fn create_text_note(client: &Client, trilium_token: &str, title: &str, content: &str) -> Result<(), Error> {
	// creating a note:
	// curl /api/clipper/notes
	//  -H 'Accept: */*' -H 'Accept-Language: en' --compressed -H 'Content-Type: application/json'
	//  -H 'Authorization: icB3xohFDpkVt7YFpbTflUYC8pucmryVGpb1DFpd6ns='
	//  -H 'trilium-local-now-datetime: 2020-05-29 __:__:__.xxx+__:__'
	//  -H 'Origin: moz-extension://13bc3fd7-5cb0-4d48-b368-76e389fd7c5f'
	//  --data '{"title":"line 1","content":"<p>line 2</p><p>line 3</p>","clipType":"note"}'
	let now = Local::now();
	client.post(&trilium_url("/api/clipper/notes"))
		.header("Authorization", trilium_token)
		.header("trilium-local-now-datetime", now.format("%Y-%m-%d %H:%M:%S%.3f%:z").to_string())
		.json(&json!({ "title": title, "content": content, "clipType": "note" }))
		.send().await?;
	Ok(())
}

async fn command_next() -> Result<(), Error> {
	let events = request_event_alerts().await?;
	let tasks = request_task_alerts().await?;
	let mut all: Vec<_> = events.into_iter().map(EventOrTask::Event).chain(tasks.into_iter().map(EventOrTask::Task)).collect();
	all.sort_by_key(|x| x.time());
	let mut printed = 0;
	let now = Local::now();
	let mut buf = "```\n".to_owned();
	for x in all {
		let time = x.time();
		if time < now {
			continue;
		}
		buf += &format!("{} {} {}\n", weekday_to_name(time.weekday()), time.format("%Y-%m-%d %H:%M").to_string(), x.description());
		printed += 1;
		if printed >= 10 {
			break;
		}
	}
	buf += "```\n";
	send_message_markdown(buf).await?;
	Ok(())
}

fn weekday_to_name(wd: Weekday) -> &'static str {
	match wd {
    	Weekday::Mon => "Mo",
    	Weekday::Tue => "Di",
    	Weekday::Wed => "Mi",
    	Weekday::Thu => "Do",
    	Weekday::Fri => "Fr",
    	Weekday::Sat => "Sa",
    	Weekday::Sun => "So",
	}
}

enum EventOrTask {
	Event(UsefulEvent),
	Task(UsefulTask)
}

impl EventOrTask {
	fn time(&self) -> DateTime<Local> {
		match self {
    		EventOrTask::Event(e) => e.todo_time,
    		EventOrTask::Task(t) => t.todo_time,
		}
	}
	fn description(&self) -> &str {
		match self {
    		EventOrTask::Event(e) => &e.event.name,
    		EventOrTask::Task(t) => &t.task.title,
		}
	}
}

// image note:
// curl /api/clipper/clippings -H 'Accept: */*' -H 'Accept-Language: en' --compressed -H 'Content-Type: application/json' -H 'Authorization: icB3xohFDpkVt7YFpbTflUYC8pucmryVGpb1DFpd6ns=' -H 'Origin: moz-extension://13bc3fd7-5cb0-4d48-b368-76e389fd7c5f' --data $'{"title":"trilium/clipper.js at master \xb7 zadam/trilium","content":"<img src=\\"BoCpsLz9je8a01MdGbj4\\">","images":[{"imageId":"BoCpsLz9je8a01MdGbj4","src":"inline.png","dataUrl":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAASkAAAESCAYAAAChJCPsAAAgAElEQV"}]}'

async fn event_alerts() {
	loop {
		let last_min = Local::now().minute();
		if let Err(e) = event_alerts_soon().await {
			println!("error: {}", e);
		}
		while Local::now().minute() == last_min {
			tokio::time::sleep(Duration::from_secs(1)).await;
		}
		tokio::time::sleep(Duration::from_secs(16)).await;
	}
}

async fn request_event_alerts() -> Result<Vec<UsefulEvent>, Error> {
	let text = CLIENT.get(&trilium_url("/custom/event_alerts")).send().await?.text().await?;
	debug!("event_alerts response {}", text);
	let events: Result<Vec<Event>, _> = serde_json::from_str(&text);
	if events.is_err() {
		eprintln!("failed to parse {}", text);
	}
	let events = events.map(|x| x.into_iter().flat_map(|event| {
		let todo_time: DateTime<Local> = TimeZone::from_local_datetime(&Local,
			&NaiveDateTime::parse_from_str(&event.start_time, "%Y-%m-%dT%H:%M:%S").ok()?).unwrap();
		Some(UsefulEvent {
			event,
			todo_time
		})
	}).collect());
	Ok(events?)
}

async fn event_alerts_soon() -> Result<(), Error> {
	let now = Local::now();

	let events = request_event_alerts().await?;	
	debug!("events_alerts: {} objects", events.len());
	for event in events {
		if event.todo_time <= now {
			continue;
		}
		let diff = event.todo_time - now;
		let minutes = diff.num_minutes();
		if minutes == 7 * 24 * 60 || minutes == 48 * 60 || minutes == 24 * 60 || minutes == 60 || minutes == 10 {
			event_alert_notify(&format_time(diff), event.event).await?;
		}
	}
	Ok(())
}

async fn event_alert_notify(time_left: &str, event: Event) -> Result<(), Error> {
	API.send(SendMessage::new(*OWNER, format!("{}: {}", time_left, event.name))).await?;
	Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Event {
	name: String,
	start_time: String,
}

struct UsefulEvent {
	event: Event,
	todo_time: DateTime<Local>,
}

async fn task_alerts() {
	loop {
		let last_min = Local::now().minute();
		if let Err(e) = task_alerts_soon().await {
			println!("error: {}", e);
		}
		while Local::now().minute() == last_min {
			tokio::time::sleep(Duration::from_secs(1)).await;
		}
		tokio::time::sleep(Duration::from_secs(1)).await;
	}
}

async fn request_task_alerts() -> Result<Vec<UsefulTask>, Error> {
	let text = CLIENT.get(&trilium_url("/custom/task_alerts")).send().await?.text().await?;
	debug!("task_alerts response {}", text);
	let tasks: Result<Vec<Task>, _> = serde_json::from_str(&text);
	if tasks.is_err() {
		eprintln!("failed to parse {}", text);
	}
	let tasks = tasks.map(|tasks| tasks.into_iter().flat_map(|task| {
		let mut todo_date = None;
		let mut todo_time = None;
		let mut is_reminder = false;
		for attribute in &task.attributes {
			if attribute.r#type != "label" {
				continue;
			}
			match &*attribute.name {
				"todoDate" => todo_date = Some(attribute.value.as_str().unwrap().to_owned()),
				"todoTime" => todo_time = Some(attribute.value.as_str().unwrap().to_owned()),
				"doneDate" => return None,
				"reminder" => is_reminder = true,
				"canceled" => if attribute.value.as_str().unwrap() == "true" { return None },
				_ => {}
			}
		}
		if todo_date.is_none() {
			return None;
		}
		let todo_date = todo_date.unwrap();
		let parts = todo_date.split('-').collect::<Vec<_>>();
		let (year, month, day) = (parts[0].parse().unwrap(), parts[1].parse().unwrap(), parts[2].parse().unwrap());
		let (hour, minute, second) = if let Some(todo_time) = todo_time {
			let parts = todo_time.split(':').collect::<Vec<_>>();
			(parts.get(0).map(|x| x.parse().unwrap()).unwrap_or(0), parts.get(1).map(|x| x.parse().unwrap()).unwrap_or(0), parts.get(2).map(|x| x.parse().unwrap()).unwrap_or(0))
		} else { (0, 0, 0) };
		let todo_time: DateTime<Local> = TimeZone::from_local_datetime(&Local, &NaiveDate::from_ymd(year, month, day).and_hms(hour, minute, second)).unwrap();
		Some(UsefulTask {
			task,
			todo_time,
			is_reminder
		})
		
	}).collect());
	Ok(tasks?)
}

async fn task_alerts_soon() -> Result<(), Error> {
	let now = Local::now();

	let tasks = request_task_alerts().await?;
	debug!("task_alerts: {} objects", tasks.len());
	for task in tasks {
		if task.todo_time <= now {
			continue;
		}
		let diff = task.todo_time - now;
		let minutes = diff.num_minutes();
		if !task.is_reminder && (minutes == 7 * 24 * 60 || minutes == 48 * 60 || minutes == 24 * 60 || minutes == 60 || minutes == 10) {
			notify_owner(&format_time(diff), task.task).await?;
		} else if task.is_reminder && minutes == 0 {
			notify_owner("⏰", task.task).await?;
		}
	}
	Ok(())
}

fn format_time(diff: chrono::Duration) -> String {
	if diff.num_weeks() > 0 {
		format!("{}w", diff.num_weeks())
	} else if diff.num_days() > 0 {
		format!("{}d", diff.num_days())
	} else if diff.num_hours() > 0 {
		if diff.num_minutes() % 60 != 0 {
			format!("{}h{:02}m", diff.num_hours(), diff.num_minutes() % 60)
		} else {
			format!("{}h", diff.num_hours())
		}
	} else {
		format!("{}m", diff.num_minutes())
	}
}

async fn notify_owner(time_left: &str, task: Task) -> Result<(), Error> {
	send_message(format!("{}: {}", time_left, task.title)).await
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct Task {
	attributes: Vec<Attribute>,
	//contentLength: usize,
	dateCreated: DateTime<FixedOffset>,
	dateModified: DateTime<FixedOffset>,
	deleteId: Option<serde_json::Value>,
	// hash: String, // removed in 0.46
	isContentAvailable: bool,
	isDeleted: bool,
	// isErased: i64, // removed in 0.46
	isProtected: bool,
	mime: String,
	noteId: String,
	title: String,
	r#type: String,
	// utcDateCreated: DateTime<Utc>, // removed in 0.46
	utcDateModified: DateTime<Utc>,
}

struct UsefulTask {
	task: Task,
	todo_time: DateTime<Local>,
	is_reminder: bool
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct Attribute {
	attributeId: String,
	noteId: String,
	r#type: String,
	name: String,
	value: serde_json::Value,
	position: usize,
	// utcDateCreated: DateTime<Utc>, // removed in 0.46
	utcDateModified: DateTime<Utc>,
	isDeleted: bool,
	deleteId: Option<serde_json::Value>,
	// hash: String, // removed in 0.46
	isInheritable: bool,
	//isOwned: bool, // removed in 0.42.2
}
