import * as path from 'path'
import * as fs from 'fs'
import { SeqpacketSocket } from '../js/seqpacket'
import { kTmp, sliently, createDefer } from './util'

const kServerpath = path.resolve(kTmp, './seqpacket_server.sock');

describe('SeqpacketSocket', () => {
  beforeAll(() => {
    sliently(() => fs.mkdirSync(kTmp))
  })
  beforeEach(async () => {
    sliently(() => fs.unlinkSync(kServerpath))
  })

  it('should create a sock file', async () => {
    const socket = new SeqpacketSocket()
    const client = new SeqpacketSocket()

    let { p, resolve } = createDefer()

    socket.listen(kServerpath);
    client.connect(kServerpath, () => {
      resolve()
    });

    await p

    socket.close();

    expect(() => socket.listen(kServerpath)).toThrow()

    client.close();
  })
})
