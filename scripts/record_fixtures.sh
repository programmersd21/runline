#!/bin/sh
set -e
repo="${1:-owner/repo}"
gh run list -R "$repo" --limit 30 --json attempt,conclusion,createdAt,databaseId,displayTitle,event,headBranch,headSha,name,number,startedAt,status,updatedAt,url,workflowName > tests/fixtures/runs.json
echo "recorded fixtures. sanitize tokens before committing."
