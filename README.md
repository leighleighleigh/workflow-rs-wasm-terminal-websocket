# Getting started

```shell
cargo install mprocs
cargo install bacon
cargo install https
cargo install websocat

# Launches the processes defined in 'mprocs.yaml' :
# - hot-reloading compilation (bacon)
# - a HTTP server at localhost:8000 (http)
# - a websocket echo-server at localhost:9090 (websocat)
mprocs
```