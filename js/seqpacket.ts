import { EventEmitter } from 'events';
import { SeqpacketSocketWrap } from './addon'

type ConnectionCb = (socket: SeqpacketSocket, addr: string) => void;

// events: close, connection, error, listening
export class SeqpacketServer extends EventEmitter {
  private wrap: SeqpacketSocketWrap
  private closed: boolean = false

  constructor() {
    super();
    this.wrap = new SeqpacketSocketWrap()
  }

  private checkClosed() {
    if (this.closed) {
      throw new Error('SeqpacketServer has been closed');
    }
  }

  private onConnection = (fd: number, addr: string) => {
    const socket = new SeqpacketSocket(fd);
    this.emit('connection', socket, addr);
  }

  address(): string {
    this.checkClosed();
    return this.wrap.address()
  }

  close() {
    if (this.closed) {
      return
    }
    this.closed = true
    this.wrap.close()
  }

  getConnections() {
    // TODO
  }

  listen(bindpath: string, backlog: number = 511) {
    this.checkClosed();
    this.wrap.setSocketCb(this.onConnection);
    this.wrap.listen(bindpath, backlog)
  }

  // ref() {
  //   // TODO
  // }

  // unref() {
  //   // TOOD
  // }
}

export class SeqpacketSocket extends EventEmitter {
  private wrap: SeqpacketSocketWrap
  private closed: boolean = false

  constructor(fd?: number) {
    super();
    this.wrap = new SeqpacketSocketWrap(fd)
  }

  private checkClosed() {
    if (this.closed) {
      throw new Error('SeqpacketSocket has been closed');
    }
  }

  onConnection = (fd: number, addr: string) => {
    const socket = new SeqpacketSocket(fd);
    this.emit('connection', socket, addr);
  }

  connect(serverPath: string, cb: () => any) {
    this.checkClosed();
    this.wrap.connect(serverPath, cb)
  }

  listen(bindpath: string, backlog: number = 511) {
    this.checkClosed();
    this.wrap.setSocketCb(this.onConnection);
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
