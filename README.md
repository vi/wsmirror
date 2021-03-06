# Description

wsmirror is a simple server application that lets you run Websocket echo server for playing with WebSockets and testing WebSocket client implementations.

It should work similar to like `echo.websocket.org` worked before.

# Public instances

* ws://ws.vi-server.org/mirror , wss://ws.vi-server.org/mirror
* ws://vi-server.org:1939/


# Usage

    Usage: wsmirror <tcp_bind_socket_address>

Specify socket address like `127.0.0.1:1234` or `[::]:8080` as a sole command line argument.  
Other options, such as maximum number of simultaneous clients, are configurable only in source code constants.


# Metrics

If you build the crate with `metrics` feature (not default) and set `PROMETHEUS_EXPORT_ADDR` environment variable to e.g. `127.0.0.1:1235` then Prometheus metrics would be exported on ` http://127.0.0.1:1235/metrics`.

# See also

* wss://echo.websocket.events/
