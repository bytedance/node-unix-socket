import { EventEmitter } from 'events';
import {
  seqAddress,
  seqClose,
  seqConnect,
  seqCreateSocket,
  seqListen,
  seqSetNapiBufSize,
  seqGetNapiBufSize,
  seqShutdownWhenFlushed,
  seqStartRecv,
  seqWrite,
} from './addon'

export type NotifyCb = () => void;

function wrapSocket(obj: EventEmitter, fd?: number) {
  obj.emit = obj.emit.bind(obj);
  seqCreateSocket(obj, fd);
}

/**
 * TODO add docs
 */
export class SeqpacketServer extends EventEmitter {
  private closed: boolean = false

  constructor() {
    super();
    wrapSocket(this);
    this.on('_connection', this.onConnection);
    this.on('_error', this.onError);
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
    return seqAddress(this)
  }

  close() {
    if (this.closed) {
      return
    }
    this.closed = true
    seqClose(this)
  }

  listen(bindpath: string, backlog: number = 511) {
    this.checkClosed();
    seqListen(this, bindpath, backlog);
  }

  // ref() {
  //   // TODO
  // }

  // unref() {
  //   // TOOD
  // }
}

/**
 * TODO add docs
 */
export class SeqpacketSocket extends EventEmitter {
  private destroyed: boolean = false
  private connectCb?: NotifyCb;
  private endCb?: NotifyCb;

  constructor(fd?: number) {
    super();
    wrapSocket(this, fd);
    if (fd) {
      seqStartRecv(this);
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
      seqStartRecv(this);
      this.emit('connect');
      if (this.connectCb) {
        this.connectCb()
        this.connectCb = undefined;
      }
    })
  }

  address(): string {
    this.checkDestroyed();
    return seqAddress(this)
  }

  connect(serverPath: string, connectCb?: NotifyCb) {
    this.checkDestroyed();
    this.connectCb = connectCb;
    seqConnect(this, serverPath);
  }

  write(buf: Buffer, offset: number, length: number, cb?: NotifyCb) {
    seqWrite(this, buf, offset, length, cb);
  }

  end(cb?: NotifyCb) {
    this.endCb = cb;
    seqShutdownWhenFlushed(this);
  }

  setInternalReadBufferSize(size: number) {
    seqSetNapiBufSize(this, size)
  }

  getInternalReadBufferSize(): number {
    return seqGetNapiBufSize(this)
  }

  destroy() {
    if (this.destroyed) {
      return
    }
    this.destroyed = true
    seqClose(this)
  }
}
