from vllm import LLM, SamplingParams
import sys
from vllm_infer_interface import Request
import pyjson5
from datetime import datetime


llm = LLM(model="Qwen/Qwen3-VL-30B-A3B-Thinking")
sampling_params = SamplingParams(temperature=0.6, top_p=0.95, top_k=20, min_p=0, max_tokens=1024)

tools = pyjson5.loads(open("tools.jsonc", "r").read())

system_message = open("system_msg.txt", "r").read()
system_message += f"\nThe current datetime is {datetime.now().isoformat()}"

json_data = sys.stdin.readline().strip()
request = Request.model_validate_json(json_data)


for thread in request.threads:
    messages = []

    for msg in thread.messages:
        if msg.msg_type == "user":
            content = []
            
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

    while True:
        outputs = llm.chat(messages, sampling_params=sampling_params, tools=tools)
        output = outputs[0].outputs[0].text.strip()

        messages.append(
            {
                "role": "assistant",
                "content": output,
            }
        )

        try:
        tool_calls = pyjson5.loads(output)
        tool_answers = [
            tool_functions[call["name"]](**call["arguments"]) for call in tool_calls
        ]
