# WORK IN PROGRESS :)

```shell
# Getting started
cargo install mprocs bacon https websocat

# Off you go!
mprocs

# 'mprocs' is a task runner, configured by 'mprocs.yaml', which runs:
# - The BACKEND (bacon -> cargo run)
# - The FRONTEND (bacon -> wasm-pack build)
# - A HTTP server at localhost:8000 (http ./frontend/root/)
# - A websocket echo-server at localhost:9090 (websocat)
```
