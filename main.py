from dotenv import load_dotenv

load_dotenv(".env")

import asyncio
import io
import os
import sqlite3
import traceback

import discord
import semchunk

from vllm_infer_interface import Request, Response, TextFile, Thread, Message, UserMessage, AssistantMessage


guilds = set()

with open("guilds.txt", "r") as f:
    for line in f.readlines():
        if line.startswith("#"):
            continue
        guilds.add(int(line))


db_conn = sqlite3.connect("ruyi.sqlite")
db_cursor = db_conn.cursor()

db_cursor.execute("CREATE TABLE IF NOT EXISTS seen_messages (message_id INTEGER PRIMARY KEY) STRICT")

chunker = semchunk.chunkerify(lambda text: len(text), 2000)

def is_text_file(filename: str) -> bool:
    return filename.endswith(".c") or \
        filename.endswith(".cpp") or \
        filename.endswith(".h") or \
        filename.endswith(".py") or \
        filename.endswith(".txt") or \
        filename.endswith(".md") or \
        filename.endswith(".html") or \
        filename.endswith(".css") or \
        filename.endswith(".js") or \
        filename.endswith(".ts") or \
        filename.endswith(".rs")


class MyClient(discord.Client):
    async def exit(self):
        if "NO_EXIT" in os.environ:
            print("NO_EXIT environment var is present. Not exiting.")
        else:
            await self.close()
            
    async def on_ready(self):
        print(f'Logged on as {self.user} for guilds: {guilds}')

        try:
            await self.task()
            # print("DONE")
        except Exception:
            traceback.print_exc()
        
        # if "NO_EXIT" in os.environ:
        #     print("NO_EXIT environment var is present. Not exiting.")
        # else:
        #     await self.close()

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
            last_messages = []

            for thread in await guild.active_threads():
                messages, last_message = await self.per_thread(thread)
                if len(messages) == 0:
                    continue
                request.threads.append(
                    Thread(messages=messages)
                )
                last_messages.append(last_message)
            
            if len(request.threads) == 0:
                await self.exit()
                return
            
            asyncio.create_task(self.vllm_infer(request, last_messages))
                
    async def vllm_infer(self, request: Request, last_messages: list[discord.Message]):
        process = await asyncio.create_subprocess_exec(
            "python",
            "vllm_infer.py",
            stdin=asyncio.subprocess.PIPE
        )
        await process.communicate(request.model_dump_json().encode("utf-8"))
        stdout_str = open("/tmp/vllm_infer.json", "r").read()
        response = Response.model_validate_json(stdout_str)

        for response, last_message in zip(response.thread_responses, last_messages):
            chunks = chunker(response.message)
            files = [discord.File(io.BytesIO(file.content.encode("utf-8")), file.filename) for file in response.text_files]

            for i, tmp_chunk in enumerate(chunks):
                # For type checker
                chunk: str = tmp_chunk # type: ignore

                if i == 0:
                    if i == len(chunks) - 1:
                        await last_message.reply(chunk, files=files)
                    else:
                        await last_message.reply(chunk)
                elif i == len(chunks) - 1:
                    await last_message.channel.send(chunk, files=files)
                else:
                    await last_message.channel.send(chunk)
        
        db_conn.commit()
        await self.exit()

    async def per_thread(self, thread: discord.Thread) -> tuple[list[Message], Message]:
        messages: list[discord.Message] = []
        last_message = None
        i = -1
        unseen = False
        entered_old = False

        async for message in thread.history(oldest_first=False):
            i += 1
            if message.author == self.user or message.is_system() or i + 1 >= thread.message_count or self.user not in message.mentions:
                continue
            if last_message is None:
                last_message = message

            messages.append(message)

            if not entered_old:
                try:
                    db_cursor.execute(f"INSERT INTO seen_messages (message_id) VALUES ({message.id})")
                    unseen = True
                except sqlite3.IntegrityError:
                    entered_old = True

        if not unseen:
            return ([], last_message) # type: ignore

        # After reversal, the messages will be oldest to newest (chronological)
        messages.reverse()

        parsed_messages = []
        for msg in messages:
            if msg.author == self.user:
                parsed = AssistantMessage(message=msg.content, text_files=[])

                for attachment in msg.attachments:
                    if is_text_file(attachment.filename):
                        content = (await attachment.read()).decode("utf-8")
                        parsed.text_files.append(TextFile(content=content, filename=attachment.filename))

                parsed_messages.append(parsed)
                continue
        
            parsed = UserMessage(author=msg.author.display_name, message=msg.content, image_urls=[], video_urls=[], text_files=[])

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

                elif is_text_file(attachment.filename):
                    content = (await attachment.read()).decode("utf-8")
                    parsed.text_files.append(TextFile(content=content, filename=attachment.filename))

            parsed_messages.append(parsed)

        return (parsed_messages, last_message) # type: ignore


intents = discord.Intents.default()
intents.message_content = True

client = MyClient(intents=intents)
client.run(os.environ["DISCORD_TOKEN"])
