[nix-socket](../README.md) / [Exports](../modules.md) / SeqpacketServer

# Class: SeqpacketServer

SeqpacketServer is used to create a SOCK_SEQPACKET server.
Note that sockets of SOCK_SEQPACKET don't works on MacOS and currently SeqpacketServer doesn't work with `cluster` module, i.e. you can share a SeqpacketServer across different Node.js processes.

SeqpacketServer is also an `EventEmitter` and will emit events including:

### Event: `'connection'`:
- socket `SeqpacketSocket`
- bindpath `string`

Emitted when a new connection is made.

### Event: `'error'`
- error `Error`

Emitted when an error occurs.

### Event: `'close'`

Emitted when the server closes.

## Hierarchy

- `EventEmitter`

  ↳ **`SeqpacketServer`**

## Table of contents

### Constructors

- [constructor](SeqpacketServer.md#constructor)

### Methods

- [address](SeqpacketServer.md#address)
- [close](SeqpacketServer.md#close)
- [listen](SeqpacketServer.md#listen)
- [ref](SeqpacketServer.md#ref)
- [unref](SeqpacketServer.md#unref)

## Constructors

### constructor

• **new SeqpacketServer**()

#### Overrides

EventEmitter.constructor

#### Defined in

seqpacket.ts:48

## Methods

### address

▸ **address**(): `string`

Returns the bound address.

#### Returns

`string`

#### Defined in

seqpacket.ts:75

___

### close

▸ **close**(): `void`

Stops the server from accepting new connections and keeps existing connections.

This function is synchronous.

#### Returns

`void`

#### Defined in

seqpacket.ts:86

___

### listen

▸ **listen**(`bindpath`, `backlog?`): `void`

Start a server listening for connections on the given path. This function is synchronous.

#### Parameters

| Name | Type | Default value |
| :------ | :------ | :------ |
| `bindpath` | `string` | `undefined` |
| `backlog` | `number` | `511` |

#### Returns

`void`

#### Defined in

seqpacket.ts:99

___

### ref

▸ **ref**(): `void`

Reference the server so that it will prevent Node.js process from exiting automatically.

#### Returns

`void`

#### Defined in

seqpacket.ts:107

___

### unref

▸ **unref**(): `void`

Unreference the server so that it won't prevent Node.js process from exiting automatically.

#### Returns

`void`

#### Defined in

seqpacket.ts:114
