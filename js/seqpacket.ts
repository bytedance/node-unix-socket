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
  seqRef,
  seqUnref,
} from './addon';

export type NotifyCb = () => void;

function wrapSocket(obj: EventEmitter, fd?: number) {
  obj.emit = obj.emit.bind(obj);
  seqCreateSocket(obj, fd);
}

/**
 * SeqpacketServer is used to create a SOCK_SEQPACKET server. SeqpacketServer doesn't works on MacOS.
 *
 * SeqpacketServer is also an `EventEmitter` and will emit events including:
 *
 * ### Event: `'connection'`:
 * - socket `SeqpacketSocket`
 * - bindpath `string`
 *
 * Emitted when a new connection is made.
 *
 * ### Event: `'error'`
 * - error `Error`
 *
 * Emitted when an error occurs.
 *
 * ### Event: `'close'`
 *
 * Emitted when the server closes.
 */
export class SeqpacketServer extends EventEmitter {
  private closed: boolean = false;

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
    // TODO test this
    this.emit('error', err);
  };

  private onConnection = (fd: number, addr: string) => {
    const socket = new SeqpacketSocket(fd);
    this.emit('connection', socket, addr);
  };

  /**
   * Returns the bound address.
   * @returns
   */
  address(): string {
    this.checkClosed();
    return seqAddress(this);
  }

  /**
   * Stops the server from accepting new connections and keeps existing connections.
   *
   * This function is synchronous.
   * @returns
   */
  close() {
    if (this.closed) {
      return;
    }
    this.closed = true;
    seqClose(this);
  }

  /**
   * Start a server listening for connections on the given path. This function is synchronous.
   * @param bindpath
   * @param backlog
   */
  listen(bindpath: string, backlog: number = 511) {
    this.checkClosed();
    seqListen(this, bindpath, backlog);
  }

  /**
   * Reference the server so that it will prevent Node.js process from exiting automatically.
   */
  ref() {
    seqRef(this);
  }

  /**
   * Unreference the server so that it won't prevent Node.js process from exiting automatically.
   */
  unref() {
    seqUnref(this);
  }
}

/**
 * SeqpacketSocket is an abstraction of a SOCK_SEQPACKET socket.
 *
 * SeqpacketSocket is also an `EventEmitter` and will emit events including:
 *
 * ### Event: `'connect'`
 *
 * Emitted when a socket connection is successfully established.
 *
 * ### Event: `'data'`
 *
 * - buffer `Buffer`
 * Emitted when data is received. All message boundaries in incoming datagrams are preserved.
 *
 * ### Event: `'end'`
 * Emitted when the other end of the socket signals the end of transmission, thus ending the readable side of the socket.
 *
 * ### Event: `'error'`
 * - error `Error`
 * Emitted when an error occurs. The 'close' event will be called directly following this event.
 *
 * ### Event: `'close'`
 * Emitted once the socket is fully closed.
 */
export class SeqpacketSocket extends EventEmitter {
  private destroyed: boolean = false;
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
      this.endCb();
      this.endCb = undefined;
    }
  };

  private onData = (buf: Buffer) => {
    if (buf.length === 0) {
      this.emit('end');
    } else {
      this.emit('data', buf);
    }
  };

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
  };

  private onConnect = () => {
    seqStartRecv(this);
    this.emit('connect');
    if (this.connectCb) {
      this.connectCb();
      this.connectCb = undefined;
    }
  };

  // address(): string {
  //   this.checkDestroyed();
  //   return seqAddress(this);
  // }

  /**
   * Initiate a connection on a given socket.
   *
   * This function is asynchronous. When the connection is established, the 'connect' event will be emitted.
   * However, connect() will throw error synchronously if the 'serverPath' is not a valid Seqpacket server.
   * @param serverPath
   * @param connectCb
   */
  connect(serverPath: string, connectCb?: NotifyCb) {
    this.checkDestroyed();
    this.connectCb = connectCb;
    seqConnect(this, serverPath);
  }

  /**
   * Sends data on the socket.
   * @param buf
   * @param offset
   * @param length
   * @param cb
   */
  write(buf: Buffer, offset: number, length: number, cb?: NotifyCb) {
    this.checkDestroyed();
    seqWrite(this, buf, offset, length, cb);
  }

  /**
   * Half-closes the socket. i.e., it sends a FIN packet. It is possible the server will still send some data.
   * @param cb
   */
  end(cb?: NotifyCb) {
    this.endCb = cb;
    seqShutdownWhenFlushed(this);
  }

  /**
   * Return the size of buffer that SeqpacketSocket uses to receive data. The data will be truncated if the buffer size is not large enough.
   *
   * Default size is 256KB.
   * @returns
   */
  getInternalReadBufferSize(): number {
    return seqGetNapiBufSize(this);
  }

  /**
   * Set the size of buffer that SeqpacketSocket uses to receive data.
   *
   * @param size
   */
  setInternalReadBufferSize(size: number) {
    seqSetNapiBufSize(this, size);
  }

  /**
   * Reference the socket so that it will prevent Node.js process from exiting automatically.
   */
  ref() {
    seqRef(this);
  }

  /**
   * Unreference the socket so that it won't prevent Node.js process from exiting automatically.
   */
  unref() {
    seqUnref(this);
  }

  /**
   * Ensures that no more I/O activity happens on this socket. Destroys the stream and closes the connection.
   */
  destroy() {
    if (this.destroyed) {
      return;
    }
    this.destroyed = true;
    seqClose(this);
  }
}
