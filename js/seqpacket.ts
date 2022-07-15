import { SeqpacketSocketWrap } from './addon'

export class SeqpacketSocket {
  private wrap: SeqpacketSocketWrap
  private closed: boolean = false

  constructor() {
    this.wrap = new SeqpacketSocketWrap()
  }

  private checkClosed() {
    if (this.closed) {
      throw new Error('SeqpacketSocket has been closed');
    }
  }

  connect(serverPath: string, cb: () => any) {
    this.checkClosed();
    this.wrap.connect(serverPath, cb)
  }

  listen(bindpath: string, backlog: number = 511) {
    this.checkClosed();
    this.wrap.listen(bindpath, backlog)
  }

  close() {
    if (this.closed) {
      return
    }
    this.closed = true
    this.wrap.close()
  }
}
