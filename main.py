from dotenv import load_dotenv

load_dotenv(".env")

import os
import sqlite3
import traceback

import discord


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

    async def on_message(self, message):
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

            for thread in await guild.active_threads():
                i = -1
                async for message in thread.history(oldest_first=False):
                    i += 1
                    if message.author == self.user or message.is_system() or i + 1 >= thread.message_count:
                        continue
                    try:
                        db_cursor.execute(f"INSERT INTO seen_messages (message_id) VALUES ({message.id})")
                    except sqlite3.IntegrityError:
                        db_conn.rollback()
                        continue
                    await message.reply("hello")
                    db_conn.commit()


intents = discord.Intents.default()
intents.message_content = True

client = MyClient(intents=intents)
client.run(os.environ["DISCORD_TOKEN"])
