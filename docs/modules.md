[node-unix-socket](README.md) / Exports

# node-unix-socket

## Table of contents

### Classes

- [DgramSocket](classes/DgramSocket.md)
- [SeqpacketServer](classes/SeqpacketServer.md)
- [SeqpacketSocket](classes/SeqpacketSocket.md)

### Type aliases

- [NotifyCb](modules.md#notifycb)
- [SendCb](modules.md#sendcb)

### Functions

- [closeFd](modules.md#closefd)
- [createReuseportFd](modules.md#createreuseportfd)

## Type aliases

### NotifyCb

Ƭ **NotifyCb**: () => `void`

#### Type declaration

▸ (): `void`

##### Returns

`void`

#### Defined in

seqpacket.ts:17

___

### SendCb

Ƭ **SendCb**: (`err`: `undefined` \| `Error`) => `void`

#### Type declaration

▸ (`err`): `void`

##### Parameters

| Name | Type |
| :------ | :------ |
| `err` | `undefined` \| `Error` |

##### Returns

`void`

#### Defined in

dgram.ts:16

## Functions

### closeFd

▸ **closeFd**(`fd`): `void`

Close a fd.

Note that you don't need to manually close fd that is listened by net.Server.

#### Parameters

| Name | Type |
| :------ | :------ |
| `fd` | `number` |

#### Returns

`void`

#### Defined in

socket.ts:40

___

### createReuseportFd

▸ **createReuseportFd**(`port?`, `host?`): `number`

Create a TCP socket with SO_REUSEADDR and SO_REUSEPORT enabled.

Use the returned fd to create a [net.Server](https://nodejs.org/docs/latest-v16.x/api/net.html#class-netserver):

```typescript
const fd = createReuseportFd(9229, '127.0.0.0');
const server = require('net').createServer();
server.listen({ fd }, () => { console.log('listen() successfully') })
```

#### Parameters

| Name | Type | Default value |
| :------ | :------ | :------ |
| `port` | `number` | `0` |
| `host` | `string` | `'0.0.0.0'` |

#### Returns

`number`

Return a fd binds to the address.

#### Defined in

socket.ts:19
