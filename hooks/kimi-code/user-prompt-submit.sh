#!/bin/sh
# kimi-code user-prompt hook.
# 1. Forwards the event JSON to the ai-memory server (fire-and-forget).
# 2. Synchronously fetches any pending cross-agent handoff and prints
#    it to stdout — kimi-code injects UserPromptSubmit hook stdout as a
#    user message (origin hook_result) before the turn, so the agent
#    sees prior context with no human in the loop. SessionStart cannot
#    deliver it: kimi-code discards SessionStart hook stdout (v0.28.1,
#    packages/agent-core/src/session/index.ts). Empty stdout injects
#    nothing, so print only when a handoff exists.
# 3. Delivers the compiled project brief ([briefing]
#    inject_on_session_start) on the FIRST prompt of the session only —
#    parity with Claude's once-per-SessionStart brief. The handoff fetch
#    keeps happening on every prompt (cheap, self-limiting: empty body
#    when nothing is pending), but without the briefing params, so the
#    server does not recompose the brief per prompt. The marker survives
#    /clear, so re-briefing after a context clear is not supported in v1.
_lib_dir="$(dirname "$0")"
[ -f "$_lib_dir/_lib.sh" ] || _lib_dir="$_lib_dir/.."
. "$_lib_dir/_lib.sh"

SERVER="${AI_MEMORY_HOOK_URL:-http://127.0.0.1:49374}"
PAYLOAD=$(cat)
CWD=$(ai_memory_extract_cwd "$PAYLOAD")
QS=$(ai_memory_marker_qs "$CWD")
SESSION_ID=$(ai_memory_extract_session_id "$PAYLOAD")
SESSION_QS=""
[ -n "$SESSION_ID" ] && SESSION_QS="&session_id=$(ai_memory_url_encode "$SESSION_ID")"

# Once-per-session briefing gate, keyed by the native session id (kimi
# always sends `sessionId`); without one, a stable hash of agent+cwd so a
# session-less payload still briefs only once per checkout.
BRIEF_KEY="$SESSION_ID"
if [ -z "$BRIEF_KEY" ]; then
    BRIEF_KEY="kimi-code-$(printf '%s' "kimi-code:$CWD" | cksum | awk '{print $1}')"
fi
BRIEF_FILE=$(ai_memory_briefed_file "$BRIEF_KEY")
BRIEF_QS=""
[ -f "$BRIEF_FILE" ] || BRIEF_QS=$(ai_memory_briefing_qs "$CWD")

printf '%s' "$PAYLOAD" \
    | ai_memory_post_hook "$SERVER/hook?event=user-prompt&agent=kimi-code${QS}" >/dev/null 2>&1 || true

HANDOFF=$(ai_memory_get_handoff "$SERVER/handoff?agent=kimi-code${QS}${SESSION_QS}${BRIEF_QS}" 2>/dev/null || true)
# Mark the session as briefed only AFTER the GET completed — success or
# error. Fail-open on purpose: with the server down, re-sending the
# brief-flagged request on every prompt would deliver nothing anyway, and
# the one lost brief returns on the next session.
mkdir -p "$(dirname "$BRIEF_FILE")" 2>/dev/null || true
: > "$BRIEF_FILE" 2>/dev/null || true
[ -n "$HANDOFF" ] && printf '%s\n' "$HANDOFF"
exit 0
