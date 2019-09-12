use chrono::prelude::*;
use futures::StreamExt;
use maplit::hashmap;
use telegram_bot::*;
use url::Url;

use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Loading configuration..");
	let owner = UserId::new(env::var("TELEGRAM_USER_ID").expect("TELEGRAM_USER_ID not set").parse().expect("TELEGRAM_USER_ID not numeric"));

	println!("Loading passwords..");
	let telegram_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
	let trilium_user = env::var("TRILIUM_USER").expect("TRILIUM_USER not set");
	let trilium_password = env::var("TRILIUM_PASSWORD").expect("TRILIUM_PASSWORD not set");

	println!("Initializing Telegram API..");
	let api = Api::new(telegram_token);

	println!("Initializing HTTPS client..");
	let client = reqwest::Client::new();
	println!("Initializing Trilium API..");
	// Trilium login:
	// curl 'http://localhost:9001/api/login/token' -H 'User-Agent: Mozilla/5.0 ..' -H 'Accept: application/json' -H 'Accept-Language: en' --compressed -H 'Content-Type: application/json' -H 'Origin: moz-extension://13bc3fd7-5cb0-4d48-b368-76e389fd7c5f' -H 'DNT: 1' -H 'Connection: keep-alive' --data '{"username":"username","password":"insert_password_here"}'
	// -> {"token":"icB3xohFDpkVt7YFpbTflUYC8pucmryVGpb1DFpd6ns="}
	let resp: HashMap<String, String> = client.post("http://localhost:9001/api/login/token")
		.json(&hashmap!{ "username" => &trilium_user, "password" => &trilium_password })
		.send().await.unwrap().json().await.unwrap();
	let trilium_token = &resp["token"];

	println!("Init done!");

	// Fetch new updates via long poll method
	let mut stream = api.stream();
	while let Some(update) = stream.next().await {
		// If the received update contains a new message...
		let update = update?;
		if let UpdateKind::Message(message) = update.kind {
			let now = Local::now();

			println!("[{}-{:02}-{:02} {:02}:{:02}] Receiving msg {:?}", now.year(), now.month(), now.day(), now.hour(), now.minute(), message);
			if message.from.id != owner {
				// don't handle message
				continue;
			}
			if let MessageKind::Text { ref data, .. } = message.kind {
				let is_url = Url::parse(&data).is_ok();
				let formatted_text = if is_url {
					format!("<ul><li><a href=\"{}\">{}</a></li></ul>", data, data)
				} else {
					format!("<ul><li>{}</li></ul>", data)
				};
				let title = format!("{} found at {}:{}", if is_url { "URL" } else { "content" }, now.hour(), now.minute());
				create_text_note(&client, trilium_token,
					&title,
					&formatted_text
				).await?;

				// answer message
				if is_url {
					api.send(message.text_reply("URL saved :-)")).await?;
				} else {
					api.send(message.text_reply("Text saved :-)")).await?;
				}
			}
		}
	}
	Ok(())
}

async fn create_text_note(client: &reqwest::Client, trilium_token: &str, title: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
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