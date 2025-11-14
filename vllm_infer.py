from dataclasses import dataclass
from typing import Any
from vllm import LLM, SamplingParams
import sys
from vllm_infer_interface import Request, Response, ThreadResponse, is_text_file
import json
from datetime import datetime
from playwright.sync_api import sync_playwright
from vllm.assets.image import ImageAsset
import string
import random


def generate_random_id(length=9):
    characters = string.ascii_letters + string.digits
    random_id = "".join(random.choice(characters) for _ in range(length))
    return random_id


# llm = LLM(model="Qwen/Qwen3-VL-30B-A3B-Thinking")
llm = LLM(model="cpatonn/Qwen3-VL-8B-Thinking-AWQ-4bit", gpu_memory_utilization=0.7, max_model_len=15000, allowed_local_media_path="/tmp")
sampling_params = SamplingParams(temperature=0.6, top_p=0.95, top_k=20, min_p=0, max_tokens=None)

tools = json.loads(open("tools.json", "r").read())

system_message = open("system_msg.txt", "r").read()
system_message += f"\nThe current datetime is {datetime.now().isoformat()}"

json_data = sys.stdin.readline().strip()
request = Request.model_validate_json(json_data)
response = Response(thread_responses=[])


@dataclass
class WebSearchResult:
    url: str | None
    text: str | None


def web_search(url: str):
    result = None
    with sync_playwright() as p:
        browser_type = p.firefox
        browser = browser_type.launch()
        page = browser.new_page()
        try:
            page.goto(url, wait_until="load")
        except Exception as e:
            print(e)
            browser.close()
            return {
                "type": "text",
                "text": "Failed to load URL"
            }

        if is_text_file(url) and not url.endswith(".html"):
            result = {
                "type": "text",
                "text": page.content()
            }
        else:
            page.screenshot(path='/tmp/vllm_web_search.png')

            result = {
                "type": "image_url",
                "image_url": {
                    "url": "file:///tmp/vllm_web_search.png"
                }
            }
        browser.close()
    return result


tool_functions = {
    "web_search": web_search
}


for thread in request.threads:
    messages = [
        {
            "role": "system",
            "content": system_message
        }
    ]

    for msg in thread.messages:
        if msg.msg_type == "user":
            content: list[Any] = [
                {
                    "type": "text",
                    "text": msg.message
                }
            ]
            
            for url in msg.image_urls:
                content.append({
                    "type": "image_url",
                    "image_url": {
                        "url": url
                    }
                })
            
            for url in msg.video_urls:
                content.append({
                    "type": "video",
                    "video": url,
                    # Taken from doc example https://docs.vllm.ai/en/latest/features/multimodal_inputs/#video-inputs
                    "total_pixels": 20480 * 28 * 28,
                    "min_pixels": 16 * 28 * 28,
                })

            messages.append({ "role": "user", "content": content })

        elif msg.msg_type == "assistant":
            messages.append({ "role": "assistant", "content": msg.message })

    thread_response = ThreadResponse(message="", text_files=[], image_prompts=[])
    print(messages)

    while True:
        outputs = llm.chat(messages, sampling_params=sampling_params, tools=tools)
        output = outputs[0].outputs[0].text.strip()
        think_index = output.find("</think>")

        if think_index != -1:
            output = output[think_index + len("</think>")::].strip()

        try:
            # Sometimes is produced
            tool_output = output.removeprefix("<tool_call>").removesuffix("</tool_call>")
            print(tool_output)
            tool_call = json.loads(tool_output)

            messages.append(
                {
                    "role": "assistant",
                    "content": output,
                }
            )
            
            content = tool_functions[tool_call["name"]](**tool_call["arguments"])

            messages.append(
                {
                    "role": "tool",
                    "content": [content],
                    "tool_call_id": generate_random_id(),
                }
            )

        except json.JSONDecodeError:
            print(output)
            thread_response.message = output
            response.thread_responses.append(thread_response)
            break


open("/tmp/vllm_infer.json", "w").write(response.model_dump_json())