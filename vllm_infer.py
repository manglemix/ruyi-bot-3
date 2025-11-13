from vllm import LLM, SamplingParams
import sys
from vllm_infer_interface import Request


llm = LLM(model="Qwen/Qwen3-VL-30B-A3B-Thinking")
sampling_params = SamplingParams(temperature=0.6, top_p=0.95, top_k=20, min_p=0, max_tokens=1024)

json_data = sys.stdin.readline().strip()
request = Request.model_validate_json(json_data)


for thread in request.threads:
    
