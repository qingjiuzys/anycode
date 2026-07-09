# Scenario evaluation (cloud models)

Record end-to-end runs after linking an anycode.work account and selecting **Cloud Auto** or a named cloud model.

| Scenario | Input prompt | Expected artifacts | Pass criteria |
|----------|--------------|-------------------|---------------|
| Coding | Home → **编码** chip | Source files under project root | Build or test command succeeds |
| PPT | Home → **写 PPT** + `office-pptx` skill | `.pptx` under workspace | File opens; ≥8 slides |
| Video | Home → **做视频** + `video-script` | `video/script.md` + assets | Script table + ≥1 asset path |
| Novel | Home → **写小说** + `novel-writer` | `chapters/01.md` | Chapter file on disk |

## Template row

| Field | Value |
|-------|-------|
| Date | YYYY-MM-DD |
| Model | cloud-auto / cloud-agnes-chat |
| Duration | |
| Tokens (usage summary) | |
| Artifact path | |
| Notes | |

## Local verification

```bash
./scripts/dev-account-portal.sh api    # :43200
./scripts/dev-account-portal.sh portal # :43201
export ANYCODE_ACCOUNT_API_URL=http://127.0.0.1:43200
export ANYCODE_ACCOUNT_PORTAL_URL=http://127.0.0.1:43201
# Desktop → link account → Settings models show 云端 → run scenarios above
```

## Production verification

1. Register at https://anycode.work/register  
2. Link desktop via **Account → Link cloud**  
3. Confirm `usage_events` increments in account-service after a chat turn  
