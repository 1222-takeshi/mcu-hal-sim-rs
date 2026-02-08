#!/usr/bin/env bash
#
# CI Wait and Report Script
#
# このスクリプトは、GitHub Actionsの実行完了を待って結果を報告します。
# PRをプッシュした後に実行すると、CIの完了を自動で監視します。
#
# 使用方法:
#   ./scripts/ci-wait.sh [run-id]
#
# run-idを指定しない場合は、最新のワークフロー実行を監視します。
#
# 例:
#   # 最新のワークフローを監視
#   ./scripts/ci-wait.sh
#
#   # 特定のrun-idを監視
#   ./scripts/ci-wait.sh 21797882688

set -euo pipefail

# 色付き出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# gh コマンドの確認
if ! command -v gh &> /dev/null; then
    echo -e "${RED}Error: 'gh' command not found. Please install GitHub CLI.${NC}"
    echo "Visit: https://cli.github.com/"
    exit 1
fi

# run-idの取得
RUN_ID="${1:-}"

if [ -z "$RUN_ID" ]; then
    echo -e "${BLUE}Fetching latest workflow run...${NC}"
    RUN_ID=$(gh run list --workflow=ci.yml --limit 1 --json databaseId --jq '.[0].databaseId')

    if [ -z "$RUN_ID" ]; then
        echo -e "${RED}Error: No workflow runs found${NC}"
        exit 1
    fi

    echo -e "${GREEN}Monitoring run ID: $RUN_ID${NC}"
fi

# ワークフローの監視
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  CI Workflow Monitor${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Run ID: ${YELLOW}$RUN_ID${NC}\n"

# 初期状態の確認
STATUS=$(gh run view "$RUN_ID" --json status --jq '.status')
echo -e "Initial status: ${YELLOW}$STATUS${NC}"

# 完了まで待機
WAIT_COUNT=0
MAX_WAIT=600  # 最大10分（600秒）
INTERVAL=10   # 10秒ごとにチェック

while [ "$STATUS" != "completed" ]; do
    if [ $WAIT_COUNT -ge $MAX_WAIT ]; then
        echo -e "\n${RED}Timeout: Workflow did not complete within $((MAX_WAIT / 60)) minutes${NC}"
        exit 1
    fi

    sleep $INTERVAL
    WAIT_COUNT=$((WAIT_COUNT + INTERVAL))

    STATUS=$(gh run view "$RUN_ID" --json status --jq '.status')
    ELAPSED=$((WAIT_COUNT / 60))
    echo -e "Status: ${YELLOW}$STATUS${NC} (${ELAPSED}m elapsed)"
done

echo -e "\n${GREEN}✓ Workflow completed!${NC}\n"

# 結果の取得
CONCLUSION=$(gh run view "$RUN_ID" --json conclusion --jq '.conclusion')
JOBS_JSON=$(gh run view "$RUN_ID" --json jobs --jq '.jobs')

# ジョブごとの結果を表示
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Job Results${NC}"
echo -e "${BLUE}========================================${NC}\n"

echo "$JOBS_JSON" | jq -r '.[] | "\(.name):\(.conclusion)"' | while IFS=: read -r name conclusion; do
    if [ "$conclusion" = "success" ]; then
        echo -e "${GREEN}✓${NC} $name: ${GREEN}$conclusion${NC}"
    elif [ "$conclusion" = "failure" ]; then
        echo -e "${RED}✗${NC} $name: ${RED}$conclusion${NC}"
    else
        echo -e "${YELLOW}?${NC} $name: ${YELLOW}$conclusion${NC}"
    fi
done

# 失敗したジョブの詳細ログを表示
if [ "$CONCLUSION" = "failure" ]; then
    echo -e "\n${YELLOW}========================================${NC}"
    echo -e "${YELLOW}  Failed Job Logs${NC}"
    echo -e "${YELLOW}========================================${NC}\n"

    gh run view "$RUN_ID" --log-failed 2>&1 | tail -100

    echo -e "\n${RED}❌ CI failed. Please review the logs above.${NC}"
    echo -e "${BLUE}Full logs: ${NC}https://github.com/$(gh repo view --json nameWithOwner --jq '.nameWithOwner')/actions/runs/$RUN_ID"
    exit 1
else
    echo -e "\n${GREEN}========================================${NC}"
    echo -e "${GREEN}  🎉 All CI checks passed!${NC}"
    echo -e "${GREEN}========================================${NC}\n"
    exit 0
fi
