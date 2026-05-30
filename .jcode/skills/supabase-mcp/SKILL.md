---
name: supabase-mcp
description: Live database queries against the setterapp Supabase project (dev + prod) via Supabase CLI (qdev/qprod aliases) — fast 360° entity lookups, surgical SQL, narrow projections. Auto-load for "decime todo del lead/contact X", "qué config tiene el agente Y", "mostrame la conversación Z", "qué pasó con el cliente W", "datos del usuario V", "cuántos messages/conversations hay", "qué columnas tiene tabla T", "deployá edge function", "advisors", "corré migration", "tipos TS de DB". Does NOT apply to stack-trace / `console.log` debugging (→ `supabase-logs`), schema design / DDL theory (→ `database-designer`), or slow-query profiling methodology (→ `performance-profiler`; this skill executes the EXPLAIN, that one interprets it).
always_on: true
---

# Supabase CLI — speed-first playbook

Queries run via two bash aliases defined in `~/.zshrc`:

```bash
qdev  "SELECT …"   # setterapp-dev  (ref: vvjbpphjbnpdwjimnzsd)
qprod "SELECT …"   # setterapp prod (ref: afqbakvvfpebnxzjewsk)

# File variant:
qdev  -f scripts/my-query.sql
qprod -f scripts/my-query.sql
```

Both aliases call `~/Documents/Projects/setterapp/scripts/q <env> <sql>`, a thin
**psql** wrapper over the `~/.pg_service.conf` profiles (`setterapp-dev` / `setterapp-prod`);
passwords live in `~/.pgpass` (chmod 600) — no host/secret in the command. Direct psql over
the transaction pooler, NO MCP (prod-safe). Output is a table by default; add `-x` for vertical,
`-tA` for script-friendly. Project ops (migrations, edge fns, advisors, types) → `supabase` CLI.

Goal: every Supabase question lands on **one CLI call with the minimum payload** instead of `SELECT *` blobs, schema rediscovery, or 5-call chains.

## First reflex — pick the right skill BEFORE querying

| User says… | First action |
|---|---|
| "decime todo del lead/contact X", "qué hay con Y" | THIS skill → `Contact 360` recipe (1 query) |
| "qué config tiene el agente X" | THIS skill → `Agent 360` recipe (1 query) |
| "mostrame la conversación con Y", "qué pasó en el chat" | THIS skill → `Conversation 360` recipe (1 query) |
| "qué columnas tiene tabla T" | THIS skill → **schema preload section below**, no CLI call needed |
| "cuántos / count / aggregates" | THIS skill → recipe library |
| "deployá / migration" | `supabase functions deploy` / `supabase db push` CLI commands |
| "revisá los logs", "qué dicen los logs", "no anduvo" | STOP — `supabase-logs` skill |
| "diseñá una tabla nueva", "qué índice agregar", "RLS pattern" | STOP — `database-designer` skill |
| "esta query tarda mucho" | `performance-profiler` for methodology + THIS skill for `EXPLAIN ANALYZE` |

## Iron rule: project selection

| User said… | Alias |
|---|---|
| "DEV", "test agent", "vvjbpphjbnpdwjimnzsd", "el agente de prueba" | `qdev` |
| "PROD", "el agente real", "el cliente X", "afqbakvvfpebnxzjewsk", real lead names, real Instagram handles | `qprod` |
| Ambiguous | **ASK FIRST**, never guess |

Real client/lead data lives in PROD. Test fixtures in DEV. Confusing them = data leak risk.

---

## Hot-table schema (PRELOADED — no CLI discovery needed)

Source of truth for the 6 most-queried tables. Re-run schema discovery only if the user reports a column NOT listed here.

### `contacts` (35 cols — the canonical "lead/user")
```
id uuid PK · user_id uuid · team_id uuid → teams.id · integration_id uuid → integrations.id
platform text · external_id text · platform_page_id text
display_name text · username text · phone text · email text · profile_picture text
lead_status text · lead_score int · contact_source text (default 'unknown')
deal_value numeric · currency text (default 'USD') · deal_type text (default 'one_time')
monthly_value numeric · subscription_months int · acquisition_cost numeric
closed_at tstz · closure_reason text · last_message_at tstz
source_post_id text · source_post_timestamp tstz
follower_count int · is_verified bool · bio text · bio_fetched_at tstz
metadata jsonb (default '{}'::jsonb)
context text  -- long-term memory written by update_context tool
is_test bool (default false) · created_at tstz · updated_at tstz
```
**Lookup keys:** `id`, `username` (Instagram handle), `phone`, `email`, `(platform, external_id)` composite.

