[unix-socket](../README.md) / [Exports](../modules.md) / SeqpacketSocket

# Class: SeqpacketSocket

SeqpacketSocket is an abstraction of a SOCK_SEQPACKET socket.

SeqpacketSocket is also an `EventEmitter` and will emit events including:

### Event: `'connect'`

Emitted when a socket connection is successfully established.

### Event: `'data'`

- buffer `Buffer`
Emitted when data is received. All message boundaries in incoming datagrams are preserved.

### Event: `'end'`
Emitted when the other end of the socket signals the end of transmission, thus ending the readable side of the socket.

### Event: `'error'`
- error `Error`
Emitted when an error occurs. The 'close' event will be called directly following this event.

### Event: `'close'`
Emitted once the socket is fully closed.

## Hierarchy

- `EventEmitter`

  ↳ **`SeqpacketSocket`**

## Table of contents

### Constructors

- [constructor](SeqpacketSocket.md#constructor)

### Methods

- [connect](SeqpacketSocket.md#connect)
- [destroy](SeqpacketSocket.md#destroy)
- [end](SeqpacketSocket.md#end)
- [getInternalReadBufferSize](SeqpacketSocket.md#getinternalreadbuffersize)
- [ref](SeqpacketSocket.md#ref)
- [setInternalReadBufferSize](SeqpacketSocket.md#setinternalreadbuffersize)
- [unref](SeqpacketSocket.md#unref)
- [write](SeqpacketSocket.md#write)

## Constructors

### constructor

• **new SeqpacketSocket**(`fd?`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `fd?` | `number` |

#### Overrides

EventEmitter.constructor

#### Defined in

seqpacket.ts:147

## Methods

### connect

▸ **connect**(`serverPath`, `connectCb?`): `void`

Initiate a connection on a given socket.

This function is asynchronous. When the connection is established, the 'connect' event will be emitted.
However, connect() will throw error synchronously if the 'serverPath' is not a valid Seqpacket server.

#### Parameters

| Name | Type |
| :------ | :------ |
| `serverPath` | `string` |
| `connectCb?` | [`NotifyCb`](../modules.md#notifycb) |

#### Returns

`void`

#### Defined in

seqpacket.ts:209

___

### destroy

▸ **destroy**(): `void`

Ensures that no more I/O activity happens on this socket. Destroys the stream and closes the connection.

#### Returns

`void`

#### Defined in

seqpacket.ts:272

___

### end

▸ **end**(`cb?`): `void`

Half-closes the socket. i.e., it sends a FIN packet. It is possible the server will still send some data.

#### Parameters

| Name | Type |
| :------ | :------ |
| `cb?` | [`NotifyCb`](../modules.md#notifycb) |

#### Returns

`void`

#### Defined in

seqpacket.ts:231

___

### getInternalReadBufferSize

▸ **getInternalReadBufferSize**(): `number`

Return the size of buffer that SeqpacketSocket uses to receive data. The data will be truncated if the buffer size is not large enough.

Default size is 256KB.

#### Returns

`number`

#### Defined in

seqpacket.ts:242

___

### ref

▸ **ref**(): `void`

Reference the socket so that it will prevent Node.js process from exiting automatically.

#### Returns

`void`

#### Defined in

seqpacket.ts:258

___

### setInternalReadBufferSize

▸ **setInternalReadBufferSize**(`size`): `void`

Set the size of buffer that SeqpacketSocket uses to receive data.

#### Parameters

| Name | Type |
| :------ | :------ |
| `size` | `number` |

#### Returns

`void`

#### Defined in

seqpacket.ts:251

___

### unref

▸ **unref**(): `void`

Unreference the socket so that it won't prevent Node.js process from exiting automatically.

#### Returns

`void`

#### Defined in

seqpacket.ts:265

___

### write

▸ **write**(`buf`, `offset`, `length`, `cb?`): `void`

Sends data on the socket.

#### Parameters

| Name | Type |
| :------ | :------ |
| `buf` | `Buffer` |
| `offset` | `number` |
| `length` | `number` |
| `cb?` | [`NotifyCb`](../modules.md#notifycb) |

#### Returns

`void`

#### Defined in

seqpacket.ts:222
