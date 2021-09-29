## Usage

Zip the contents of the `notes` directory into a .zip file and import it into Trilium Notes (right-click into the tree sidebar -> Import).

Acquire a bot token from Telegram and save it in the environment variable `TELEGRAM_BOT_TOKEN`.  
Set `TRILIUM_HOST` to `http://IP:port` (or `https://domain:port`) of your sync server.  
Set `TRILIUM_USER` and `TRILIUM_PASSWORD`.  
Set `TELEGRAM_USER_ID` to your own Telegram User ID.

Then simply run the program: `cargo run --release`.