### `conversations` (43 cols)
```
id uuid PK · user_id uuid · team_id uuid → teams.id
contact_id uuid → contacts.id · agent_id uuid → agents.id · integration_id uuid → integrations.id
contact text · contact_alias text · contact_metadata jsonb
platform text · platform_conversation_id text · platform_page_id text
lead_status text · ai_enabled bool (default true) · current_phase text (default 'opener')
conversation_type text (default 'inbound') · lead_replied bool (default false)
unread_count int (default 0) · message_count int (default 0)
follow_up_count int (default 0) · followup_max int NOT NULL (default 3)
last_message_at tstz · last_inbound_at tstz · last_outbound_read_at tstz
processing_since tstz · first_response_time_seconds int · first_response_latency_ms int
conversation_duration_minutes int · lead_response_time_seconds int
engagement_score numeric · last_message_preview text
keyword_response_triggered bool · keyword_response_id text
conversation_summary text · summary_message_count int (default 0) · prior_context text
source_post_id uuid → instagram_posts.id · source_ig_media_id text
is_urgent bool · is_test bool · metadata jsonb (default '{}'::jsonb)
created_at tstz · updated_at tstz
```

### `messages` (14 cols — high row volume)
```
id uuid PK · conversation_id uuid → conversations.id · user_id uuid · team_id uuid → teams.id
platform_message_id text
content text NOT NULL · content_tsv tsvector
direction text NOT NULL  -- 'inbound' | 'outbound'
message_type text (default 'text') · sent_by text  -- 'ai' | 'human' | platform handle
status text (default 'sent') · metadata jsonb (default '{}'::jsonb)
created_at tstz · updated_at tstz
```

### `agents` (13 cols — heavy JSONB)
```
id uuid PK · user_id uuid · team_id uuid → teams.id · personality_id uuid → setter_personalities.id
name text · description text · platform text
config jsonb · custom_ai_context text · followup_prompt text
followup_banned_phrases text[] (default with 6 banned phrases)
created_at tstz · updated_at tstz
```

### `teams` (9 cols)
```
id uuid PK · name text · owner_id uuid
branding jsonb · ai_settings jsonb
auto_tag_enabled bool · auto_tag_button_mode bool
created_at tstz · updated_at tstz
```

### `team_members` (12 cols)
```
id uuid PK · team_id uuid → teams.id · user_id uuid · role text NOT NULL
display_name text · email text · avatar_url text
permissions jsonb (default '{}'::jsonb) · invited_by uuid
joined_at tstz · created_at tstz · updated_at tstz
```

### Foreign-key graph (compact)
```
teams ← team_members, agents, contacts, conversations, messages, tags, lead_status_history
contacts ← conversations.contact_id ← contact_tags.contact_id ← lead_status_history.contact_id
conversations ← messages.conversation_id ← lead_status_history.conversation_id
agents ← conversations.agent_id
agents.personality_id → setter_personalities.id
```

### JSONB key map — never `jsonb_pretty`, project specific keys

**`agents.config`** (top-level keys):
```
ownerName, companyName, assistantName, businessNiche, ticketSize, languageAccent
clientGoals, offerDetails, toneGuidelines, additionalContext, custom_ai_context
resources (array of {url, description, tag?})
dmKeywords (array)
qualification_v2 (object with threshold_score, criteria, etc)
followUpSchedule (object), enableFollowUp (bool), externalBotMessages (array)
meetingDuration, meetingBufferMinutes, meetingAvailableDays, meetingAvailableHoursStart/End
enableMeetingScheduling (bool)
typingDelayMinMs / MaxMs / MsPerWord
aiResponseDelayFirstMin / FirstMax / NormalMin / NormalMax
memory_enabled (bool), conversationExamples (array)
```

---

## Identifier resolver — turn anything into a `contact_id`

```sql
WITH resolved AS (
  SELECT id FROM contacts
  WHERE id::text = '<INPUT>'
     OR username = '<INPUT>'
     OR phone = '<INPUT>'
     OR email = '<INPUT>'
     OR (platform = 'instagram' AND external_id = '<INPUT>')
  LIMIT 1
)
SELECT id FROM resolved;
```

---

## Entity 360 recipes — single round-trip queries

### Contact 360

