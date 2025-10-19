curl -L https://install.meilisearch.com | sh

python3 -m venv venv
source venv/bin/activate
VLLM_TORCH_BACKEND=AUTO  pip install vllm
