export interface EnrichedInfo {
  country: string | null
  country_name: string | null
  continent: string | null
  continent_name: string | null
  asn: number | null
  as_name: string | null
  as_domain: string | null
}

export interface TraceMessage {
  ttl: number
  hop_type: string
  addr: string | null
  rtt: number | null
  enriched_info: EnrichedInfo | null
}

export interface Node {
  dns_name: string
  ipv4: string
  ipv6: string
}

export type IpVersion = 'ipv4' | 'ipv6'

export interface NodeConnection {
  id: string
  ipVersion: IpVersion
  status: 'connecting' | 'connected' | 'disconnected' | 'error'
  lastMessage?: TraceMessage
  traceMessages: TraceMessage[]
  lastUpdate: Date
  error?: string
}

export interface NodeConnectionMap {
  [nodeId: string]: {
    ipv4: NodeConnection
    ipv6: NodeConnection
  }
}

export type NodeEventCallback = (nodeId: string, ipVersion: IpVersion, message: TraceMessage) => void
export type NodeStatusCallback = (nodeId: string, ipVersion: IpVersion, status: NodeConnection['status'], error?: string) => void

class TraceAPI {
  private eventSources = new Map<string, { ipv4?: EventSource; ipv6?: EventSource }>()
  private baseUrl = 'https://inband-traceroute.net'

  async fetchNodes(): Promise<NodeMap> {
    const response = await fetch(`${this.baseUrl}/nodes.json`)
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    return response.json()
  }

  private parseTraceMessage(data: string): TraceMessage | null {
    try {
      return JSON.parse(data)
    } catch (error) {
      console.error('Failed to parse trace message:', error)
      return null
    }
  }

  private getNodeUrl(node: Node, ipVersion: IpVersion): string {
    const baseDomain = node.dns_name
    const prefix = ipVersion === 'ipv4' ? 'ipv4.' : 'ipv6.'
    return `https://${prefix}${baseDomain}/sse`
  }

  connectToNode(
    nodeId: string,
    node: Node,
    onMessage: NodeEventCallback,
    onStatusChange: NodeStatusCallback
  ): void {
    if (!this.eventSources.has(nodeId)) {
      this.eventSources.set(nodeId, {})
    }

    const sources = this.eventSources.get(nodeId)!
    const versions: IpVersion[] = ['ipv4', 'ipv6']

    versions.forEach((version) => {
      if (sources[version]) return

      onStatusChange(nodeId, version, 'connecting')

      try {
        const url = this.getNodeUrl(node, version)
        const eventSource = new EventSource(url)

        eventSource.onopen = () => {
          onStatusChange(nodeId, version, 'connected')
        }

        eventSource.onmessage = (event) => {
          const message = this.parseTraceMessage(event.data)
          if (message) {
            onMessage(nodeId, version, message)
          }
        }

        eventSource.onerror = () => {
          onStatusChange(nodeId, version, 'error', 'Connection failed')
          eventSource.close()
          sources[version] = undefined
        }

        sources[version] = eventSource
      } catch (error) {
        onStatusChange(
          nodeId,
          version,
          'error',
          error instanceof Error ? error.message : 'Failed to connect'
        )
      }
    })
  }

  disconnectFromNode(nodeId: string, onStatusChange: NodeStatusCallback): void {
    const sources = this.eventSources.get(nodeId)
    if (sources) {
      if (sources.ipv4) {
        sources.ipv4.close()
        onStatusChange(nodeId, 'ipv4', 'disconnected')
      }
      if (sources.ipv6) {
        sources.ipv6.close()
        onStatusChange(nodeId, 'ipv6', 'disconnected')
      }
      this.eventSources.delete(nodeId)
    }
  }

  disconnectAll(): void {
    this.eventSources.forEach((sources) => {
      sources.ipv4?.close()
      sources.ipv6?.close()
    })
    this.eventSources.clear()
  }

  formatRtt(rtt: number): string {
    return (rtt / 1_000_000).toFixed(2) // Convert nanoseconds to milliseconds
  }
}

export const traceApi = new TraceAPI()
