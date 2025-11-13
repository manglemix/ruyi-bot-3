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
