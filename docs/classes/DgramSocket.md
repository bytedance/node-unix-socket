[node-unix-socket](../README.md) / [Exports](../modules.md) / DgramSocket

# Class: DgramSocket

DgramSocket is used to create a SOCK_DGRAM unix domain socket.
Currently DgramSocket doesn't work with `cluster` module.

DgramSocket is also an `EventEmitter` and will emit events including:

### Event: `'data'`
- buffer `Buffer`
- path `string`

Emitted when data is received. `path` indicates remote address information.

### Event: `'error'`
- error `Error`

Emitted when an error occurs.

### Event: `'close'`
The 'close' event is emitted after a socket is closed with close().

## Hierarchy

- `EventEmitter`

  ↳ **`DgramSocket`**

## Table of contents

### Constructors

- [constructor](DgramSocket.md#constructor)

### Methods

- [address](DgramSocket.md#address)
- [bind](DgramSocket.md#bind)
- [close](DgramSocket.md#close)
- [getRecvBufferSize](DgramSocket.md#getrecvbuffersize)
- [getSendBufferSize](DgramSocket.md#getsendbuffersize)
- [sendTo](DgramSocket.md#sendto)
- [setRecvBufferSize](DgramSocket.md#setrecvbuffersize)
- [setSendBufferSize](DgramSocket.md#setsendbuffersize)

## Constructors

### constructor

• **new DgramSocket**()

#### Overrides

EventEmitter.constructor

#### Defined in

dgram.ts:46

## Methods

### address

▸ **address**(): `string`

Returns the bound address.

#### Returns

`string`

#### Defined in

dgram.ts:135

___

### bind

▸ **bind**(`socketPath`): `void`

Listen for datagram messages on a path.

#### Parameters

| Name | Type |
| :------ | :------ |
| `socketPath` | `string` |

#### Returns

`void`

#### Defined in

dgram.ts:75

___

### close

▸ **close**(): `void`

Close the underlying socket and stop listening for data on it.

#### Returns

`void`

#### Defined in

dgram.ts:143

___

### getRecvBufferSize

▸ **getRecvBufferSize**(): `number`

#### Returns

`number`

the SO_RCVBUF socket receive buffer size in bytes.

#### Defined in

dgram.ts:102

___

### getSendBufferSize

▸ **getSendBufferSize**(): `number`

#### Returns

`number`

the SO_SNDBUF socket send buffer size in bytes.

#### Defined in

dgram.ts:118

___

### sendTo

▸ **sendTo**(`buf`, `offset`, `length`, `destPath`, `onWrite?`): `void`

Send messages to the destination path.

#### Parameters

| Name | Type |
| :------ | :------ |
| `buf` | `Buffer` |
| `offset` | `number` |
| `length` | `number` |
| `destPath` | `string` |
| `onWrite?` | [`SendCb`](../modules.md#sendcb) |

#### Returns

`void`

#### Defined in

dgram.ts:88

___

### setRecvBufferSize

▸ **setRecvBufferSize**(`size`): `void`

Sets the SO_RCVBUF socket option. Sets the maximum socket receive buffer in bytes.

#### Parameters

| Name | Type |
| :------ | :------ |
| `size` | `number` |

#### Returns

`void`

#### Defined in

dgram.ts:111

___

### setSendBufferSize

▸ **setSendBufferSize**(`size`): `void`

Sets the SO_SNDBUF socket option. Sets the maximum socket send buffer in bytes.

#### Parameters

| Name | Type |
| :------ | :------ |
| `size` | `number` |

#### Returns

`void`

#### Defined in

dgram.ts:127
