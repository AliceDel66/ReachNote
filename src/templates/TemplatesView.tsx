import { BadgeInfo, FileText, Github, Play, Rss } from "lucide-react";

import { TEMPLATES } from "../constants";
import type { TemplateId, TemplateItem } from "../types";

interface TemplatesViewProps {
  selectedTemplateId: TemplateId;
  defaultTemplateId: TemplateId;
  onTemplateChange: (templateId: TemplateId) => void;
}

export function TemplatesView({ selectedTemplateId, defaultTemplateId, onTemplateChange }: TemplatesViewProps) {
  return (
    <div className="templates-screen">
      <div className="page-heading">
        <h1>研究模板</h1>
        <p>系统模板已注册，可选择默认模板；暂不支持自定义编辑。</p>
      </div>
      <div className="template-grid">
        {TEMPLATES.map((item) => (
          <TemplateCard
            defaultTemplateId={defaultTemplateId}
            isSelected={selectedTemplateId === item.id}
            item={item}
            key={item.id}
            onSelect={() => onTemplateChange(item.id)}
          />
        ))}
      </div>
      <div className="soft-banner templates-note">
        <BadgeInfo size={24} />
        <span>本阶段所有模板共用 research_card_v1 输出结构，只改变分析 prompt 的关注重点。</span>
      </div>
    </div>
  );
}

function TemplateCard({
  item,
  isSelected,
  defaultTemplateId,
  onSelect
}: {
  item: TemplateItem;
  isSelected: boolean;
  defaultTemplateId: TemplateId;
  onSelect: () => void;
}) {
  return (
    <article className={`template-card ${isSelected ? "selected" : ""}`}>
      <div className={`template-icon ${item.icon}`}>
        {item.icon === "github" && <Github size={54} fill="currentColor" />}
        {item.icon === "article" && <FileText size={56} />}
        {item.icon === "video" && <Play size={46} fill="currentColor" />}
        {item.icon === "rss" && <Rss size={52} />}
      </div>
      <div className="template-copy">
        <h2>{item.title}</h2>
        <p>{item.description}</p>
        <p className="template-profile">{item.promptProfile}</p>
        <div className="template-tags">
          {item.chips.map((chip) => (
            <span key={chip}>{chip}</span>
          ))}
        </div>
      </div>
      <div className="template-actions">
        <span className={`template-state ${item.state === "preview" ? "preview" : ""}`}>
          {defaultTemplateId === item.id ? "默认" : item.state === "preview" ? "可用" : "计划中"}
        </span>
        <button type="button" className="template-select" disabled={defaultTemplateId === item.id} onClick={onSelect}>
          {defaultTemplateId === item.id ? "已设默认" : "设为默认"}
        </button>
      </div>
    </article>
  );
}
