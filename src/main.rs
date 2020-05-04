use chrono::prelude::*;
use futures_util::stream::StreamExt;
use maplit::hashmap;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_derive::Deserialize;
use serde_json::json;
use telegram_bot::*;
use telegram_bot::types::{EditMessageText, InlineKeyboardButton, InlineKeyboardMarkup, SendMessage};
use thiserror::Error;
use tokio::task;
use url::Url;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

use std::collections::HashMap;
use std::convert::Infallible;
use std::env;
use std::sync::Arc;
use std::time::Duration;

static API: Lazy<Arc<Api>> = Lazy::new(|| {
	let telegram_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
	println!("Initializing Telegram API..");
	Arc::new(Api::new(telegram_token))
});

static OWNER: Lazy<UserId> = Lazy::new(|| {
	println!("Loading configuration..");
	UserId::new(env::var("TELEGRAM_USER_ID").expect("TELEGRAM_USER_ID not set").parse().expect("TELEGRAM_USER_ID not numeric"))
});

static CLIENT: Lazy<Client> = Lazy::new(Client::new);

#[tokio::main]
async fn main() -> Result<(), Error> {
	&*OWNER;

	println!("Loading passwords..");
	let trilium_user = env::var("TRILIUM_USER").expect("TRILIUM_USER not set");
	let trilium_password = env::var("TRILIUM_PASSWORD").expect("TRILIUM_PASSWORD not set");

	&*API;

	println!("Initializing Trilium API..");
	// Trilium login:
	// curl 'http://localhost:9001/api/login/token' -H 'User-Agent: Mozilla/5.0 ..' -H 'Accept: application/json' -H 'Accept-Language: en' --compressed -H 'Content-Type: application/json' -H 'Origin: moz-extension://13bc3fd7-5cb0-4d48-b368-76e389fd7c5f' -H 'DNT: 1' -H 'Connection: keep-alive' --data '{"username":"username","password":"insert_password_here"}'
	// -> {"token":"icB3xohFDpkVt7YFpbTflUYC8pucmryVGpb1DFpd6ns="}
	let resp: HashMap<String, String> = CLIENT.post("http://localhost:9001/api/login/token")
		.json(&hashmap!{ "username" => &trilium_user, "password" => &trilium_password })
		.send().await?.json().await?;
	let trilium_token = &resp["token"];
	println!("Acquired token: {}", trilium_token);

	println!("Init done!");

	task::spawn(async move {
		start_server().await;
	});

	task::spawn(async move {
		start_polling().await;
	});

	let mut reminder_msg = MessageId::new(1);
	let mut reminder_text = String::new();
	let mut reminder_start = Local::now();
	let mut reminder_time = chrono::Duration::minutes(0);

	// Fetch new updates via long poll method
	let mut stream = API.stream();
	while let Some(update) = stream.next().await {
		// If the received update contains a new message...
		if update.is_err() {
			println!("Error: {:?}", update.err().unwrap());
			continue;
		}
		let update = update.unwrap();
		if let UpdateKind::Message(message) = update.kind {
			let now = Local::now();

			println!("[{}-{:02}-{:02} {:02}:{:02}] Receiving msg {:?}", now.year(), now.month(), now.day(), now.hour(), now.minute(), message);
			if message.from.id != *OWNER {
				// don't handle message
				continue;
			}
			if let MessageKind::Text { ref data, .. } = message.kind {
				if data == "/remindme" {
					let mut msg = SendMessage::new(*OWNER, "in 0m: new reminder");
					msg.reply_markup(get_keyboard());
					reminder_msg = API.send(msg).await?.to_message_id();
					reminder_text = "new reminder".to_owned();
					reminder_time = chrono::Duration::minutes(0);
					reminder_start = Local::now();
					continue;
				} else if !reminder_text.is_empty() {
					reminder_text = data.to_owned();
					let mut edit = EditMessageText::new(*OWNER, reminder_msg, format!("in {}: {}", format_time(reminder_time), reminder_text));
					edit.reply_markup(get_keyboard());
					API.send(edit).await?;
					continue;
				}
				let is_url = false; //Url::parse(&data).is_ok(); // TODO: read this data from the Telegram json data (utf16 idxes..)
				let formatted_text = if is_url {
					format!("<ul><li><a href=\"{}\">{}</a></li></ul>", data, data)
				} else {
					format!("<ul><li>{}</li></ul>", data)
				};
				let title = format!("{} found at {:02}:{:02}", if is_url { "URL" } else { "content" }, now.hour(), now.minute());
				create_text_note(&CLIENT, trilium_token,
					&title,
					&formatted_text
				).await?;

				// answer message
				if is_url {
					API.send(message.text_reply("URL saved :-)")).await?;
				} else {
					API.send(message.text_reply("Text saved :-)")).await?;
				}
			}
		} else if let UpdateKind::CallbackQuery(cb) = update.kind {
			match &*cb.data.unwrap_or_default() {
				"10m_cb" => {
					reminder_time = reminder_time.checked_add(&chrono::Duration::minutes(10)).unwrap();
					let mut edit = EditMessageText::new(*OWNER, reminder_msg, format!("in {}: {}", format_time(reminder_time), reminder_text));
					edit.reply_markup(get_keyboard());
					API.send(edit).await?;
				},
				"1h_cb" => {
					reminder_time = reminder_time.checked_add(&chrono::Duration::hours(1)).unwrap();
					let mut edit = EditMessageText::new(*OWNER, reminder_msg, format!("in {}: {}", format_time(reminder_time), reminder_text));
					edit.reply_markup(get_keyboard());
					API.send(edit).await?;
				},
				"1d_cb" => {
					reminder_time = reminder_time.checked_add(&chrono::Duration::days(1)).unwrap();
					let mut edit = EditMessageText::new(*OWNER, reminder_msg, format!("in {}: {}", format_time(reminder_time), reminder_text));
					edit.reply_markup(get_keyboard());
					API.send(edit).await?;
				},
				"1w_cb" => {
					reminder_time = reminder_time.checked_add(&chrono::Duration::days(7)).unwrap();
					let mut edit = EditMessageText::new(*OWNER, reminder_msg, format!("in {}: {}", format_time(reminder_time), reminder_text));
					edit.reply_markup(get_keyboard());
					API.send(edit).await?;
				},
				"save_cb" => {
					let remind_time = reminder_start + reminder_time;
					CLIENT.get("http://localhost:9001/custom/new_reminder").form(&json!({
						"time": remind_time.to_rfc3339(),
						"task": reminder_text
					})).send().await?.text().await?;
					API.send(SendMessage::new(*OWNER, "Reminder saved :-)")).await?;
					reminder_text = String::new();
				},
				_ => {}
			}
		} else {
			println!("{:?}", update.kind);
		}
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
	// curl 'http://localhost:9001/api/clipper/notes'
	//  -H 'Accept: */*' -H 'Accept-Language: en' --compressed -H 'Content-Type: application/json'
	//  -H 'Authorization: icB3xohFDpkVt7YFpbTflUYC8pucmryVGpb1DFpd6ns='
	//  -H 'Origin: moz-extension://13bc3fd7-5cb0-4d48-b368-76e389fd7c5f'
	//  --data '{"title":"line 1","content":"<p>line 2</p><p>line 3</p>","clipType":"note"}'

	client.post("http://localhost:9001/api/clipper/notes")
		.header("Authorization", trilium_token)
		.json(&hashmap!{ "title" => title, "content" => content, "clipType" => "note" })
		.send().await?;
	Ok(())
}

// image note:
// curl 'http://localhost:9001/api/clipper/clippings' -H 'Accept: */*' -H 'Accept-Language: en' --compressed -H 'Content-Type: application/json' -H 'Authorization: icB3xohFDpkVt7YFpbTflUYC8pucmryVGpb1DFpd6ns=' -H 'Origin: moz-extension://13bc3fd7-5cb0-4d48-b368-76e389fd7c5f' --data $'{"title":"trilium/clipper.js at master \xb7 zadam/trilium","content":"<img src=\\"BoCpsLz9je8a01MdGbj4\\">","images":[{"imageId":"BoCpsLz9je8a01MdGbj4","src":"inline.png","dataUrl":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAASkAAAESCAYAAAChJCPsAAAgAElEQV"}]}'

async fn start_polling() {
	loop {
		let last_min = Local::now().minute();
		if let Err(e) = one_req().await {
			println!("error: {}", e);
		}
		while Local::now().minute() == last_min {
			tokio::time::delay_for(Duration::from_secs(1)).await;
		}
		tokio::time::delay_for(Duration::from_secs(1)).await;
	}
}

async fn one_req() -> Result<(), Error> {
	let now = Local::now();
	//println!("{}", CLIENT.get("http://localhost:9001/custom/task_alerts").send().await?.text().await?);

	let tasks: Vec<Task> = CLIENT.get("http://localhost:9001/custom/task_alerts").send().await?.json().await?;
	//println!("{:?}", tasks);
	'task: for task in tasks {
		let mut todo_date = None;
		let mut todo_time = None;
		let mut is_reminder = false;
		//println!("considering {:?} with {:?}", task.title, task.attributes);
		for attribute in &task.attributes {
			if attribute.r#type != "label" {
				continue;
			}
			match &*attribute.name {
				"todoDate" => todo_date = Some(attribute.value.as_str().unwrap().to_owned()),
				"todoTime" => todo_time = Some(attribute.value.as_str().unwrap().to_owned()),
				"doneDate" => continue 'task,
				"reminder" => is_reminder = true,
				_ => {}
			}
		}
		if todo_date.is_none() {
			continue;
		}
		let todo_date = todo_date.unwrap();
		let parts = todo_date.split('-').collect::<Vec<_>>();
		let (year, month, day) = (parts[0].parse().unwrap(), parts[1].parse().unwrap(), parts[2].parse().unwrap());
		let (hour, minute, second) = if let Some(todo_time) = todo_time {
			let parts = todo_time.split(':').collect::<Vec<_>>();
			(parts.get(0).map(|x| x.parse().unwrap()).unwrap_or(0), parts.get(1).map(|x| x.parse().unwrap()).unwrap_or(0), parts.get(2).map(|x| x.parse().unwrap()).unwrap_or(0))
		} else { (0, 0, 0) };
		let todo_time: DateTime<Local> = TimeZone::from_local_datetime(&Local, &NaiveDate::from_ymd(year, month, day).and_hms(hour, minute, second)).unwrap();
		if todo_time <= now {
			continue;
		}
		let diff = todo_time - now;
		let minutes = diff.num_minutes();
		if !is_reminder && (minutes == 7 * 24 * 60 || minutes == 48 * 60 || minutes == 24 * 60 || minutes == 60 || minutes == 10) {
			notify_owner_impl(&format_time(diff), task).await?;
		} else if is_reminder && minutes == 0 {
			notify_owner_impl("⏰", task).await?;
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

async fn start_server() {
	let heartbeat = warp::get()
		.and(warp::path("heartbeat"))
		.map(|| {
			println!("Trilium heartbeat received");
			"OK"
		});
	let task_alert = warp::post()
		.and(warp::path("task_alert"))
		.and(warp::path::param::<String>())
		.and(warp::body::json())
		.and_then(notify_owner)
		.recover(handle_rejection);
	let routes = heartbeat.or(task_alert);

	warp::serve(routes)
		.run(([127, 0, 0, 1], 19517))
		.await;
}

async fn notify_owner(note_id: String, task: Task) -> Result<impl warp::Reply, Rejection> {
	match notify_owner_impl("..", task).await {
		Err(e) => Err(warp::reject::custom(e)),
		x => x.map_err(|_| unreachable!())
	}
}

async fn notify_owner_impl(time_left: &str, task: Task) -> Result<impl warp::Reply, Error> {
	API.send(SendMessage::new(*OWNER, format!("{}: {}", time_left, task.title))).await?;
	Ok("OK")
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct Task {
	attributes: Vec<Attribute>,
	contentLength: usize,
	dateCreated: DateTime<FixedOffset>,
	dateModified: DateTime<FixedOffset>,
	deleteId: Option<serde_json::Value>,
	hash: String,
	isContentAvailable: bool,
	isDeleted: bool,
	isErased: i64,
	isProtected: bool,
	mime: String,
	noteId: String,
	title: String,
	r#type: String,
	utcDateCreated: DateTime<Utc>,
	utcDateModified: DateTime<Utc>,
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
	utcDateCreated: DateTime<Utc>,
	utcDateModified: DateTime<Utc>,
	isDeleted: bool,
	deleteId: Option<serde_json::Value>,
	hash: String,
	isInheritable: bool,
	isOwned: bool,
}

#[derive(Error, Debug)]
pub enum Error {
	#[error("network error: {0}")]
	Network(#[from] reqwest::Error),
	#[error("telegram error: {0}")]
	Telegram(#[from] telegram_bot::Error),
}

impl warp::reject::Reject for Error {}

// This function receives a `Rejection` and tries to return a custom
// value, otherwise simply passes the rejection along.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
	Ok(warp::reply::with_status(format!("{:?}", err), StatusCode::INTERNAL_SERVER_ERROR))
}
