import { useState } from "react"
import { Button } from "@workspace/ui/components/button"
import { Input } from "@workspace/ui/components/input"

type 结果 = { ok: boolean; message: string }

async function 请求JSON(url: string, init?: RequestInit): Promise<结果> {
  const res = await fetch(url, init)
  const ct = res.headers.get("content-type") ?? ""
  if (!ct.includes("application/json")) {
    const text = await res.text()
    return {
      ok: false,
      message: `后端没返回 JSON (${res.status} ${ct})，前 80 字符：${text.slice(0, 80)}`,
    }
  }
  return res.json()
}

export function App() {
  const [路径, 设置路径] = useState("")
  const [消息, 设置消息] = useState("")
  const [忙碌, 设置忙碌] = useState(false)

  async function 检查() {
    if (!路径) return 设置消息("请输入路径")
    设置忙碌(true); 设置消息("检查中…")
    try {
      const data = await 请求JSON(
        `/api/check-wallpaper?path=${encodeURIComponent(路径)}`
      )
      设置消息(data.message)
    } catch (e) {
      设置消息(`请求失败: ${e}`)
    } finally {
      设置忙碌(false)
    }
  }

  async function 设置() {
    if (!路径) return 设置消息("请输入路径")
    设置忙碌(true); 设置消息("设置中…")
    try {
      const data = await 请求JSON("/api/set-wallpaper", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: 路径 }),
      })
      设置消息(data.ok ? `✅ ${data.message}` : `❌ ${data.message}`)
    } catch (e) {
      设置消息(`请求失败: ${e}`)
    } finally {
      设置忙碌(false)
    }
  }

  async function 选文件() {
    设置忙碌(true); 设置消息("打开文件对话框…")
    try {
      const data = await 请求JSON("/api/pick-wallpaper")
      if (data.ok) {
        设置路径(data.message)
        设置消息("已选择路径")
      } else {
        设置消息(data.message)
      }
    } catch (e) {
      设置消息(`请求失败: ${e}`)
    } finally {
      设置忙碌(false)
    }
  }

  async function 一键设置() {
    设置忙碌(true); 设置消息("等待选择文件…")
    try {
      const data = await 请求JSON("/api/pick-and-set")
      设置消息(data.ok ? `✅ ${data.message}` : `❌ ${data.message}`)
    } catch (e) {
      设置消息(`请求失败: ${e}`)
    } finally {
      设置忙碌(false)
    }
  }

  return (
    <div className="flex min-h-svh p-6">
      <div className="flex max-w-md min-w-0 flex-col gap-4 text-sm leading-loose">
        <h1 className="font-medium">切换桌面壁纸！</h1>

        <div className="flex gap-2">
          <Button onClick={一键设置} disabled={忙碌}>
            📁 选择并设置壁纸
          </Button>
          <Button variant="secondary" onClick={选文件} disabled={忙碌}>
            只选路径
          </Button>
        </div>

        <Input
          value={路径}
          onChange={e => 设置路径(e.target.value)}
          placeholder="例如 /home/lilyco/图片/wallpaper.png"
        />
        <div className="flex gap-2">
          <Button onClick={检查} disabled={忙碌 || !路径}>检查</Button>
          <Button onClick={设置} disabled={忙碌 || !路径}>设置壁纸</Button>
        </div>

        {消息 && <p className="text-muted-foreground break-all">{消息}</p>}
      </div>
    </div>
  )
}
