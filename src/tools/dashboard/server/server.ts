import express from 'express'
import { exec } from 'child_process'
import { promisify } from 'util'
import cors from 'cors'

const execAsync = promisify(exec)
const app = express()

app.use(cors())

let lastData: any = null
let lastTs = 0
const CACHE_TTL_MS = 3000

async function getPodsRaw() {
  // Podman 버전에 따라 --format json 이 지원 안 되면 다른 포맷 필요할 수 있음
  const { stdout } = await execAsync('podman ps --format json')
  return JSON.parse(stdout)
}

function mapPodmanToDTO(raw: any[]): any[] {
  return raw.map(item => ({
    id: item.Id,
    name: item.Names?.[0] || item.Names || '',
    image: item.Image || item.ImageName,
    status: item.Status,
    state: item.State,
    createdAt: item.CreatedAt,
    ports: item.Ports,
    command: item.Command,
    labels: item.Labels
  }))
}

app.get('/api/pods', async (_req, res) => {
  try {
    const now = Date.now()
    if (!lastData || now - lastTs > CACHE_TTL_MS) {
      const raw = await getPodsRaw()
      lastData = mapPodmanToDTO(raw)
      lastTs = now
    }
    res.json({ ok: true, data: lastData, cachedAt: new Date(lastTs).toISOString() })
  } catch (e: any) {
    res.status(500).json({ ok: false, error: e.message })
  }
})

app.get('/api/health', (_req, res) => res.json({ ok: true }))

const PORT = process.env.BACKEND_PORT || 5174
app.listen(PORT, () => {
  console.log(`[backend] listening on http://localhost:${PORT}`)
})
