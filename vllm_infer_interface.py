from typing import Literal
from pydantic import BaseModel


class TextFile(BaseModel):
    filename: str
    content: str


class UserMessage(BaseModel):
    msg_type: Literal["user"] = "user"
    author: str
    message: str
    image_urls: list[str]
    video_urls: list[str]
    text_files: list[TextFile]


class AssistantMessage(BaseModel):
    msg_type: Literal["assistant"] = "assistant"
    message: str
    text_files: list[TextFile]


Message = UserMessage | AssistantMessage


class Thread(BaseModel):
    messages: list[Message]


class Request(BaseModel):
    threads: list[Thread]


class ThreadResponse(BaseModel):
    message: str
    text_files: list[TextFile]
    image_prompts: list[str]


class Response(BaseModel):
    thread_responses: list[ThreadResponse]


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
