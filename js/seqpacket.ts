import { EventEmitter } from 'events';
import { SeqpacketSocketWrap } from './addon'

// type ConnectionCb = (socket: SeqpacketSocket, addr: string) => void;
type NotifyCb = () => void;

// events: close, connection, error, listening
export class SeqpacketServer extends EventEmitter {
  private wrap: SeqpacketSocketWrap
  private closed: boolean = false

  constructor() {
    super();
    this.wrap = new SeqpacketSocketWrap(this.emit.bind(this))
    this.on('_connection', this.onConnection);
    this.on('error', this.onError);
  }

  private checkClosed() {
    if (this.closed) {
      throw new Error('SeqpacketServer has been closed');
    }
  }

  private onError = (err: Error) => {
    this.close();
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

  listen(bindpath: string, backlog: number = 511) {
    this.checkClosed();
    this.wrap.listen(bindpath, backlog);
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
  private destroyed: boolean = false
  private connectCb?: NotifyCb;
  private endCb?: NotifyCb;

  constructor(fd?: number) {
    super();
    this.wrap = new SeqpacketSocketWrap(this.emit.bind(this), fd)
    if (fd) {
      this.wrap.startRecv();
    }
    this.on('_data', this.onData);
    this.on('_connect', this.onConnect);
    this.on('_error', this.onError);
    this.on('_shutdown', this.onShutdown);
  }

  private onShutdown = () => {
    if (this.endCb) {
      this.endCb()
      this.endCb = undefined;
    }
  }

  private onData = (buf: Buffer) => {
    if (buf.length === 0) {
      this.emit('end')
    } else {
      this.emit('data', buf);
    }
  }

  private checkDestroyed() {
    if (this.destroyed) {
      throw new Error('SeqpacketSocket has been destroyed');
    }
  }

  private onError = (err: Error) => {
    process.nextTick(() => {
      this.emit('error', err);
      this.destroy();
    });
  }

  private onConnect = () => {
    process.nextTick(() => {
      this.wrap.startRecv();
      if (this.connectCb) {
        this.connectCb()
        this.connectCb = undefined;
      }
      this.emit('connect');
    })
  }

  address(): string {
    this.checkDestroyed();
    return this.wrap.address()
  }

  connect(serverPath: string, connectCb?: NotifyCb) {
    this.checkDestroyed();
    this.connectCb = connectCb;
    this.wrap.connect(serverPath);
  }

  write(buf: Buffer, offset: number, length: number, cb?: NotifyCb) {
    this.wrap.write(buf, offset, length, cb);
  }

  end(cb?: NotifyCb) {
    this.endCb = cb;
    this.wrap.shutdownWhenFlushed();
  }

  destroy() {
    if (this.destroyed) {
      return
    }
    this.destroyed = true
    this.wrap.close()
  }
}
