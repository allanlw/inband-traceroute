import type { Node } from '@/services/traceApi'

export type TraceEvent =
  | { type: 'hop'; ttl: number; hop: any }
  | { type: 'reverseDns'; ttl: number; reverseDns: any }
  | { type: 'error'; error: any }

export class TraceConnection {
  private eventSource: EventSource | null = null
  private traceData: { [ttl: number]: any } = {}
  private reverseDns: { [ttl: number]: any } = {}
  private listeners: Array<(events: { hops: [number, any][]; reverseDnsMap: { [ttl: number]: any } }) => void> = []

  constructor(
    private node: Node,
    private protocol: 'IPv4' | 'IPv6',
    private getNodeUrl: (node: Node, protoKey: string) => string
  ) {}

  connect() {
    const protoKey = this.protocol === 'IPv4' ? 'ipv4' : 'ipv6'
    const url = this.getNodeUrl(this.node, protoKey)
    this.eventSource = new EventSource(url)
    this.eventSource.onmessage = (event) => {
      const evt = JSON.parse(event.data)
      if (evt && 'Hop' in evt) {
        const hop = evt.Hop
        this.traceData[hop.ttl] = hop
        this.publish()
      } else if (evt && 'ReverseDns' in evt) {
        const rdns = evt.ReverseDns
        for (const [ttl, hop] of Object.entries(this.traceData)) {
          if (hop.addr === rdns.ip) {
            this.reverseDns[Number(ttl)] = rdns
          }
        }
        this.publish()
      }
    }
    this.eventSource.onerror = (err) => {
      this.eventSource?.close()
      this.eventSource = null
      this.publish()
    }
  }

  disconnect() {
    this.eventSource?.close()
    this.eventSource = null
  }

  onUpdate(listener: (events: { hops: [number, any][]; reverseDnsMap: { [ttl: number]: any } }) => void) {
    this.listeners.push(listener)
  }

  private publish() {
    const hops = Object.entries(this.traceData)
      .map(([k, v]) => [Number(k), v] as [number, any])
      .sort((a, b) => b[0] - a[0])
    for (const listener of this.listeners) {
      listener({ hops, reverseDnsMap: { ...this.reverseDns } })
    }
  }
}
