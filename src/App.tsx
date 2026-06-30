import { useState } from "react";
import {
  Button,
  Card,
  CardBody,
  CardHeader,
  Chip,
  Input,
  Spinner,
  Textarea,
} from "@heroui/react";
import { invoke } from "@tauri-apps/api/core";
import { readText } from "@tauri-apps/plugin-clipboard-manager";

/** 与 reachnote-core 的 AnalysisResult 对齐。 */
type AnalysisResult = {
  title: string;
  summary: string;
  key_points: string[];
  tags: string[];
  score: number;
  next_action: string;
  model: string;
};

export default function App() {
  const [url, setUrl] = useState("");
  const [content, setContent] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<AnalysisResult | null>(null);

  async function pasteFromClipboard() {
    try {
      setUrl((await readText()) ?? "");
    } catch {
      /* 剪贴板为空或无权限时忽略 */
    }
  }

  async function handleCapture() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      // P0 竖切：填了正文则直接测 AI 分析（不依赖 agent-reach 安装）；
      // 留空则后端走 agent-reach 读取。Provider 此处示例用本地 claude-cli。
      const res = await invoke<AnalysisResult>("capture", {
        args: {
          url,
          content: content || null,
          provider: { provider: "claude-cli" },
        },
      });
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="max-w-xl mx-auto p-6 flex flex-col gap-4">
      <header className="flex flex-col gap-1">
        <h1 className="text-2xl font-bold">ReachNote</h1>
        <p className="text-default-500 text-sm">
          AI-powered web capture for Notion · P0 骨架
        </p>
      </header>

      <Card>
        <CardHeader className="font-semibold">捕获链接</CardHeader>
        <CardBody className="flex flex-col gap-3">
          <div className="flex gap-2 items-end">
            <Input
              label="URL"
              placeholder="https://github.com/owner/repo"
              value={url}
              onValueChange={setUrl}
            />
            <Button variant="flat" onPress={pasteFromClipboard}>
              粘贴
            </Button>
          </div>
          <Textarea
            label="正文（可选）"
            placeholder="粘贴正文可跳过 Agent-Reach 直接测试 AI 分析；留空则由 agent-reach 读取"
            value={content}
            onValueChange={setContent}
            minRows={3}
          />
          <Button
            color="primary"
            onPress={handleCapture}
            isLoading={loading}
            isDisabled={!url}
          >
            分析并生成研究卡
          </Button>
        </CardBody>
      </Card>

      {loading && <Spinner label="分析中…" />}

      {error && (
        <Card className="border border-danger">
          <CardBody className="text-danger text-sm whitespace-pre-wrap">
            {error}
          </CardBody>
        </Card>
      )}

      {result && (
        <Card>
          <CardHeader className="flex justify-between items-center gap-2">
            <span className="font-semibold">{result.title}</span>
            <Chip color="success" variant="flat">
              Score {result.score}
            </Chip>
          </CardHeader>
          <CardBody className="flex flex-col gap-3 text-sm">
            <p>{result.summary}</p>
            {result.key_points?.length > 0 && (
              <ul className="list-disc pl-5">
                {result.key_points.map((k, i) => (
                  <li key={i}>{k}</li>
                ))}
              </ul>
            )}
            <div className="flex flex-wrap gap-1">
              {result.tags?.map((t, i) => (
                <Chip key={i} size="sm" variant="flat">
                  {t}
                </Chip>
              ))}
            </div>
            {result.next_action && (
              <p className="text-default-500">
                <b>下一步：</b>
                {result.next_action}
              </p>
            )}
            <p className="text-default-400 text-xs">via {result.model}</p>
          </CardBody>
        </Card>
      )}
    </div>
  );
}
