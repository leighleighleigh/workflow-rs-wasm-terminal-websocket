# Getting started

```shell
cargo install mprocs bacon https websocat

# Launches the processes defined in 'mprocs.yaml' :
# - hot-reloading compilation (bacon)
# - a HTTP server at localhost:8000 (http - host these things please)
# - a websocket echo-server at localhost:9090 (websocat)
mprocs
```