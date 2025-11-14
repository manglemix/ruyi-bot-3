from typing import Any
from vllm import LLM, SamplingParams
import sys
from vllm_infer_interface import Request, Response, ThreadResponse
import pyjson5
from datetime import datetime


# llm = LLM(model="Qwen/Qwen3-VL-30B-A3B-Thinking")
llm = LLM(model="cpatonn/Qwen3-VL-8B-Instruct-AWQ-4bit", gpu_memory_utilization=0.7, max_model_len=15000)
sampling_params = SamplingParams(temperature=0.6, top_p=0.95, top_k=20, min_p=0, max_tokens=1024)

tools = pyjson5.loads(open("tools.jsonc", "r").read())

system_message = open("system_msg.txt", "r").read()
system_message += f"\nThe current datetime is {datetime.now().isoformat()}"

json_data = sys.stdin.readline().strip()
request = Request.model_validate_json(json_data)
response = Response(thread_responses=[])


for thread in request.threads:
    messages = []

    for msg in thread.messages:
        if msg.msg_type == "user":
            content: list[Any] = [
                {
                    "type": "text",
                    "text": pyjson5.dumps({ "username": msg.author, "message": msg.message })
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

    while True:
        outputs = llm.chat(messages, sampling_params=sampling_params, tools=tools)
        output = outputs[0].outputs[0].text.strip()

        try:
            tool_calls = pyjson5.loads(output)

            messages.append(
                {
                    "role": "assistant",
                    "content": output,
                }
            )
            # tool_answers = [
            #     tool_functions[call["name"]](**call["arguments"]) for call in tool_calls
            # ]

        except pyjson5.Json5DecoderException:
            # output = output.replace('\\', '\\\\')
            # open("test.json", "w").write(output)
            thread_response.message = output
            response.thread_responses.append(thread_response)
            break


print(response.model_dump_json())
