# unix-socket

## Tmp
- Connect, Accept
  - how to make connect async?
    - set nonblock and connect
    - wait POLLOUT?
  - Mode
    - all callbacks, a lot
    - event emitter instead

## TODO

- support `socket.setSendBufferSize(size)`
- current, server don't work with cluster module
