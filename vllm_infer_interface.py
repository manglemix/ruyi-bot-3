from pydantic import BaseModel


class TextFile(BaseModel):
    filename: str
    content: str


class Message(BaseModel):
    author: str
    message: str
    image_urls: list[str]
    video_urls: list[str]
    text_files: list[TextFile]


class Thread(BaseModel):
    messages: list[Message]


class Request(BaseModel):
    threads: list[Thread]
