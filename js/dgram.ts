import { DgramSocketWrap } from './addon'

type FnRecv = (err: undefined | Error, buf: Buffer) => void;
type FnSend = (err: undefined | Error) => void;

export class DgramSocket {
  private wrap: DgramSocketWrap;
  private closed: boolean = false

  constructor(onRecv: FnRecv) {
    this.wrap = new DgramSocketWrap(onRecv)
    // NOTE: always refThis() before startRecv() in avoid of DgramSocketWrap being
    // reclaimed too early.
    this.wrap.refThis(this.wrap)
    this.wrap.startRecv();
  }

  private checkClosed() {
    if (this.closed) {
      throw new Error('DgramSocket has been closed')
    }
  }

  bind(socketPath: string) {
    this.checkClosed();
    this.wrap.bind(socketPath);
  }

  sendTo(buf: Buffer, offset: number, length: number, destPath: string, onWrite: FnSend) {
    this.checkClosed();
    this.wrap.sendTo(buf, offset, length, destPath, onWrite);
  }

  getRecvBufferSize() {
    return this.wrap.getRecvBufferSize()
  }

  setRecvBufferSize(size: number) {
    return this.wrap.setRecvBufferSize(size)
  }

  getSendBufferSize() {
    return this.wrap.getSendBufferSize()
  }

  setSendBufferSize(size: number) {
    return this.wrap.setSendBufferSize(size)
  }

  address(): string {
    return this.wrap.address();
  }

  close() {
    if (this.closed) {
      return
    }
    this.closed = true
    this.wrap.close()
  }
}