```sql
WITH resolved AS (
  SELECT id FROM contacts
  WHERE id::text = '<IDENTIFIER>'
     OR username = '<IDENTIFIER>'
     OR phone = '<IDENTIFIER>'
     OR email = '<IDENTIFIER>'
  LIMIT 1
)
SELECT jsonb_build_object(
  'contact', (
    SELECT to_jsonb(co) - 'metadata' || jsonb_build_object('metadata_keys', (
      SELECT array_agg(k) FROM jsonb_object_keys(co.metadata) k
    ))
    FROM contacts co WHERE co.id = (SELECT id FROM resolved)
  ),
  'team', (
    SELECT jsonb_build_object('id', t.id, 'name', t.name, 'owner_id', t.owner_id)
    FROM teams t WHERE t.id = (SELECT team_id FROM contacts WHERE id = (SELECT id FROM resolved))
  ),
  'conversations', (
    SELECT jsonb_agg(jsonb_build_object(
      'id', cv.id, 'platform', cv.platform, 'agent_id', cv.agent_id,
      'lead_status', cv.lead_status, 'ai_enabled', cv.ai_enabled,
      'current_phase', cv.current_phase, 'message_count', cv.message_count,
      'last_message_at', cv.last_message_at,
      'last_message_preview', LEFT(cv.last_message_preview, 120),
      'is_urgent', cv.is_urgent, 'follow_up_count', cv.follow_up_count
    ) ORDER BY cv.last_message_at DESC)
    FROM conversations cv WHERE cv.contact_id = (SELECT id FROM resolved)
  ),
  'recent_messages', (
    SELECT jsonb_agg(jsonb_build_object(
      'id', m.id, 'conversation_id', m.conversation_id,
      'direction', m.direction, 'sent_by', m.sent_by,
      'message_type', m.message_type, 'status', m.status,
      'content', LEFT(m.content, 300), 'created_at', m.created_at
    ) ORDER BY m.created_at DESC)
    FROM (
      SELECT m.* FROM messages m
      JOIN conversations cv ON cv.id = m.conversation_id
      WHERE cv.contact_id = (SELECT id FROM resolved)
      ORDER BY m.created_at DESC LIMIT 20
    ) m
  ),
  'tags', (
    SELECT jsonb_agg(jsonb_build_object('name', t.name, 'color', t.color))
    FROM contact_tags ct JOIN tags t ON t.id = ct.tag_id
    WHERE ct.contact_id = (SELECT id FROM resolved)
  ),
  'lead_status_history', (
    SELECT jsonb_agg(to_jsonb(h.*) ORDER BY h.created_at DESC)
    FROM (
      SELECT * FROM lead_status_history
      WHERE contact_id = (SELECT id FROM resolved)
      ORDER BY created_at DESC LIMIT 5
    ) h
  )
) AS contact_360;
```

### Agent 360

```sql
SELECT jsonb_build_object(
  'agent', jsonb_build_object(
    'id', a.id, 'name', a.name, 'team_id', a.team_id,
    'platform', a.platform, 'personality_id', a.personality_id,
    'created_at', a.created_at, 'updated_at', a.updated_at,
    'custom_ai_context_chars', LENGTH(a.custom_ai_context),
    'banned_phrases_count', COALESCE(array_length(a.followup_banned_phrases, 1), 0)
  ),
  'config_keys', jsonb_build_object(
    'accent', a.config->>'languageAccent',
    'ticket', a.config->>'ticketSize',
    'niche', a.config->>'businessNiche',
    'fu_on', (a.config->>'enableFollowUp')::bool,
    'fu_schedule', a.config->'followUpSchedule',
    'qual_threshold', a.config->'qualification_v2'->>'threshold_score',
    'memory_enabled', (a.config->>'memory_enabled')::bool,
    'meeting_enabled', (a.config->>'enableMeetingScheduling')::bool,
    'n_resources', jsonb_array_length(COALESCE(a.config->'resources', '[]'::jsonb)),
    'n_dm_keywords', jsonb_array_length(COALESCE(a.config->'dmKeywords', '[]'::jsonb))
  ),
  'recent_conversations', (
    SELECT jsonb_agg(jsonb_build_object(
      'id', cv.id, 'contact_id', cv.contact_id, 'lead_status', cv.lead_status,
      'message_count', cv.message_count, 'last_message_at', cv.last_message_at
    ) ORDER BY cv.last_message_at DESC)
    FROM (SELECT * FROM conversations WHERE agent_id = a.id ORDER BY last_message_at DESC NULLS LAST LIMIT 5) cv
  )
) AS agent_360
FROM agents a WHERE a.id = '<AGENT_UUID>';
```

### Conversation 360

