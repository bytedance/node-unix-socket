import * as workerThreads from 'worker_threads'
import { initCleanupHook } from './addon'

export { SendCb, DgramSocket } from './dgram'
export { NotifyCb, SeqpacketSocket, SeqpacketServer } from './seqpacket'
export { createReuseportFd, closeFd } from './socket'

// Node.js will abort when threads are termiated if we don't clean up uv handles.
if (!workerThreads.isMainThread) {
  initCleanupHook()
}
