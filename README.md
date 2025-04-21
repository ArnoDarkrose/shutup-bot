This telegram bot can be used to prevent some people from spamming memes in chats.

Just set your token for telegram api in the TELOXIDE_TOKEN environment variable, pass username of the person you want to stop from spamming in
the --shutup-target  and the maximum amount of memes in the --meme-limit key (optional, defaults to 2) and start the server by simply running the binary.

This bot deletes all the user's forward messages if their number exceeds MEME_LIMIT for the day and sends a demotivator to the user in DM. 
