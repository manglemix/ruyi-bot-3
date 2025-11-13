from dotenv import load_dotenv

load_dotenv(".env")

import os
import sqlite3
import traceback

import discord

from vllm_infer_interface import Request, TextFile, Thread, Message


guilds = set()

with open("guilds.txt", "r") as f:
    for line in f.readlines():
        if line.startswith("#"):
            continue
        guilds.add(int(line))


db_conn = sqlite3.connect("ruyi.sqlite")
db_cursor = db_conn.cursor()

db_cursor.execute("CREATE TABLE IF NOT EXISTS seen_messages (message_id INTEGER PRIMARY KEY) STRICT")


class MyClient(discord.Client):
    async def on_ready(self):
        print(f'Logged on as {self.user} for guilds: {guilds}')

        try:
            await self.task()
            print("DONE")
        except Exception:
            traceback.print_exc()
        
        if "NO_EXIT" in os.environ:
            print("NO_EXIT environment var is present. Not exiting.")
        else:
            await self.close()

    async def on_message(self, message: discord.Message):
        if "MSG_TRACK" in os.environ:
            if message.channel.guild is None:
                if type(message.channel) == discord.DMChannel:
                    # This is the DM channel ID, not the user id since for some reason that is not always accessible
                    print(f'Message "{message.content}" within User DMs {message.channel.id}')
                else:
                    print(f'Message within {message.channel}')
            else:
                print(f'Message within guild {message.channel.guild.id}')
    
    async def task(self):
        for guild_id in guilds:
            guild = self.get_guild(guild_id)
            if guild is None:
                print(f"{guild_id} does not exist")
                continue
            
            request = Request(threads=[])

            for thread in await guild.active_threads():
                messages = await self.per_thread(thread)
                if len(messages) == 0:
                    continue
                request.threads.append(
                    Thread(messages=messages)
                )

                
    
    async def per_thread(self, thread: discord.Thread) -> list[Message]:
        messages: list[discord.Message] = []
        i = -1
        unseen = False

        async for message in thread.history(oldest_first=False):
            i += 1
            if message.author == self.user or message.is_system() or i + 1 >= thread.message_count or self.user not in message.mentions:
                continue

            messages.append(message)

            try:
                db_cursor.execute(f"INSERT INTO seen_messages (message_id) VALUES ({message.id})")
                unseen = True
            except sqlite3.IntegrityError:
                pass

        if not unseen:
            return []

        # After reversal, the messages will be oldest to newest (chronological)
        messages.reverse()

        parsed_messages = []
        for msg in messages:
            parsed = Message(author=msg.author.display_name, message=msg.content, image_urls=[], video_urls=[], text_files=[])

            for attachment in msg.attachments:
                if attachment.filename.endswith(".png") or \
                        attachment.filename.endswith(".jpg") or \
                        attachment.filename.endswith(".jpeg") or \
                        attachment.filename.endswith(".webp"):
                    parsed.image_urls.append(attachment.proxy_url)

                elif attachment.filename.endswith(".mp4") or \
                        attachment.filename.endswith(".mov") or \
                        attachment.filename.endswith(".mkv"):
                    parsed.video_urls.append(attachment.proxy_url)

                elif attachment.filename.endswith(".c") or \
                        attachment.filename.endswith(".cpp") or \
                        attachment.filename.endswith(".h") or \
                        attachment.filename.endswith(".py") or \
                        attachment.filename.endswith(".txt") or \
                        attachment.filename.endswith(".md") or \
                        attachment.filename.endswith(".html") or \
                        attachment.filename.endswith(".css") or \
                        attachment.filename.endswith(".js") or \
                        attachment.filename.endswith(".ts") or \
                        attachment.filename.endswith(".rs"):
                    content = (await attachment.read()).decode("utf-8")
                    parsed.text_files.append(TextFile(content=content, filename=attachment.filename))

            parsed_messages.append(parsed)

        return parsed_messages


intents = discord.Intents.default()
intents.message_content = True

client = MyClient(intents=intents)
client.run(os.environ["DISCORD_TOKEN"])
