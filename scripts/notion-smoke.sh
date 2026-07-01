#!/usr/bin/env bash
# ReachNote · Notion 连通性自检
# ---------------------------------------------------------------------------
# 目的:在写任何 Notion adapter 代码之前,独立确认 token + database 凭证真实可用。
#       这样第 6 步出问题时,你能立刻区分「凭证问题」还是「代码问题」。
# 用法:
#   1) cp .env.notion.example .env.notion   # 然后填入真实 token / database id
#   2) bash scripts/notion-smoke.sh
# 依赖:curl、python3(macOS 自带)。
# 安全:从 .env.notion(已被 gitignore)读取;只显示 token 前缀;创建一条测试 page 后提示你删除。
# ---------------------------------------------------------------------------
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="${ROOT}/.env.notion"

if [[ -f "${ENV_FILE}" ]]; then
  set -a; source "${ENV_FILE}"; set +a
else
  echo "❌ 未找到 ${ENV_FILE}"
  echo "   先执行: cp .env.notion.example .env.notion  并填入真实值"
  exit 1
fi

: "${NOTION_TOKEN:?缺少 NOTION_TOKEN(在 .env.notion 设置)}"
: "${NOTION_DATABASE_ID:?缺少 NOTION_DATABASE_ID(在 .env.notion 设置)}"
NOTION_VERSION="${NOTION_VERSION:-2022-06-28}"
NOTION_DATA_SOURCE_ID="${NOTION_DATA_SOURCE_ID:-}"

if [[ "${NOTION_TOKEN}" == *REPLACE_ME* || "${NOTION_DATABASE_ID}" == *REPLACE_ME* ]]; then
  echo "❌ .env.notion 仍是模板占位符,请填入真实 token / database id。"
  exit 1
fi

echo "Token 前缀: ${NOTION_TOKEN:0:7}…(其余已隐藏)"
echo "Database : ${NOTION_DATABASE_ID}"
echo "API 版本 : ${NOTION_VERSION}"
echo

api() {  # api <method> <url> [data]
  local method="$1" url="$2" data="${3:-}"
  if [[ -n "${data}" ]]; then
    curl -sS -w $'\n%{http_code}' -X "${method}" \
      -H "Authorization: Bearer ${NOTION_TOKEN}" \
      -H "Notion-Version: ${NOTION_VERSION}" \
      -H "Content-Type: application/json" \
      --data "${data}" "${url}"
  else
    curl -sS -w $'\n%{http_code}' -X "${method}" \
      -H "Authorization: Bearer ${NOTION_TOKEN}" \
      -H "Notion-Version: ${NOTION_VERSION}" "${url}"
  fi
}

# --- Step 1: GET database — 验证 token 有效 + 已 share + 读取真实 schema ---
echo "[1/2] 读取 database(验证 token 与 share)…"
db_resp="$(api GET "https://api.notion.com/v1/databases/${NOTION_DATABASE_ID}")"
db_code="$(tail -n1 <<<"${db_resp}")"
db_body="$(sed '$d' <<<"${db_resp}")"

if [[ "${db_code}" != "200" ]]; then
  echo "❌ 读取 database 失败(HTTP ${db_code})"
  python3 -c 'import sys,json;d=json.load(sys.stdin);print("   Notion:",d.get("code"),"-",d.get("message"))' <<<"${db_body}" 2>/dev/null || echo "   ${db_body}"
  echo
  echo "常见原因:"
  echo "  unauthorized      → NOTION_TOKEN 错误或已失效"
  echo "  object_not_found  → database 未 share 给 integration,或 NOTION_DATABASE_ID 错"
  echo "  validation_error  → 该 database 含多个 data source,改用 2025-09-03 + NOTION_DATA_SOURCE_ID"
  exit 1
fi

title_prop="$(python3 -c 'import sys,json;d=json.load(sys.stdin);print(next((k for k,v in d.get("properties",{}).items() if v.get("type")=="title"),""))' <<<"${db_body}")"
ds_found="$(python3 -c 'import sys,json;d=json.load(sys.stdin);ds=d.get("data_sources") or [];print(ds[0]["id"] if ds else "")' <<<"${db_body}")"
props="$(python3 -c 'import sys,json;print(", ".join(json.load(sys.stdin).get("properties",{}).keys()))' <<<"${db_body}")"

echo "✅ database 可访问"
echo "   标题 property: ${title_prop:-（未找到 title 类型字段!)}"
echo "   现有字段: ${props}"
[[ -n "${ds_found}" ]] && echo "   data_source_id(2025-09-03 用): ${ds_found}"
echo

# --- Step 2: POST page — 验证能写入 ---
echo "[2/2] 创建一条测试 page…"
parent_json="$(python3 - "${NOTION_VERSION}" "${NOTION_DATABASE_ID}" "${NOTION_DATA_SOURCE_ID}" "${ds_found}" <<'PY'
import json,sys
ver,db,ds_env,ds_found=sys.argv[1:5]
ds=ds_env or ds_found
if ver>="2025-09-03":
    if not ds:
        sys.stderr.write("需要 data_source_id 但未提供也未发现\n"); sys.exit(3)
    print(json.dumps({"type":"data_source_id","data_source_id":ds}))
else:
    print(json.dumps({"database_id":db}))
PY
)"

page_body="$(python3 - "${parent_json}" "${title_prop:-Name}" <<'PY'
import json,sys
parent=json.loads(sys.argv[1]); title_prop=sys.argv[2] or "Name"
print(json.dumps({"parent":parent,"properties":{title_prop:{"title":[{"text":{"content":"ReachNote smoke test ✅"}}]}}}))
PY
)"

page_resp="$(api POST "https://api.notion.com/v1/pages" "${page_body}")"
page_code="$(tail -n1 <<<"${page_resp}")"
page_body_resp="$(sed '$d' <<<"${page_resp}")"

if [[ "${page_code}" == "200" ]]; then
  url="$(python3 -c 'import sys,json;print(json.load(sys.stdin).get("url",""))' <<<"${page_body_resp}")"
  echo "✅ 成功创建测试 page:"
  echo "   ${url}"
  echo
  echo "🎉 凭证就绪:token 有效、database 已 share、可写入。"
  echo "   → 请到 Notion 删除这条 'ReachNote smoke test' 测试 page。"
else
  echo "❌ 创建 page 失败(HTTP ${page_code})"
  python3 -c 'import sys,json;d=json.load(sys.stdin);print("   Notion:",d.get("code"),"-",d.get("message"))' <<<"${page_body_resp}" 2>/dev/null || echo "   ${page_body_resp}"
  echo
  echo "  validation_error → 多半是 API 版本 / data source 不匹配,或 title property 名异常"
  exit 1
fi