```sql
SELECT jsonb_build_object(
  'conversation', jsonb_build_object(
    'id', cv.id, 'platform', cv.platform, 'lead_status', cv.lead_status,
    'ai_enabled', cv.ai_enabled, 'current_phase', cv.current_phase,
    'message_count', cv.message_count, 'follow_up_count', cv.follow_up_count,
    'followup_max', cv.followup_max, 'is_urgent', cv.is_urgent,
    'last_message_at', cv.last_message_at, 'processing_since', cv.processing_since,
    'metadata', cv.metadata, 'summary', LEFT(cv.conversation_summary, 500)
  ),
  'contact', (SELECT jsonb_build_object(
    'id', co.id, 'display_name', co.display_name, 'username', co.username,
    'phone', co.phone, 'email', co.email, 'lead_status', co.lead_status, 'lead_score', co.lead_score
  ) FROM contacts co WHERE co.id = cv.contact_id),
  'agent', (SELECT jsonb_build_object('id', a.id, 'name', a.name) FROM agents a WHERE a.id = cv.agent_id),
  'messages', (
    SELECT jsonb_agg(jsonb_build_object(
      'id', m.id, 'direction', m.direction, 'sent_by', m.sent_by, 'status', m.status,
      'message_type', m.message_type, 'content', m.content, 'created_at', m.created_at
    ) ORDER BY m.created_at ASC)
    FROM messages m WHERE m.conversation_id = cv.id
  )
) AS conversation_360
FROM conversations cv WHERE cv.id = '<CONV_UUID>';
```

---

## Recipe library — atomic queries (paste-ready)

### Agent inspection (slim)
```sql
SELECT id, name, team_id, personality_id, updated_at
FROM agents WHERE name ILIKE '%estefan%'
ORDER BY updated_at DESC LIMIT 5;
```

### Agent config keys (no blobs)
```sql
SELECT
  name,
  config->>'languageAccent' AS accent,
  config->>'ticketSize'     AS ticket,
  config->>'businessNiche'  AS niche,
  (config->>'enableFollowUp')::bool                              AS fu_on,
  config->'qualification_v2'->>'threshold_score'                 AS qual_thresh,
  jsonb_array_length(COALESCE(config->'resources','[]'::jsonb))  AS n_resources,
  COALESCE(array_length(followup_banned_phrases, 1), 0)          AS n_banned
FROM agents WHERE id='…';
```

### Conversation lookup by lead handle
```sql
SELECT c.id, c.contact_id, c.last_message_at, c.current_phase,
       co.display_name, co.username, co.phone
FROM conversations c
JOIN contacts co ON co.id = c.contact_id
WHERE co.username = '<handle>'
   OR co.phone = '<phone>'
   OR co.email = '<email>'
ORDER BY c.last_message_at DESC LIMIT 5;
```

### Last N messages of a conversation (slim)
```sql
SELECT id, direction, sent_by, status, created_at,
       LEFT(content, 200) AS content_excerpt, message_type
FROM messages
WHERE conversation_id = '<conv-uuid>'
ORDER BY created_at DESC LIMIT 20;
```

### Schema discovery
```sql
SELECT column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_schema='public' AND table_name='<tbl>'
ORDER BY ordinal_position;
```

### Counts / aggregates
```sql
SELECT lead_status, COUNT(*) AS n
FROM contacts WHERE team_id='…'
GROUP BY lead_status ORDER BY n DESC;
```

### Function logs
```sql
SELECT created_at, function_name, status, LEFT(message, 200) AS msg, meta
FROM function_logs
WHERE conversation_id = '<conv-uuid>'
ORDER BY created_at DESC LIMIT 30;
```

---

## Surgical SQL — the three rules

### 1. Never `SELECT *` on rows with JSONB/long-text columns
Heavy columns: `agents.config`, `agents.custom_ai_context`, `conversations.metadata`, `messages.content`, `contacts.context`.

### 2. JSONB → use `->`/`->>` operators, not `jsonb_pretty(config)` blobs
```sql
-- BAD
SELECT jsonb_pretty(config) FROM agents WHERE id='��';

-- GOOD
SELECT config->>'languageAccent' AS accent, config->>'ticketSize' AS ticket
FROM agents WHERE id='…';
```

### 3. ALWAYS bound rows with `LIMIT` when exploring
Default `LIMIT 5` for exploration. The user can ask for more.

---

## Hard rules

1. **Never `SELECT *`** on `agents`, `conversations`, `messages`, `contacts` — heavy columns. Project explicitly.
2. **Never `jsonb_pretty(config)`** unless the user explicitly asks for the full prompt context.
3. **Never query PROD** for a question clearly about DEV, and vice versa. Ask if ambiguous.
4. **Never run DDL in PROD** without user confirmation.
5. **Default `LIMIT 5`** when exploring; `LIMIT 30` when paginating known rows.
6. **For "everything about X" questions** — reach for the matching 360 recipe FIRST, not 5 separate queries.
7. **Schema preload section is canonical** — only re-run schema discovery if a user references a column not listed there.
