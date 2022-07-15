import * as path from 'path'
import * as fs from 'fs'
import { SeqpacketSocket, SeqpacketServer } from '../js/seqpacket'
import { kTmp, sliently, createDefer } from './util'

const kServerpath = path.resolve(kTmp, './seqpacket_server.sock');

describe('SeqpacketSocket', () => {
  beforeAll(() => {
    sliently(() => fs.mkdirSync(kTmp))
  })
  beforeEach(async () => {
    sliently(() => fs.unlinkSync(kServerpath))
  })

  it('should allow to pass in a fd', async () => {
    // const socket = new SeqpacketSocket(1)
    // socket.close()
  });

  it('should create a sock file', async () => {
    const server = new SeqpacketServer()
    const client = new SeqpacketSocket()

    const { p: waitConnectCb, resolve: resolveConnect } = createDefer()
    const { p: waitConnection, resolve: resolveConnection } = createDefer<{
      socket: SeqpacketSocket,
      addr: string,
    }>()

    server.listen(kServerpath);
    expect(server.address()).toBe(kServerpath);

    server.on('connection', (socket, addr) => {
      resolveConnection({
        socket, addr,
      });
    });

    client.connect(kServerpath, () => {
      resolveConnect()
    });

    const { socket, addr } = await waitConnection
    expect(socket).toBeTruthy()
    expect(addr).toBe('')

    await waitConnectCb

    server.close();

    expect(() => server.listen(kServerpath)).toThrow()

    client.close();
  })
})
